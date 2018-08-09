use kay::{ActorSystem, World, Actor, Fate};
use compact::{CVec, COption};
use descartes::P2;

use transport::lane::{Lane, LaneID};
use simulation::Ticks;
use construction::{ConstructionID, Constructable, ConstructableID};
use planning::{Prototype, PrototypeKind};

pub mod rendering;
pub mod architecture;

use economy::households::HouseholdID;
use transport::pathfinding::PreciseLocation;
use economy::immigration_and_development::ImmigrationManagerID;
use land_use::zone_planning::Lot;

#[derive(Copy, Clone)]
pub struct Unit(Option<HouseholdID>, UnitType);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum UnitType {
    Dwelling,
    Retail,
    Agriculture,
    Mill,
    Bakery,
    NeighboringTownTrade,
}

#[derive(Copy, Clone)]
pub struct UnitIdx(usize);

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum BuildingStyle {
    FamilyHouse,
    GroceryShop,
    Field,
    Mill,
    Bakery,
    NeighboringTownConnection,
}

#[derive(Compact, Clone)]
pub struct Building {
    id: BuildingID,
    units: CVec<Unit>,
    lot: Lot,
    pub location: Option<PreciseLocation>,
    style: BuildingStyle,
    being_destroyed_for: COption<ConstructionID>,
    started_reconnect: bool,
}

//use stagemaster::geometry::add_debug_line;

impl Building {
    pub fn spawn(id: BuildingID, style: BuildingStyle, lot: &Lot, world: &mut World) -> Building {
        println!("Spawned building {:?}", style);

        rendering::on_add(id, lot, style, world);

        Simulation::local_first(world).wake_up_in(
            Ticks::from(Duration::from_minutes(10)),
            id.into(),
            world,
        );

        Building {
            id,
            units: units_for_style(style),
            lot: lot.clone(),
            location: None,
            style,
            being_destroyed_for: COption(None),
            started_reconnect: false,
        }
    }

    pub fn try_offer_unit(
        &mut self,
        required_unit_type: UnitType,
        requester: ImmigrationManagerID,
        world: &mut World,
    ) {
        println!(
            "{:?} got offer request for {:?}",
            self.style, required_unit_type
        );
        if self.being_destroyed_for.is_none() {
            if let Some(idx) = self.units.iter().position(|&Unit(household, unit_type)| {
                household.is_none() && unit_type == required_unit_type
            }) {
                requester.on_unit_offer(self.id, UnitIdx(idx), world);
                println!("...and responded positively!!!!");
            } else {
                println!("...but doesn't have the unit type");
            }
        } else {
            println!("...but is being destroyed");
        }
    }

    pub fn add_household(&mut self, household: HouseholdID, unit: UnitIdx, _: &mut World) {
        self.units[unit.0].0 = Some(household);
    }

    pub fn remove_household(&mut self, household: HouseholdID, world: &mut World) {
        let position = self
            .units
            .iter()
            .position(|&Unit(user, _)| user == Some(household))
            .expect("Tried to remove a household not in the building");
        self.units[position].0 = None;

        if self.being_destroyed_for.is_some() && self.all_households().is_empty() {
            self.id.finally_destroy(world);
        }
    }

    pub fn all_households(&self) -> Vec<HouseholdID> {
        self.units
            .iter()
            .filter_map(|&Unit(user, _)| user)
            .collect()
    }

    pub fn finally_destroy(&mut self, world: &mut World) -> Fate {
        rendering::on_destroy(self.id, world);
        if let Some(location) = self.location {
            location.node.remove_attachee(self.id_as(), world);
        }
        self.being_destroyed_for
            .unwrap()
            .action_done(self.id.into(), world);
        Fate::Die
    }
}

impl Constructable for Building {
    fn morph(&mut self, new_prototype: &Prototype, report_to: ConstructionID, world: &mut World) {
        if let PrototypeKind::Lot(ref lot_prototype) = new_prototype.kind {
            self.lot = lot_prototype.lot.clone();
            rendering::on_destroy(self.id, world);
            rendering::on_add(self.id, &self.lot, self.style, world);
            report_to.action_done(self.id.into(), world);
        } else {
            unreachable!()
        }
    }

