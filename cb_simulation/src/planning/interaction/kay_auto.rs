//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;





impl PlanManagerID {
    pub fn get_all_plans(&self, ui: PlanningUIID, known_master: KnownHistoryState, known_proposals: CHashMap < ProposalID , KnownProposalState >, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_get_all_plans(ui, known_master, known_proposals));
    }
    
    pub fn get_proposal_preview_update(&self, ui: PlanningUIID, proposal_id: ProposalID, known_result: KnownPlanResultState, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_get_proposal_preview_update(ui, proposal_id, known_result));
    }
    
    pub fn switch_to(&self, machine: MachineID, proposal_id: ProposalID, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_switch_to(machine, proposal_id));
    }
    
    pub fn start_new_gesture(&self, proposal_id: ProposalID, machine_id: MachineID, new_gesture_id: GestureID, intent: GestureIntent, start: P2, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_start_new_gesture(proposal_id, machine_id, new_gesture_id, intent, start));
    }
    
    pub fn finish_gesture(&self, machine_id: MachineID, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_finish_gesture(machine_id));
    }
    
    pub fn add_control_point(&self, proposal_id: ProposalID, gesture_id: GestureID, new_point: P2, add_to_end: bool, commit: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_add_control_point(proposal_id, gesture_id, new_point, add_to_end, commit));
    }
    
    pub fn insert_control_point(&self, proposal_id: ProposalID, gesture_id: GestureID, new_point: P2, commit: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_insert_control_point(proposal_id, gesture_id, new_point, commit));
    }
    
    pub fn move_control_point(&self, proposal_id: ProposalID, gesture_id: GestureID, point_index: u32, new_position: P2, is_move_finished: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_move_control_point(proposal_id, gesture_id, point_index, new_position, is_move_finished));
    }
    
    pub fn split_gesture(&self, proposal_id: ProposalID, gesture_id: GestureID, split_at: P2, commit: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_split_gesture(proposal_id, gesture_id, split_at, commit));
    }
    
    pub fn set_intent(&self, proposal_id: ProposalID, gesture_id: GestureID, new_intent: GestureIntent, is_move_finished: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_set_intent(proposal_id, gesture_id, new_intent, is_move_finished));
    }
    
    pub fn undo(&self, proposal_id: ProposalID, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_undo(proposal_id));
    }
    
    pub fn redo(&self, proposal_id: ProposalID, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_redo(proposal_id));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_get_all_plans(pub PlanningUIID, pub KnownHistoryState, pub CHashMap < ProposalID , KnownProposalState >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_get_proposal_preview_update(pub PlanningUIID, pub ProposalID, pub KnownPlanResultState);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_switch_to(pub MachineID, pub ProposalID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_start_new_gesture(pub ProposalID, pub MachineID, pub GestureID, pub GestureIntent, pub P2);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_finish_gesture(pub MachineID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_add_control_point(pub ProposalID, pub GestureID, pub P2, pub bool, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_insert_control_point(pub ProposalID, pub GestureID, pub P2, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_move_control_point(pub ProposalID, pub GestureID, pub u32, pub P2, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_split_gesture(pub ProposalID, pub GestureID, pub P2, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_set_intent(pub ProposalID, pub GestureID, pub GestureIntent, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_undo(pub ProposalID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_redo(pub ProposalID);


#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_get_all_plans(ui, ref known_master, ref known_proposals), instance, world| {
            instance.get_all_plans(ui, known_master, known_proposals, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_get_proposal_preview_update(ui, proposal_id, ref known_result), instance, world| {
            instance.get_proposal_preview_update(ui, proposal_id, known_result, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_switch_to(machine, proposal_id), instance, world| {
            instance.switch_to(machine, proposal_id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_start_new_gesture(proposal_id, machine_id, new_gesture_id, ref intent, start), instance, world| {
            instance.start_new_gesture(proposal_id, machine_id, new_gesture_id, intent, start, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_finish_gesture(machine_id), instance, world| {
            instance.finish_gesture(machine_id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_add_control_point(proposal_id, gesture_id, new_point, add_to_end, commit), instance, world| {
            instance.add_control_point(proposal_id, gesture_id, new_point, add_to_end, commit, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_insert_control_point(proposal_id, gesture_id, new_point, commit), instance, world| {
            instance.insert_control_point(proposal_id, gesture_id, new_point, commit, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_move_control_point(proposal_id, gesture_id, point_index, new_position, is_move_finished), instance, world| {
            instance.move_control_point(proposal_id, gesture_id, point_index, new_position, is_move_finished, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_split_gesture(proposal_id, gesture_id, split_at, commit), instance, world| {
            instance.split_gesture(proposal_id, gesture_id, split_at, commit, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_set_intent(proposal_id, gesture_id, ref new_intent, is_move_finished), instance, world| {
            instance.set_intent(proposal_id, gesture_id, new_intent, is_move_finished, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_undo(proposal_id), instance, world| {
            instance.undo(proposal_id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_redo(proposal_id), instance, world| {
            instance.redo(proposal_id, world); Fate::Live
        }, false
    );
}