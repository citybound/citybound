use kay::{World, ActorSystem, TypedID};
use ::construction::ConstructionID;
use ::{PlanHistory, PlanResult, Gesture, Project, GestureID, PrototypeID, VersionedGesture,
PlanningLogic};
use compact::{CVec, CHashMap};
use cb_util::random::{Uuid, uuid};
use cb_util::log::{error, info};
const LOG_T: &str = "Planning";

pub mod interaction;
use self::interaction::PlanManagerUIState;
pub mod ui;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct ProjectID(pub Uuid);

impl ProjectID {
    pub fn new() -> ProjectID {
        ProjectID(uuid())
    }
}

#[derive(Compact, Clone)]
//#[derive(Clone)]
pub struct PlanManager<Logic: PlanningLogic + 'static> {
    id: PlanManagerID<Logic>,
    master_plan: PlanHistory<Logic::GestureIntent>,
    master_result: PlanResult<Logic::PrototypeKind>,
    projects: CHashMap<ProjectID, Project<Logic::GestureIntent>>,
    implemented_projects: CHashMap<ProjectID, Project<Logic::GestureIntent>>,
    ui_state: PlanManagerUIState<Logic>,
}

//mod compact_workaround;

impl<Logic: PlanningLogic + 'static> PlanManager<Logic> {
    pub fn spawn(id: PlanManagerID<Logic>, _: &mut World) -> PlanManager<Logic> {
        PlanManager {
            id,
            master_plan: PlanHistory::new(),
            master_result: PlanResult::new(),
            projects: CHashMap::new(),
            implemented_projects: CHashMap::new(),
            ui_state: PlanManagerUIState::new(),
        }
    }

    pub fn get_current_version_of(
        &self,
        gesture_id: GestureID,
        project_id: ProjectID,
    ) -> &Gesture<Logic::GestureIntent> {
        self.projects
            .get(project_id)
            .expect("Expected project to exist")
            .current_history()
            .iter()
            .rfold(None, |found, step| {
                found.or_else(|| step.gestures.get(gesture_id))
            })
            .into_iter()
            .chain(
                self.master_plan
                    .gestures
                    .get(gesture_id)
                    .map(|VersionedGesture(ref g, _)| g),
            )
            .next()
            .expect("Expected gesture (that point should be added to) to exist!")
    }

    pub fn start_new_project(&mut self, project_id: ProjectID, _: &mut World) {
        self.projects.insert(project_id, Project::new());
    }

    pub fn implement(&mut self, project_id: ProjectID, world: &mut World) {
        let project = self
            .projects
            .remove(project_id)
            .expect("Project should exist");

        self.master_plan = project.apply_to(&self.master_plan);

        match Logic::calculate_result(&self.master_plan) {
            Ok(result) => {
                let (actions, new_prototypes) = self.master_result.actions_to(&result);
                ConstructionID::<Logic::PrototypeKind>::global_first(world).implement(
                    actions,
                    new_prototypes,
                    world,
                );
                self.implemented_projects.insert(project_id, project);
                self.master_result = result;

                self.ui_state.invalidate_all();
            }
            Err(err) => {
                let err_str = match err {
                    ::descartes::AreaError::LeftOver(string) => {
                        format!("Implement Plan Error: {}", string)
                    }
                    _ => format!("Implement Plan Error: {:?}", err),
                };
                error(LOG_T, err_str, self.id, world);
            }
        }
    }

    pub fn implement_artificial_project(
        &mut self,
        project: &Project<Logic::GestureIntent>,
        based_on: &CVec<PrototypeID>,
        world: &mut World,
    ) {
        if based_on
            .iter()
            .all(|prototype_id| self.master_result.prototypes.contains_key(*prototype_id))
        {
            let project_id = ProjectID::new();
            self.projects.insert(project_id, project.clone());
            self.implement(project_id, world);
        } else {
            info(
                LOG_T,
                "Tried to implement artificial project based on outdated prototypes",
                self.id,
                world,
            );
        }
    }
}

pub fn setup<Logic: PlanningLogic + 'static>(system: &mut ActorSystem) {
    system.register::<PlanManager<Logic>>();
    auto_setup::<Logic>(system);
    interaction::auto_setup::<Logic>(system);
    ui::auto_setup::<Logic>(system);
}

pub fn spawn<Logic: PlanningLogic + 'static>(world: &mut World) -> PlanManagerID<Logic> {
    PlanManagerID::spawn(world)
}

pub mod kay_auto;
pub use self::kay_auto::*;
