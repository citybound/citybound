use kay::Actor;
use stdweb::serde::Serde;
use stdweb::js_export;
use SYSTEM;

#[js_export]
pub fn plan_grid(proposal_id: Serde<::planning::ProposalID>, n: usize, spacing: Serde<f32>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();

    let plan_manager = ::planning::PlanManager::global_first(world);

    use ::transport::transport_planning::RoadIntent;
    use ::planning::{GestureID, GestureIntent};
    use ::descartes::{N, P2};

    for x in 0..n {
        let id = GestureID::new();
        let p1 = P2::new(x as f32 * spacing.0, 0.0);
        let p2 = P2::new(x as f32 * spacing.0, n as f32 * spacing.0);
        plan_manager.start_new_gesture(
            proposal_id.0,
            ::kay::MachineID(0),
            id,
            GestureIntent::Road(RoadIntent::new(3, 3)),
            p1,
            world,
        );
        plan_manager.add_control_point(proposal_id.0, id, p2, true, true, world);
    }

    for y in 0..n {
        let id = GestureID::new();
        let p1 = P2::new(0.0, y as f32 * spacing.0);
        let p2 = P2::new(n as f32 * spacing.0, y as f32 * spacing.0);
        plan_manager.start_new_gesture(
            proposal_id.0,
            ::kay::MachineID(0),
            id,
            GestureIntent::Road(RoadIntent::new(3, 3)),
            p1,
            world,
        );
        plan_manager.add_control_point(proposal_id.0, id, p2, true, true, world);
    }
}

#[js_export]
pub fn spawn_cars(tries_per_lane: usize) {
    use transport::lane::Lane;
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    for _ in 0..tries_per_lane {
        Lane::global_broadcast(world).manually_spawn_car_add_lane(world);
    }
}
