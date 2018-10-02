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
pub fn start_new_gesture(
    proposal_id: Serde<::planning::ProposalID>,
    gesture_id: Serde<::planning::GestureID>,
    intent: Serde<::planning::GestureIntent>,
    start: Serde<::descartes::P2>,
) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    ::planning::PlanManager::global_first(world).start_new_gesture(
        proposal_id.0,
        ::kay::MachineID(0),
        gesture_id.0,
        intent.0,
        start.0,
        world,
    )
}

#[js_export]
pub fn add_control_point(
    proposal_id: Serde<::planning::ProposalID>,
    gesture_id: Serde<::planning::GestureID>,
    new_point: Serde<::descartes::P2>,
    add_to_end: bool,
    done_adding: bool,
) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    ::planning::PlanManager::global_first(world).add_control_point(
        proposal_id.0,
        gesture_id.0,
        new_point.0,
        add_to_end,
        done_adding,
        world,
    )
}

#[js_export]
pub fn finish_gesture() {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    ::planning::PlanManager::global_first(world).finish_gesture(::kay::MachineID(0), world)
}

#[js_export]
pub fn undo(proposal_id: Serde<::planning::ProposalID>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    ::planning::PlanManager::global_first(world).undo(proposal_id.0, world)
}

#[js_export]
pub fn redo(proposal_id: Serde<::planning::ProposalID>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    ::planning::PlanManager::global_first(world).redo(proposal_id.0, world)
}

#[js_export]
pub fn implement_proposal(proposal_id: Serde<::planning::ProposalID>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    ::planning::PlanManager::global_first(world).implement(proposal_id.0, world);
}
