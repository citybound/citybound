use kay::{ActorSystem, World, External};
use compact::CVec;
use descartes::{P2, V2, Norm, Curve};
use stagemaster::combo::{Bindings, Combo2};
use stagemaster::{UserInterfaceID, Event3d, Interactable3d, Interactable3dID,
                  MSG_Interactable3d_on_event};
use stagemaster::combo::Button::*;
use stagemaster::geometry::AnyShape;
use transport::lane::{Lane, LaneID};
use planning::materialized_reality::{MaterializedReality, MaterializedRealityID};


pub mod rendering;

use super::households::HouseholdID;

#[derive(Compact, Clone)]
pub struct Building {
    id: BuildingID,
    households: CVec<HouseholdID>,
    lot: Lot,
}

use stagemaster::geometry::add_debug_line;

impl Building {
    pub fn spawn(
        id: BuildingID,
        materialized_reality: MaterializedRealityID,
        households: &CVec<HouseholdID>,
        lot: &Lot,
        world: &mut World,
    ) -> Building {
        add_debug_line(
            lot.position,
            lot.adjacent_lane_position,
            [0.5, 0.5, 0.5],
            0.0,
            world,
        );
        materialized_reality.on_building_built(lot.position, id, lot.adjacent_lane, world);
        Building {
            id,
            households: households.clone(),
            lot: lot.clone(),
        }
    }

    pub fn add_household(&mut self, household: HouseholdID, world: &mut World) {
        self.households.push(household);
        // TODO: such a weird place to do this, but ok for now
        if self.households.len() == 1 {
            rendering::on_add(self, world);
        }
    }
}

use transport::pathfinding::{RoughLocation, LocationRequesterID, PositionRequesterID,
                             RoughLocationID, MSG_RoughLocation_resolve_as_location,
                             MSG_RoughLocation_resolve_as_position};
use core::simulation::Instant;

impl RoughLocation for Building {
    fn resolve_as_location(
        &mut self,
        requester: LocationRequesterID,
        rough_location: RoughLocationID,
        instant: Instant,
        world: &mut World,
    ) {
        Into::<RoughLocationID>::into(self.lot.adjacent_lane)
            .resolve_as_location(requester, rough_location, instant, world)
    }

    fn resolve_as_position(
        &mut self,
        requester: PositionRequesterID,
        rough_location: RoughLocationID,
        world: &mut World,
    ) {
        requester.position_resolved(rough_location, self.lot.position, world)
    }
}

#[derive(Compact, Clone)]
pub struct Lot {
    pub position: P2,
    pub orientation: V2,
    pub adjacent_lane: LaneID,
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
        let building_id = BuildingID::spawn(materialized_reality, CVec::new(), lot.clone(), world);

        if building_id._raw_id.instance_id % 6 == 0 {
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

const MIN_LANE_BUILDING_DISTANCE: f32 = 15.0;

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

use core::simulation::{Sleeper, SleeperID, MSG_Sleeper_wake};

impl Sleeper for BuildingSpawner {
    fn wake(&mut self, _time: Instant, world: &mut World) {
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
            for building in &self.buildings {
                if stroke_to_create.path().distance_to(building.0) < MIN_LANE_BUILDING_DISTANCE {
                    buildings_to_destroy.insert(building.1);
                }
            }
        }

        println!("Buildings to destroy: {:?}", buildings_to_destroy);

        BuildingPlanResultDelta { buildings_to_destroy: buildings_to_destroy.into_iter().collect() }
    }
}

impl MaterializedReality {
    pub fn on_building_built(&mut self, position: P2, id: BuildingID, lane: LaneID, _: &mut World) {
        self.buildings.buildings.push((position, id, lane));
    }
}

use super::households::family::FamilyID;
use super::households::grocery_shop::GroceryShopID;
use core::simulation::{SimulationID, Ticks};

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
