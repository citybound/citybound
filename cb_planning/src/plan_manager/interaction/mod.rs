use kay::{World};
use compact::{CHashMap, COption};
use descartes::AreaError;
use ::{PlanHistory, PlanResult, ActionGroups, KnownHistoryState, KnownProjectState, ProjectUpdate,
PlanningLogic, GestureID, Gesture, Plan, KnownPlanResultState};
use super::{PlanManager, PlanManagerID, ProjectID};
use super::ui::PlanningUIID;
use cb_util::log::error;
const LOG_T: &str = "Planning Interaction";

#[derive(Compact, Clone)]
pub struct PreviewSet<Logic: PlanningLogic> {
    history: PlanHistory<Logic::GestureIntent>,
    result: COption<PlanResult<Logic::PrototypeKind>>,
    actions: COption<ActionGroups>,
}

#[derive(Compact, Clone)]
pub struct PlanManagerUIState<Logic: PlanningLogic> {
    previews: CHashMap<ProjectID, PreviewSet<Logic>>,
}

impl<Logic: PlanningLogic> PlanManagerUIState<Logic> {
    pub fn new() -> Self {
        PlanManagerUIState {
            previews: CHashMap::new(),
        }
    }

    pub fn invalidate(&mut self, project_id: ProjectID) {
        self.previews.remove(project_id);
    }

    pub fn invalidate_all(&mut self) {
        self.previews = CHashMap::new();
    }
}

impl<Logic: PlanningLogic> PlanManager<Logic> {
    pub fn get_all_plans(
        &mut self,
        ui: PlanningUIID<Logic>,
        known_master: &KnownHistoryState,
        known_projects: &CHashMap<ProjectID, KnownProjectState>,
        world: &mut World,
    ) {
        let master_update = self.master_plan.update_for(known_master);
        let mut unmatched_known_projects = known_projects
            .keys()
            .cloned()
            .collect::<::std::collections::HashSet<_>>();
        let project_updates = self
            .projects
            .pairs()
            .map(|(project_id, project)| {
                (
                    *project_id,
                    known_projects
                        .get(*project_id)
                        .map(|known_state| {
                            unmatched_known_projects.remove(project_id);
                            project.update_for(known_state)
                        })
                        .unwrap_or_else(|| ProjectUpdate::ChangedCompletely(project.clone())),
                )
            })
            .collect::<Vec<_>>();
        let project_updates_with_removals = project_updates
            .into_iter()
            .chain(
                unmatched_known_projects
                    .into_iter()
                    .map(|unmatched_id| (unmatched_id, ProjectUpdate::Removed)),
            )
            .collect();
        ui.on_plans_update(master_update, project_updates_with_removals, world);
    }

    pub fn get_project_preview_update(
        &mut self,
        ui: PlanningUIID<Logic>,
        project_id: ProjectID,
        known_result: &KnownPlanResultState<Logic::PrototypeKind>,
        world: &mut World,
    ) {
        let (plan_history, maybe_result, maybe_actions) =
            self.try_ensure_preview(project_id, world);

        if let (Some(result), Some(actions)) = (maybe_result, maybe_actions) {
            ui.on_project_preview_update(
                project_id,
                plan_history.clone(),
                result.update_for(known_result),
                actions.clone(),
                world,
            );
        }
    }

    #[allow(clippy::type_complexity)]
    fn try_ensure_preview(
        &mut self,
        project_id: ProjectID,
        log_in: &mut World,
    ) -> (
        &PlanHistory<Logic::GestureIntent>,
        Option<&PlanResult<Logic::PrototypeKind>>,
        Option<&ActionGroups>,
    ) {
        if !self.ui_state.previews.contains_key(project_id) {
            let preview_history = self
                .projects
                .get(project_id)
                .unwrap()
                .apply_to_with_ongoing(&self.master_plan);

            let maybe_preview_result = match Logic::calculate_result(&preview_history) {
                Ok(preview_plan_result) => Some(preview_plan_result),
                Err(err) => {
                    let err_str = match err {
                        AreaError::LeftOver(string) => format!("Preview Plan Error: {}", string),
                        _ => format!("Preview Plan Error: {:?}", err),
                    };
                    error(LOG_T, err_str, self.id, log_in);
                    None
                }
            };

            let maybe_preview_actions = maybe_preview_result
                .as_ref()
                .map(|preview_plan_result| self.master_result.actions_to(preview_plan_result).0);

            self.ui_state.previews.insert(
                project_id,
                PreviewSet {
                    history: preview_history,
                    result: COption(maybe_preview_result),
                    actions: COption(maybe_preview_actions),
                },
            );
        }

        let preview_set = self
            .ui_state
            .previews
            .get(project_id)
            .expect("Should have previews by now.");

        (
            &preview_set.history,
            preview_set.result.as_ref(),
            preview_set.actions.as_ref(),
        )
    }

    pub fn start_new_gesture(
        &mut self,
        project_id: ProjectID,
        new_gesture_id: GestureID,
        intent: &Logic::GestureIntent,
        _: &mut World,
    ) {
        let new_gesture = Gesture::new(intent.clone());

        let new_step = Plan::from_gestures(Some((new_gesture_id, new_gesture)));

        self.projects
            .get_mut(project_id)
            .unwrap()
            .set_ongoing_step(new_step);
        self.projects.get_mut(project_id).unwrap().start_new_step();

        self.ui_state.invalidate(project_id);
    }

    pub fn set_intent(
        &mut self,
        project_id: ProjectID,
        gesture_id: GestureID,
        new_intent: &Logic::GestureIntent,
        is_move_finished: bool,
        _: &mut World,
    ) {
        let current_change = {
            let current_gesture = self.get_current_version_of(gesture_id, project_id);

            let new_gesture = Gesture {
                intent: new_intent.clone(),
                ..current_gesture.clone()
            };

            Plan::from_gestures(Some((gesture_id, new_gesture)))
        };

        self.projects
            .get_mut(project_id)
            .unwrap()
            .set_ongoing_step(current_change);

        // TODO: can we update only part of the preview
        // for better rendering performance while dragging?
        self.ui_state.invalidate(project_id);

        if is_move_finished {
            self.projects.get_mut(project_id).unwrap().start_new_step();
        }
    }

    pub fn undo(&mut self, project_id: ProjectID, _: &mut World) {
        self.projects.get_mut(project_id).unwrap().undo();
        self.ui_state.invalidate(project_id);
    }

    pub fn redo(&mut self, project_id: ProjectID, _: &mut World) {
        self.projects.get_mut(project_id).unwrap().redo();
        self.ui_state.invalidate(project_id);
    }
}

pub mod kay_auto;
pub use self::kay_auto::*;
