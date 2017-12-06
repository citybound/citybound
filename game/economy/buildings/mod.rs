use kay::{ActorSystem, World, External, TypedID, Actor};
use compact::CVec;
use descartes::{P2, V2, Norm, Curve};
use stagemaster::combo::{Bindings, Combo2};
use stagemaster::{UserInterfaceID, Event3d, Interactable3d, Interactable3dID};
use stagemaster::combo::Button::*;
use stagemaster::geometry::AnyShape;
use transport::lane::{Lane, LaneID};
use planning::materialized_reality::{MaterializedReality, MaterializedRealityID};


use super::households::family::FamilyID;
use super::households::grocery_shop::GroceryShopID;
use super::households::cow_farm::CowFarmID;
use super::households::grain_farm::GrainFarmID;
use super::households::vegetable_farm::VegetableFarmID;
use super::households::mill::MillID;
use super::households::bakery::BakeryID;
use super::households::neighboring_town_trade::NeighboringTownTradeID;
use core::simulation::Ticks;
use core::random::{seed, Rng};

pub mod rendering;
pub mod architecture;
use self::architecture::BuildingStyle;

use super::households::HouseholdID;
use transport::pathfinding::PreciseLocation;

#[derive(Copy, Clone)]
pub struct Unit(Option<HouseholdID>, UnitType);

#[derive(Copy, Clone)]
pub enum UnitType {
    Dwelling,
    Retail,
    Agriculture,
    Mill,
    Bakery,
}

#[derive(Copy, Clone)]
pub struct UnitIdx(usize);

#[derive(Compact, Clone)]
pub struct Building {
    id: BuildingID,
    units: CVec<Unit>,
    lot: Lot,
    style: BuildingStyle,
    being_destroyed: bool,
    started_reconnect: bool,
}

//use stagemaster::geometry::add_debug_line;

impl Building {
    pub fn spawn(
        id: BuildingID,
        materialized_reality: MaterializedRealityID,
        units: &CVec<Unit>,
        style: BuildingStyle,
        lot: &Lot,
        world: &mut World,
    ) -> Building {
        // add_debug_line(
        //     lot.position,
        //     lot.adjacent_lane_position,
        //     [0.5, 0.5, 0.5],
        //     0.0,
        //     world,
        // );
        // TODO: ugly: untyped RawID shenanigans
        let adjacent_lane = unsafe {
            LaneID::from_raw(
                lot.location
                    .expect("Lot should already have location")
                    .node
                    .as_raw(),
            )
        };
        materialized_reality.on_building_built(lot.position, id, adjacent_lane, world);

        rendering::on_add(id, lot, style, world);
        lot.location
            .expect("Lot should already have location")
            .node
            .add_attachee(id.into(), world);

        Building {
            id,
            units: units.clone(),
            lot: lot.clone(),
            style,
            being_destroyed: false,
            started_reconnect: false,
        }
    }

    pub fn add_household(&mut self, household: HouseholdID, unit: UnitIdx, _: &mut World) {
        self.units[unit.0].0 = Some(household);
    }

    pub fn remove_household(&mut self, household: HouseholdID, world: &mut World) {
        let position = self.units
            .iter()
            .position(|&Unit(user, _)| user == Some(household))
            .expect("Tried to remove a household not in the building");
        self.units[position].0 = None;

        if self.being_destroyed && self.all_households().is_empty() {
            self.id.finally_destroy(world);
        }
    }

    pub fn destroy(&mut self, world: &mut World) {
        for household in &self.all_households() {
            household.destroy(world);
        }

        self.being_destroyed = true;
    }

    pub fn all_households(&self) -> Vec<HouseholdID> {
        self.units
            .iter()
            .filter_map(|&Unit(user, _)| user)
            .collect()
    }

    pub fn finally_destroy(&mut self, world: &mut World) -> ::kay::Fate {
        rendering::on_destroy(self.id, world);
        if let Some(location) = self.lot.location {
            location.node.remove_attachee(self.id_as(), world);
        }
        ::kay::Fate::Die
    }
}

use transport::pathfinding::{Location, Attachee, AttacheeID};
use core::simulation::{Simulation, SimulationID, Sleeper, SleeperID, Duration};

