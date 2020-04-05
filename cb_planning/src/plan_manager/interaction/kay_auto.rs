//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;





impl<Logic: PlanningLogic> PlanManagerID<Logic> {
    pub fn get_all_plans(self, ui: PlanningUIID < Logic >, known_master: KnownHistoryState, known_projects: CHashMap < ProjectID , KnownProjectState >, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_get_all_plans::<Logic>(ui, known_master, known_projects));
    }
    
    pub fn get_project_preview_update(self, ui: PlanningUIID < Logic >, project_id: ProjectID, known_result: KnownPlanResultState < Logic :: PrototypeKind >, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_get_project_preview_update::<Logic>(ui, project_id, known_result));
    }
    
    pub fn start_new_gesture(self, project_id: ProjectID, new_gesture_id: GestureID, intent: Logic :: GestureIntent, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_start_new_gesture::<Logic>(project_id, new_gesture_id, intent));
    }
    
    pub fn set_intent(self, project_id: ProjectID, gesture_id: GestureID, new_intent: Logic :: GestureIntent, is_move_finished: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_set_intent::<Logic>(project_id, gesture_id, new_intent, is_move_finished));
    }
    
    pub fn undo(self, project_id: ProjectID, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_undo(project_id));
    }
    
    pub fn redo(self, project_id: ProjectID, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_redo(project_id));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_get_all_plans<Logic: PlanningLogic>(pub PlanningUIID < Logic >, pub KnownHistoryState, pub CHashMap < ProjectID , KnownProjectState >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_get_project_preview_update<Logic: PlanningLogic>(pub PlanningUIID < Logic >, pub ProjectID, pub KnownPlanResultState < Logic :: PrototypeKind >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_start_new_gesture<Logic: PlanningLogic>(pub ProjectID, pub GestureID, pub Logic :: GestureIntent);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_set_intent<Logic: PlanningLogic>(pub ProjectID, pub GestureID, pub Logic :: GestureIntent, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_undo(pub ProjectID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_redo(pub ProjectID);


#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup<Logic: PlanningLogic>(system: &mut ActorSystem) {
    
    
    system.add_handler::<PlanManager<Logic>, _, _>(
        |&MSG_PlanManager_get_all_plans::<Logic>(ui, ref known_master, ref known_projects), instance, world| {
            instance.get_all_plans(ui, known_master, known_projects, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager<Logic>, _, _>(
        |&MSG_PlanManager_get_project_preview_update::<Logic>(ui, project_id, ref known_result), instance, world| {
            instance.get_project_preview_update(ui, project_id, known_result, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager<Logic>, _, _>(
        |&MSG_PlanManager_start_new_gesture::<Logic>(project_id, new_gesture_id, ref intent), instance, world| {
            instance.start_new_gesture(project_id, new_gesture_id, intent, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager<Logic>, _, _>(
        |&MSG_PlanManager_set_intent::<Logic>(project_id, gesture_id, ref new_intent, is_move_finished), instance, world| {
            instance.set_intent(project_id, gesture_id, new_intent, is_move_finished, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager<Logic>, _, _>(
        |&MSG_PlanManager_undo(project_id), instance, world| {
            instance.undo(project_id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager<Logic>, _, _>(
        |&MSG_PlanManager_redo(project_id), instance, world| {
            instance.redo(project_id, world); Fate::Live
        }, false
    );
}