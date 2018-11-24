use kay::World;
use compact::CHashMap;
use super::{PlanHistory, PlanHistoryUpdate, ProjectID, ProjectUpdate,
PlanResultUpdate, ActionGroups};

pub trait PlanningUI {
    fn on_plans_update(
        &mut self,
        master_update: &PlanHistoryUpdate,
        project_updates: &CHashMap<ProjectID, ProjectUpdate>,
        _world: &mut World,
    );

    fn on_project_preview_update(
        &mut self,
        _project_id: ProjectID,
        effective_history: &PlanHistory,
        result_update: &PlanResultUpdate,
        new_actions: &ActionGroups,
        _world: &mut World,
    );
}

pub mod kay_auto;
pub use self::kay_auto::*;