impl Attachee for Building {
    fn location_changed(
        &mut self,
        _old: Option<Location>,
        maybe_new: Option<Location>,
        world: &mut World,
    ) {
        if let Some(new) = maybe_new {
            self.lot
                .location
                .as_mut()
                .expect("Only an existing location can change")
                .location = new;
        } else {
            self.lot.location = None;
            Simulation::local_first(world).wake_up_in(
                Ticks::from(
                    Duration::from_minutes(10),
                ),
                self.id_as(),
                world,
            );
        }
    }
}

impl Sleeper for Building {
    fn wake(&mut self, _instant: Instant, world: &mut World) {
        if self.started_reconnect {
            if self.lot.location.is_none() {
                self.destroy(world);
            } else {
                self.started_reconnect = false;
            }
        } else {
            Lane::global_broadcast(world).try_reconnect_building(self.id, self.lot.position, world);
            Simulation::local_first(world).wake_up_in(
                Ticks::from(
                    Duration::from_minutes(10),
                ),
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
        new_adjacent_lane_position: P2,
        world: &mut World,
    ) {
        if self.lot.location.is_none() {
            self.lot.location = Some(new_location);
            self.lot.adjacent_lane_position = new_adjacent_lane_position;
            new_location.node.add_attachee(self.id_as(), world);
        }
    }
}

use transport::pathfinding::{RoughLocation, RoughLocationID, RoughLocationResolve};
use core::simulation::Instant;

impl RoughLocation for Building {
    fn resolve(&self) -> RoughLocationResolve {
        RoughLocationResolve::Done(self.lot.location, self.lot.position)
    }
}

#[derive(Compact, Clone)]
pub struct Lot {
    pub position: P2,
    pub orientation: V2,
    pub location: Option<PreciseLocation>,
    pub adjacent_lane_position: P2,
}

#[derive(Serialize, Deserialize)]
pub struct BuildingSpawnerBindings(Bindings);

impl Default for BuildingSpawnerBindings {
    fn default() -> Self {
        BuildingSpawnerBindings(Bindings::new(
            vec![("Spawn Building", Combo2::new(&[B], &[]))],
        ))
    }
}

#[derive(Compact, Clone)]
pub enum BuildingSpawnerState {
    Idle,
    Collecting(CVec<Lot>),
    CheckingBuildings(CVec<Lot>, CVec<bool>),
    CheckingLanes(CVec<Lot>, CVec<bool>),
}

#[derive(Compact, Clone)]
pub struct BuildingSpawner {
    id: BuildingSpawnerID,
    simulation: SimulationID,
    materialized_reality: MaterializedRealityID,
    bindings: External<BuildingSpawnerBindings>,
    state: BuildingSpawnerState,
}

impl BuildingSpawner {
    pub fn init(
        id: BuildingSpawnerID,
        user_interface: UserInterfaceID,
        simulation: SimulationID,
        materialized_reality: MaterializedRealityID,
        world: &mut World,
    ) -> BuildingSpawner {
        user_interface.add(id.into(), AnyShape::Everywhere, 0, world);
        user_interface.focus(id.into(), world);

        BuildingSpawner {
            id,
            simulation,
            materialized_reality,
            bindings: External::new(::ENV.load_settings("Building Spawning")),
            state: BuildingSpawnerState::Idle,
        }
    }

    pub fn found_lot(&mut self, lot: &Lot, _: &mut World) {
        if let BuildingSpawnerState::Collecting(ref mut lots) = self.state {
            lots.push(lot.clone())
        } else {
            println!("Unexpected found lot");
        }
    }

    fn spawn_building(
        materialized_reality: MaterializedRealityID,
        lot: &Lot,
        simulation: SimulationID,
        world: &mut World,
    ) {
        if lot.location
            .map(|PreciseLocation { offset, .. }| offset)
            .unwrap_or(0.0) > MIN_ROAD_LENGTH_TO_TOWN
        {
            const FAMILIES_PER_NEIGHBORING_TOWN: usize = 5;
            let building_id =
                BuildingID::spawn(
                    materialized_reality,
                    vec![Unit(None, UnitType::Dwelling); 1 + FAMILIES_PER_NEIGHBORING_TOWN].into(),
                    BuildingStyle::NeihboringTownConnection,
                    lot.clone(),
                    world,
                );
            let trade_id = NeighboringTownTradeID::move_into(building_id, simulation, world);
            building_id.add_household(trade_id.into(), UnitIdx(0), world);
            for i in 0..FAMILIES_PER_NEIGHBORING_TOWN {
                let family_id = FamilyID::move_into(3, building_id, simulation, world);
                building_id.add_household(family_id.into(), UnitIdx(i + 1), world);
            }
        } else {
            let family_share = 1.0;
            let grocery_share = 0.02;
            let cow_farm_share = 0.09;
            let veg_farm_share = 0.026;
            let grain_farm_share = 0.02; //0.0016;
            let mill_share = 0.02; //0.001;
            let bakery_share = 0.02; //0.01;

            let total_share = family_share + grocery_share + cow_farm_share + veg_farm_share +
                grain_farm_share + mill_share +
                bakery_share;

            let dot = seed((lot.position.x as u32, lot.position.y as u32))
                .gen_range(0.0, total_share);

            if dot < family_share {
                let building_id = BuildingID::spawn(
                    materialized_reality,
                    vec![Unit(None, UnitType::Dwelling)].into(),
                    BuildingStyle::FamilyHouse,
                    lot.clone(),
                    world,
                );
                let family_id = FamilyID::move_into(3, building_id, simulation, world);
                building_id.add_household(family_id.into(), UnitIdx(0), world);
            } else if dot < family_share + grocery_share {
                let building_id = BuildingID::spawn(
                    materialized_reality,
                    vec![Unit(None, UnitType::Retail)].into(),
                    BuildingStyle::GroceryShop,
                    lot.clone(),
                    world,
                );
                let shop_id = GroceryShopID::move_into(building_id, simulation, world);
                building_id.add_household(shop_id.into(), UnitIdx(0), world);
            } else if dot < family_share + grocery_share + cow_farm_share {
                let building_id = BuildingID::spawn(
                    materialized_reality,
                    vec![Unit(None, UnitType::Agriculture)].into(),
                    BuildingStyle::Field,
                    lot.clone(),
                    world,
                );
                let farm_id = CowFarmID::move_into(building_id, simulation, world);
                building_id.add_household(farm_id.into(), UnitIdx(0), world);
            } else if dot < family_share + grocery_share + cow_farm_share + veg_farm_share {
                let building_id = BuildingID::spawn(
                    materialized_reality,
                    vec![Unit(None, UnitType::Agriculture)].into(),
                    BuildingStyle::Field,
                    lot.clone(),
                    world,
                );
                let farm_id = VegetableFarmID::move_into(building_id, simulation, world);
                building_id.add_household(farm_id.into(), UnitIdx(0), world);
            } else if dot <
                       family_share + grocery_share + cow_farm_share + veg_farm_share +
                           grain_farm_share
            {
                let building_id = BuildingID::spawn(
                    materialized_reality,
                    vec![Unit(None, UnitType::Agriculture)].into(),
                    BuildingStyle::Field,
                    lot.clone(),
                    world,
                );
                let farm_id = GrainFarmID::move_into(building_id, simulation, world);
                building_id.add_household(farm_id.into(), UnitIdx(0), world);
            } else if dot <
                       family_share + grocery_share + cow_farm_share + veg_farm_share +
                           grain_farm_share + mill_share
            {
                let building_id = BuildingID::spawn(
                    materialized_reality,
                    vec![Unit(None, UnitType::Mill)].into(),
                    BuildingStyle::Mill,
                    lot.clone(),
                    world,
                );
                let mill_id = MillID::move_into(building_id, simulation, world);
                building_id.add_household(mill_id.into(), UnitIdx(0), world);
            } else {
                let building_id = BuildingID::spawn(
                    materialized_reality,
                    vec![Unit(None, UnitType::Bakery)].into(),
                    BuildingStyle::Bakery,
                    lot.clone(),
                    world,
                );
                let bakery_id = BakeryID::move_into(building_id, simulation, world);
                building_id.add_household(bakery_id.into(), UnitIdx(0), world);
            }
        }
    }

    pub fn update_feasibility(&mut self, new_feasibility: &CVec<bool>, _: &mut World) {
        match self.state {
            BuildingSpawnerState::CheckingBuildings(_, ref mut feasibility) |
            BuildingSpawnerState::CheckingLanes(_, ref mut feasibility) => {
                for (old, new) in feasibility.iter_mut().zip(new_feasibility) {
                    *old = *old && *new;
                }
            }
            _ => println!("Unexpected feasibility"),
        }
    }
}

impl Interactable3d for BuildingSpawner {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        if let Event3d::Combos(combos) = event {
            self.bindings.0.do_rebinding(&combos.current);

            if self.bindings.0["Spawn Building"].is_freshly_in(&combos) {
                if let BuildingSpawnerState::Idle = self.state {
                    Lane::global_broadcast(world).find_lot(self.id, world);
                    self.simulation.wake_up_in(Ticks(10), self.id_as(), world);
                    self.state = BuildingSpawnerState::Collecting(CVec::new());
                }
            }
        };
    }
}

use core::simulation::{Simulatable, SimulatableID};

impl Simulatable for BuildingSpawner {
    fn tick(&mut self, _dt: f32, current_instant: Instant, world: &mut World) {
        if current_instant.ticks() % 1000 == 0 {
            if let BuildingSpawnerState::Idle = self.state {
                Lane::global_broadcast(world).find_lot(self.id, world);
                self.simulation.wake_up_in(Ticks(10), self.id_as(), world);
                self.state = BuildingSpawnerState::Collecting(CVec::new());
            }
        }
    }
}

const MIN_BUILDING_DISTANCE: f32 = 20.0;
pub const MIN_ROAD_LENGTH_TO_TOWN: f32 = 4000.0;
const MIN_NEIGHBORING_TOWN_DISTANCE: f32 = 2000.0;

pub trait LotConflictor {
    fn find_conflicts(&mut self, lots: &CVec<Lot>, requester: BuildingSpawnerID, world: &mut World);
}

impl LotConflictor for Building {
    fn find_conflicts(
        &mut self,
        lots: &CVec<Lot>,
        requester: BuildingSpawnerID,
        world: &mut World,
    ) {
        requester.update_feasibility(
            lots.iter()
                .map(|lot| {
                    let min_distance = if lot.location
                        .map(|PreciseLocation { offset, .. }| offset)
                        .unwrap_or(0.0) >
                        MIN_ROAD_LENGTH_TO_TOWN &&
                        self.lot
                            .location
                            .map(|PreciseLocation { offset, .. }| offset)
                            .unwrap_or(0.0) > MIN_ROAD_LENGTH_TO_TOWN
                    {
                        MIN_NEIGHBORING_TOWN_DISTANCE
                    } else {
                        MIN_BUILDING_DISTANCE
                    };
                    (lot.position - self.lot.position).norm() > min_distance
                })
                .collect(),
            world,
        )
    }
}

pub const MIN_LANE_BUILDING_DISTANCE: f32 = 15.0;

impl LotConflictor for Lane {
    fn find_conflicts(
        &mut self,
        lots: &CVec<Lot>,
        requester: BuildingSpawnerID,
        world: &mut World,
    ) {
        requester.update_feasibility(
            lots.iter()
                .map(|lot| {
                    self.construction.path.distance_to(lot.position) > MIN_LANE_BUILDING_DISTANCE
                })
                .collect(),
            world,
        )
    }
}

impl Sleeper for BuildingSpawner {
    fn wake(&mut self, _time: Instant, world: &mut World) {
        self.state = match self.state {
            BuildingSpawnerState::Collecting(ref mut lots) => {
                let buildings: LotConflictorID = Building::global_broadcast(world).into();
                let mut nonconflicting_lots = CVec::<Lot>::new();
                for lot in lots.iter() {
                    let far_from_all = nonconflicting_lots.iter().all(|other_lot| {
                        let min_distance = if lot.location
                            .map(|PreciseLocation { offset, .. }| offset)
                            .unwrap_or(0.0) >
                            MIN_ROAD_LENGTH_TO_TOWN &&
                            other_lot
                                .location
                                .map(|PreciseLocation { offset, .. }| offset)
                                .unwrap_or(0.0) >
                                MIN_ROAD_LENGTH_TO_TOWN
                        {
                            MIN_NEIGHBORING_TOWN_DISTANCE
                        } else {
                            MIN_BUILDING_DISTANCE
                        };
                        (lot.position - other_lot.position).norm() > min_distance
                    });
                    if far_from_all {
                        nonconflicting_lots.push(lot.clone());
                    }
                }
                buildings.find_conflicts(nonconflicting_lots.clone(), self.id, world);
                self.simulation.wake_up_in(Ticks(10), self.id.into(), world);

                let nonconclicting_lots_len = nonconflicting_lots.len();
                BuildingSpawnerState::CheckingBuildings(
                    nonconflicting_lots,
                    vec![true; nonconclicting_lots_len].into(),
                )
            }
            BuildingSpawnerState::CheckingBuildings(ref mut lots, ref mut feasible) => {
                let new_lots: CVec<_> = lots.iter()
                    .zip(feasible)
                    .filter_map(|(lot, feasible)| if *feasible {
                        Some(lot.clone())
                    } else {
                        None
                    })
                    .collect();
                let lanes = unsafe { LotConflictorID::from_raw(world.global_broadcast::<Lane>()) };
                lanes.find_conflicts(new_lots.clone(), self.id, world);
                self.simulation.wake_up_in(Ticks(10), self.id.into(), world);

                let new_lots_len = new_lots.len();
                BuildingSpawnerState::CheckingLanes(new_lots, vec![true; new_lots_len].into())
            }
            BuildingSpawnerState::CheckingLanes(ref mut lots, ref mut feasible) => {
                for (lot, feasible) in lots.iter().zip(feasible) {
                    if *feasible &&
                        seed((lot.position.x as u32, lot.position.y as u32)).gen_weighted_bool(3)
                    {
                        Self::spawn_building(
                            self.materialized_reality,
                            lot,
                            self.simulation,
                            world,
                        );
                    }

                }
                BuildingSpawnerState::Idle
            }
            BuildingSpawnerState::Idle => BuildingSpawnerState::Idle,
        }
    }
}

#[derive(Compact, Clone, Default)]
pub struct BuildingPlanResultDelta {
    buildings_to_destroy: CVec<BuildingID>,
}

#[derive(Compact, Clone, Default)]
pub struct MaterializedBuildings {
    buildings: CVec<(P2, BuildingID, LaneID)>,
}

use transport::planning::road_plan::RoadPlanResultDelta;
use std::collections::HashSet;

impl MaterializedBuildings {
    pub fn delta_with_road_result_delta(
        &self,
        road_result_delta: &RoadPlanResultDelta,
    ) -> BuildingPlanResultDelta {
        let all_strokes_to_create =
            road_result_delta
                .intersections
                .to_create
                .values()
                .flat_map(|intersection| intersection.strokes.iter())
                .chain(road_result_delta.trimmed_strokes.to_create.values());

        let mut buildings_to_destroy = HashSet::<BuildingID>::new();

        for stroke_to_create in all_strokes_to_create {
            for &(building_pos, building_id, _) in &self.buildings {
                if stroke_to_create.path().distance_to(building_pos) < MIN_LANE_BUILDING_DISTANCE {
                    buildings_to_destroy.insert(building_id);
                }
            }
        }

        BuildingPlanResultDelta { buildings_to_destroy: buildings_to_destroy.into_iter().collect() }
    }

    pub fn apply(&mut self, world: &mut World, result_delta: &BuildingPlanResultDelta) {
        for building_id in &result_delta.buildings_to_destroy {
            let position = self.buildings
                .iter()
                .position(|&(_, built_building_id, _)| {
                    built_building_id == *building_id
                })
                .expect("Tried to destroy a non-built building");
            self.buildings[position].1.destroy(world);
            self.buildings.remove(position);
        }
    }
}

impl MaterializedReality {
    pub fn on_building_built(&mut self, position: P2, id: BuildingID, lane: LaneID, _: &mut World) {
        self.buildings.buildings.push((position, id, lane));
    }
}

pub fn setup(
    system: &mut ActorSystem,
    user_interface: UserInterfaceID,
    simulation: SimulationID,
    materialized_reality: MaterializedRealityID,
) {
    system.register::<Building>();
    system.register::<BuildingSpawner>();
    rendering::setup(system, user_interface);

    kay_auto::auto_setup(system);

    BuildingSpawnerID::init(
        user_interface,
        simulation,
        materialized_reality,
        &mut system.world(),
    );
}

mod kay_auto;
pub use self::kay_auto::*;
