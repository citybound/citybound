//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;





impl PlanManagerID {
    pub fn get_all_plans(self, ui: PlanningUIID, known_master: KnownHistoryState, known_projects: CHashMap < ProjectID , KnownProjectState >, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_get_all_plans(ui, known_master, known_projects));
    }
    
    pub fn get_project_preview_update(self, ui: PlanningUIID, project_id: ProjectID, known_result: KnownPlanResultState, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_get_project_preview_update(ui, project_id, known_result));
    }
    
    pub fn start_new_gesture(self, project_id: ProjectID, new_gesture_id: GestureID, intent: GestureIntent, start: P2, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_start_new_gesture(project_id, new_gesture_id, intent, start));
    }
    
    pub fn add_control_point(self, project_id: ProjectID, gesture_id: GestureID, new_point: P2, add_to_end: bool, commit: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_add_control_point(project_id, gesture_id, new_point, add_to_end, commit));
    }
    
    pub fn insert_control_point(self, project_id: ProjectID, gesture_id: GestureID, new_point: P2, commit: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_insert_control_point(project_id, gesture_id, new_point, commit));
    }
    
    pub fn move_control_point(self, project_id: ProjectID, gesture_id: GestureID, point_index: u32, new_position: P2, is_move_finished: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_move_control_point(project_id, gesture_id, point_index, new_position, is_move_finished));
    }
    
    pub fn split_gesture(self, project_id: ProjectID, gesture_id: GestureID, split_at: P2, commit: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_split_gesture(project_id, gesture_id, split_at, commit));
    }
    
    pub fn set_intent(self, project_id: ProjectID, gesture_id: GestureID, new_intent: GestureIntent, is_move_finished: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_set_intent(project_id, gesture_id, new_intent, is_move_finished));
    }
    
    pub fn undo(self, project_id: ProjectID, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_undo(project_id));
    }
    
    pub fn redo(self, project_id: ProjectID, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_redo(project_id));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_get_all_plans(pub PlanningUIID, pub KnownHistoryState, pub CHashMap < ProjectID , KnownProjectState >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_get_project_preview_update(pub PlanningUIID, pub ProjectID, pub KnownPlanResultState);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_start_new_gesture(pub ProjectID, pub GestureID, pub GestureIntent, pub P2);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_add_control_point(pub ProjectID, pub GestureID, pub P2, pub bool, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_insert_control_point(pub ProjectID, pub GestureID, pub P2, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_move_control_point(pub ProjectID, pub GestureID, pub u32, pub P2, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_split_gesture(pub ProjectID, pub GestureID, pub P2, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_set_intent(pub ProjectID, pub GestureID, pub GestureIntent, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_undo(pub ProjectID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_redo(pub ProjectID);


#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_get_all_plans(ui, ref known_master, ref known_projects), instance, world| {
            instance.get_all_plans(ui, known_master, known_projects, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_get_project_preview_update(ui, project_id, ref known_result), instance, world| {
            instance.get_project_preview_update(ui, project_id, known_result, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_start_new_gesture(project_id, new_gesture_id, ref intent, start), instance, world| {
            instance.start_new_gesture(project_id, new_gesture_id, intent, start, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_add_control_point(project_id, gesture_id, new_point, add_to_end, commit), instance, world| {
            instance.add_control_point(project_id, gesture_id, new_point, add_to_end, commit, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_insert_control_point(project_id, gesture_id, new_point, commit), instance, world| {
            instance.insert_control_point(project_id, gesture_id, new_point, commit, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_move_control_point(project_id, gesture_id, point_index, new_position, is_move_finished), instance, world| {
            instance.move_control_point(project_id, gesture_id, point_index, new_position, is_move_finished, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_split_gesture(project_id, gesture_id, split_at, commit), instance, world| {
            instance.split_gesture(project_id, gesture_id, split_at, commit, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_set_intent(project_id, gesture_id, ref new_intent, is_move_finished), instance, world| {
            instance.set_intent(project_id, gesture_id, new_intent, is_move_finished, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_undo(project_id), instance, world| {
            instance.undo(project_id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_redo(project_id), instance, world| {
            instance.redo(project_id, world); Fate::Live
        }, false
    );
}