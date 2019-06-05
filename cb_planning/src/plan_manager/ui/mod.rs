use kay::World;
use compact::CHashMap;
use ::{PlanHistory, PlanHistoryUpdate, ProjectUpdate, PlanResultUpdate, ActionGroups, PlanningLogic};
use super::ProjectID;

pub trait PlanningUI<Logic: PlanningLogic> {
    fn on_plans_update(
        &mut self,
        master_update: &PlanHistoryUpdate<Logic::GestureIntent>,
        project_updates: &CHashMap<ProjectID, ProjectUpdate<Logic::GestureIntent>>,
        _world: &mut World,
    );

    fn on_project_preview_update(
        &mut self,
        _project_id: ProjectID,
        effective_history: &PlanHistory<Logic::GestureIntent>,
        result_update: &PlanResultUpdate<Logic::PrototypeKind>,
        new_actions: &ActionGroups,
        _world: &mut World,
    );
}

pub mod kay_auto;
pub use self::kay_auto::*;
