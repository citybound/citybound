use kay::{ID, ActorSystem, Fate, World, External};
use kay::swarm::{Swarm, ToRandom};
use compact::CVec;
use descartes::{P2, Norm, Curve};
use stagemaster::combo::{Bindings, Combo2};
use stagemaster::{UserInterface, AddInteractable, Focus};
use stagemaster::combo::Button::*;
use stagemaster::Event3d;
use stagemaster::geometry::AnyShape;
use transport::lane::Lane;

mod rendering;

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
        let adjacent_lane = RoughLocationID { _raw_id: self.lot.adjacent_lane };
        adjacent_lane.resolve_as_location(requester, rough_location, tick, world)
    }
}

#[derive(Compact, Clone)]
pub struct Lot {
    pub position: P2,
    pub adjacent_lane: ID,
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
    bindings: External<BuildingSpawnerBindings>,
    state: BuildingSpawnerState,
}

impl BuildingSpawner {
    pub fn init(id: BuildingSpawnerID, world: &mut World) -> BuildingSpawner {
        world.send_to_id_of::<UserInterface, _>(
            AddInteractable(id._raw_id, AnyShape::Everywhere, 0),
        );
        world.send_to_id_of::<UserInterface, _>(Focus(id._raw_id));

        BuildingSpawner {
            id,
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

    fn spawn_building(lot: &Lot, world: &mut World) {
        let building_id = BuildingID::spawn(CVec::new(), lot.clone(), world);

        if building_id._raw_id.sub_actor_id % 6 == 0 {
            let shop_id = GroceryShopID::move_into(building_id, world);
            building_id.add_household(shop_id.into(), world);
        } else {
            let family_id = FamilyID::move_into(3, building_id, world);
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

// TODO: remove this once Lane is a newstyle actor
struct LaneID(ID);

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
                let buildings: LotConflictorID = BuildingID::broadcast(world).into();
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
                world.send_to_id_of::<Simulation, _>(WakeUpIn(Ticks(10), self.id.into()));
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
                let lanes = LotConflictorID { _raw_id: world.id::<Swarm<Lane>>().broadcast() };
                lanes.find_conflicts(new_lots.clone(), self.id, world);
                world.send_to_id_of::<Simulation, _>(WakeUpIn(Ticks(10), self.id.into()));
                let new_lots_len = new_lots.len();
                BuildingSpawnerState::CheckingLanes(new_lots, vec![true; new_lots_len].into())
            }
            BuildingSpawnerState::CheckingLanes(ref mut lots, ref mut feasible) => {
                for (lot, feasible) in lots.iter().zip(feasible) {
                    if *feasible {
                        Self::spawn_building(lot, world);
                    }
                }
                BuildingSpawnerState::Idle
            }
            BuildingSpawnerState::Idle => BuildingSpawnerState::Idle,
        }
    }
}

#[derive(Copy, Clone)]
pub struct FindLot {
    pub requester: BuildingSpawnerID,
}

#[derive(Copy, Clone)]
pub struct InitializeUI;

use super::households::family::FamilyID;
use super::households::grocery_shop::GroceryShopID;
use core::simulation::{Simulation, WakeUpIn, Ticks};

pub fn setup(system: &mut ActorSystem) {
    system.add(Swarm::<Building>::new(), |_| {});
    system.add(
        Swarm::<BuildingSpawner>::new(),
        Swarm::<BuildingSpawner>::subactors(|mut each_spawner| {

            each_spawner.on(move |event, spawner, world| {
                if let Event3d::Combos(combos) = *event {
                    spawner.bindings.0.do_rebinding(&combos.current);

                    if spawner.bindings.0["Spawn Building"].is_freshly_in(&combos) {
                        if let BuildingSpawnerState::Idle = spawner.state {
                            world.send_to_id_of::<Swarm<Lane>, _>(ToRandom {
                                message: FindLot { requester: spawner.id },
                                n_recipients: 5000,
                            });
                            world.send_to_id_of::<Simulation, _>(
                                WakeUpIn(Ticks(10), spawner.id.into()),
                            );
                            spawner.state = BuildingSpawnerState::Collecting(CVec::new());
                        }
                    }
                };

                Fate::Live
            });
        }),
    );

    kay_auto::auto_setup(system);

    BuildingSpawnerID::init(&mut system.world());
}

pub fn setup_ui(system: &mut ActorSystem) {
    rendering::setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
