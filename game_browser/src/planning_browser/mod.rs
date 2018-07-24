use kay::Actor;
use stdweb::serde::Serde;
use stdweb::js_export;
use SYSTEM;

#[js_export]
pub fn move_gesture_point(
    proposal_id: Serde<::planning::ProposalID>,
    gesture_id: Serde<::planning::GestureID>,
    point_idx: u32,
    new_position: Serde<::descartes::P2>,
    done_moving: bool,
) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    ::planning::PlanManager::global_first(world).move_control_point(
        proposal_id.0,
        gesture_id.0,
        point_idx,
        new_position.0,
        done_moving,
        world,
    );
}

#[js_export]
pub fn implement_proposal(proposal_id: Serde<::planning::ProposalID>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    ::planning::PlanManager::global_first(world).implement(proposal_id.0, world);
}