    fn destruct(&mut self, report_to: ConstructionID, world: &mut World) -> Fate {
        self.being_destroyed_for = COption(Some(report_to));

        if self.all_households().is_empty() {
            self.finally_destroy(world)
        } else {
            for household in &self.all_households() {
                household.destroy(world);
            }

            Fate::Live
        }
    }
}

use transport::pathfinding::{Location, Attachee, AttacheeID};
use simulation::{Simulation, Sleeper, SleeperID, Duration};

impl Attachee for Building {
    fn location_changed(
        &mut self,
        _old: Option<Location>,
        maybe_new: Option<Location>,
        world: &mut World,
    ) {
        if let Some(new) = maybe_new {
            self.location
                .as_mut()
                .expect("Only an existing location can change")
                .location = new;
        } else {
            self.location = None;
            Simulation::local_first(world).wake_up_in(
                Ticks::from(Duration::from_minutes(10)),
                self.id_as(),
                world,
            );
        }
    }
}

impl Sleeper for Building {
    fn wake(&mut self, _instant: Instant, world: &mut World) {
        if self.started_reconnect {
            if self.location.is_none() {
                // TODO: do we still need to destroy here?
            } else {
                self.started_reconnect = false;
            }
        } else {
            Lane::global_broadcast(world).try_reconnect_building(
                self.id,
                self.lot.center_point(),
                world,
            );
            Simulation::local_first(world).wake_up_in(
                Ticks::from(Duration::from_minutes(10)),
                self.id_as(),
                world,
            );
            self.started_reconnect = true;
        }
    }
}

impl Building {
    pub fn reconnect(
        &mut self,
        new_location: PreciseLocation,
        _new_connection_point: P2,
        world: &mut World,
    ) {
        if self.location.is_none() {
            self.location = Some(new_location);
            new_location.node.add_attachee(self.id_as(), world);
        }
    }
}

use transport::pathfinding::{RoughLocation, RoughLocationID, RoughLocationResolve};
use simulation::Instant;

impl RoughLocation for Building {
    fn resolve(&self) -> RoughLocationResolve {
        RoughLocationResolve::Done(self.location, self.lot.center_point())
    }
}

const FAMILIES_PER_NEIGHBORING_TOWN: usize = 5;

pub fn units_for_style(style: BuildingStyle) -> CVec<Unit> {
    match style {
        BuildingStyle::FamilyHouse => vec![Unit(None, UnitType::Dwelling)],
        BuildingStyle::GroceryShop => vec![Unit(None, UnitType::Retail)],
        BuildingStyle::Bakery => vec![Unit(None, UnitType::Bakery)],
        BuildingStyle::Mill => vec![Unit(None, UnitType::Mill)],
        BuildingStyle::Field => vec![Unit(None, UnitType::Agriculture)],
        BuildingStyle::NeighboringTownConnection => {
            Some(Unit(None, UnitType::NeighboringTownTrade))
                .into_iter()
                .chain(vec![
                    Unit(None, UnitType::Dwelling);
                    FAMILIES_PER_NEIGHBORING_TOWN
                ])
                .collect()
        }
    }.into()
}

pub const MIN_ROAD_LENGTH_TO_TOWN: f32 = 4000.0;
const MIN_NEIGHBORING_TOWN_DISTANCE: f32 = 2000.0;

pub const MIN_LANE_BUILDING_DISTANCE: f32 = 15.0;

#[derive(Compact, Clone, Default)]
pub struct BuildingPlanResultDelta {
    buildings_to_destroy: CVec<BuildingID>,
}

#[derive(Compact, Clone, Default)]
pub struct MaterializedBuildings {
    buildings: CVec<(P2, BuildingID, LaneID)>,
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Building>();
    kay_auto::auto_setup(system);
    rendering::auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
