use kay::{ActorSystem, World, External};
use compact::CVec;
use descartes::{P2, Norm, Curve};
use stagemaster::combo::{Bindings, Combo2};
use stagemaster::{UserInterfaceID, Event3d, Interactable3d, Interactable3dID,
                  MSG_Interactable3d_on_event};
use stagemaster::combo::Button::*;
use stagemaster::geometry::AnyShape;
use transport::lane::{Lane, LaneID};

pub mod rendering;

use super::households::HouseholdID;

#[derive(Compact, Clone)]
pub struct Building {
    id: BuildingID,
    households: CVec<HouseholdID>,
    lot: Lot,
}

impl Building {
    pub fn spawn(
        id: BuildingID,
        households: &CVec<HouseholdID>,
        lot: &Lot,
        world: &mut World,
    ) -> Building {
        rendering::on_add(id, lot.position, world);
        Building {
            id,
            households: households.clone(),
            lot: lot.clone(),
        }
    }

    pub fn add_household(&mut self, household: HouseholdID, _: &mut World) {
        self.households.push(household);
    }
}

use transport::pathfinding::{RoughLocation, LocationRequesterID, RoughLocationID,
                             MSG_RoughLocation_resolve_as_location};
use core::simulation::Timestamp;

impl RoughLocation for Building {
    fn resolve_as_location(
        &mut self,
        requester: LocationRequesterID,
        rough_location: RoughLocationID,
        tick: Timestamp,
        world: &mut World,
    ) {
        Into::<RoughLocationID>::into(self.lot.adjacent_lane)
            .resolve_as_location(requester, rough_location, tick, world)
    }
}

#[derive(Compact, Clone)]
pub struct Lot {
    pub position: P2,
    pub adjacent_lane: LaneID,
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
    bindings: External<BuildingSpawnerBindings>,
    state: BuildingSpawnerState,
}

impl BuildingSpawner {
    pub fn init(
        id: BuildingSpawnerID,
        user_interface: UserInterfaceID,
        simulation: SimulationID,
        world: &mut World,
    ) -> BuildingSpawner {
        user_interface.add(id.into(), AnyShape::Everywhere, 0, world);
        user_interface.focus(id.into(), world);

        BuildingSpawner {
            id,
            simulation,
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

    fn spawn_building(lot: &Lot, simulation: SimulationID, world: &mut World) {
        let building_id = BuildingID::spawn(CVec::new(), lot.clone(), world);

        if building_id._raw_id.sub_actor_id % 6 == 0 {
            let shop_id = GroceryShopID::move_into(building_id, world);
            building_id.add_household(shop_id.into(), world);
        } else {
            let family_id = FamilyID::move_into(3, building_id, simulation, world);
            building_id.add_household(family_id.into(), world);
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
                    LaneID::global_broadcast(world).find_lot(self.id, world);
                    self.simulation.wake_up_in(Ticks(10), self.id.into(), world);
                    self.state = BuildingSpawnerState::Collecting(CVec::new());
                }
            }
        };
    }
}

const MIN_BUILDING_DISTANCE: f32 = 20.0;

trait LotConflictor {
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
                    (lot.position - self.lot.position).norm() > MIN_BUILDING_DISTANCE
                })
                .collect(),
            world,
        )
    }
}

impl LotConflictor for Lane {
    fn find_conflicts(
        &mut self,
        lots: &CVec<Lot>,
        requester: BuildingSpawnerID,
        world: &mut World,
    ) {
        const MIN_LANE_BUILDING_DISTANCE: f32 = 10.0;

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

use core::simulation::{Sleeper, SleeperID, MSG_Sleeper_wake};

impl Sleeper for BuildingSpawner {
    fn wake(&mut self, _time: Timestamp, world: &mut World) {
        self.state = match self.state {
            BuildingSpawnerState::Collecting(ref mut lots) => {
                let buildings: LotConflictorID = BuildingID::global_broadcast(world).into();
                let mut nonconflicting_lots = CVec::<Lot>::new();
                for lot in lots.iter() {
                    let far_from_all = nonconflicting_lots.iter().all(|other_lot| {
                        (lot.position - other_lot.position).norm() > MIN_BUILDING_DISTANCE
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
                let lanes = LotConflictorID { _raw_id: world.global_broadcast::<Lane>() };
                lanes.find_conflicts(new_lots.clone(), self.id, world);
                self.simulation.wake_up_in(Ticks(10), self.id.into(), world);

                let new_lots_len = new_lots.len();
                BuildingSpawnerState::CheckingLanes(new_lots, vec![true; new_lots_len].into())
            }
            BuildingSpawnerState::CheckingLanes(ref mut lots, ref mut feasible) => {
                for (lot, feasible) in lots.iter().zip(feasible) {
                    if *feasible {
                        Self::spawn_building(lot, self.simulation, world);
                    }
                }
                BuildingSpawnerState::Idle
            }
            BuildingSpawnerState::Idle => BuildingSpawnerState::Idle,
        }
    }
}

#[derive(Copy, Clone)]
pub struct InitializeUI;

use super::households::family::FamilyID;
use super::households::grocery_shop::GroceryShopID;
use core::simulation::{SimulationID, Ticks};

pub fn setup(system: &mut ActorSystem, user_interface: UserInterfaceID, simulation: SimulationID) {
    system.register::<Building>();
    system.register::<BuildingSpawner>();
    rendering::setup(system, user_interface);

    kay_auto::auto_setup(system);

    BuildingSpawnerID::init(user_interface, simulation, &mut system.world());
}

mod kay_auto;
pub use self::kay_auto::*;
