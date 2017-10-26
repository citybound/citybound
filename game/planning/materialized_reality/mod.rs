use kay::{ActorSystem, World};

// TODO: How to evolve this:
// - get rid of building confirmations, use that we synchronously get IDs for spawned stuff
// - switch to a system that can constantly be in flux, with a backlog of ConstructionTaskBatches

use super::plan::{Plan, PlanDelta, PlanResult, PlanResultDelta};
use super::plan_manager::PlanManagerID;
use transport::planning::materialized_roads::{MaterializedRoads, RoadUpdateState};
use economy::buildings::MaterializedBuildings;

// TODO: a lot of this shouldn't be `pub` - all the different aspects should rather
//       just define helper functions, instead of extending the `MaterializedReality` actor
#[derive(Compact, Clone)]
pub struct MaterializedReality {
    id: MaterializedRealityID,
    current_plan: Plan,
    current_result: PlanResult,
    pub state: MaterializedRealityState,
    pub roads: MaterializedRoads,
    pub buildings: MaterializedBuildings,
}

#[allow(large_enum_variant)]
#[derive(Compact, Clone)]
pub enum MaterializedRealityState {
    Ready(()),
    Updating(PlanManagerID, Plan, PlanResult, PlanResultDelta, RoadUpdateState),
}

use self::MaterializedRealityState::{Ready, Updating};

impl MaterializedReality {
    pub fn spawn(id: MaterializedRealityID, _: &mut World) -> MaterializedReality {
        MaterializedReality {
            id,
            current_plan: Plan::default(),
            current_result: PlanResult::default(),
            state: MaterializedRealityState::Ready(()),
            roads: MaterializedRoads::default(),
            buildings: MaterializedBuildings::default(),
        }
    }

    pub fn simulate(&mut self, requester: PlanManagerID, delta: &PlanDelta, world: &mut World) {
        let new_plan = self.current_plan.with_delta(delta);
        let result = new_plan.get_result();
        let result_delta = result.delta(&self.current_result, &self.buildings, &self.roads);
        requester.on_simulation_result(result_delta, world);
    }

    pub fn apply(&mut self, requester: PlanManagerID, delta: &PlanDelta, world: &mut World) {
        self.state = match self.state {
            Updating(..) => panic!("Already applying a plan"),
            Ready(()) => {
                let new_plan = self.current_plan.with_delta(delta);
                let new_result = new_plan.get_result();
                let result_delta = new_result.delta(&self.current_result, &self.buildings, &self.roads);

                let road_update_state = MaterializedRoads::start_applying_roads(self.id, &mut self.roads, &result_delta.roads, world);
                self.buildings.apply(world, &result_delta.buildings);

                Updating(
                    requester,
                    new_plan,
                    new_result,
                    result_delta,
                    road_update_state
                )
            }
        };

        self.check_if_done(world);
    }

    pub fn check_if_done(&mut self, world: &mut World) {
        let maybe_new_state = match self.state {
            Updating(requester,
                     ref new_plan,
                     ref new_result,
                     ref result_delta,
                     ref road_update_state) => {
                if road_update_state.done() {
                    let (new_roads, new_roads_view) = MaterializedRoads::finish_applying_roads(
                        self.id,
                        &self.roads,
                        &new_plan.roads,
                        &result_delta.roads,
                        world,
                    );

                    requester.materialized_reality_changed(new_roads_view, world);

                    Some(MaterializedReality {
                        id: self.id,
                        current_plan: new_plan.clone(),
                        current_result: new_result.clone(),
                        roads: new_roads,
                        buildings: self.buildings.clone(),
                        state: Ready(()),
                    })
                } else {
                    None
                }
            }
            Ready(()) => panic!("Checked if done in Ready state"),
        };

        if let Some(new_state) = maybe_new_state {
            *self = new_state;
        }
    }
}

pub fn setup(system: &mut ActorSystem) -> MaterializedRealityID {
    system.register::<MaterializedReality>();

    auto_setup(system);

    if system.networking_machine_id() > 0 {
        MaterializedRealityID::global_first(&mut system.world())
    } else {
        MaterializedRealityID::spawn(&mut system.world())
    }
}

mod kay_auto;
pub use self::kay_auto::*;
