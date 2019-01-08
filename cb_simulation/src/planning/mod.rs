#![allow(clippy::new_without_default_derive)]
#![allow(clippy::new_without_default)]
use kay::{World, MachineID, ActorSystem, TypedID};
use compact::{CVec, COption, CHashMap};
use descartes::{N, P2, AreaError};
use util::random::{seed, RngCore, Uuid, uuid};
use std::hash::Hash;

use transport::transport_planning::{RoadIntent, RoadPrototype};
use land_use::zone_planning::{ZoneIntent, BuildingIntent, LotPrototype};
use environment::vegetation::{PlantIntent, PlantPrototype};
use construction::ConstructionID;

use log::{error, info};
const LOG_T: &str = "Planning";

pub mod interaction;
pub mod ui;

// idea for improvement:
// - everything (Gestures, Prototypes) immutable (helps caching)
// - everything separated by staggered grid (to save work)

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub struct Gesture {
    pub points: CVec<P2>,
    pub intent: GestureIntent,
    deleted: bool,
}

impl Gesture {
    pub fn new(points: CVec<P2>, intent: GestureIntent) -> Self {
        Gesture {
            points,
            intent,
            deleted: false,
        }
    }

    pub fn simplify(self) -> Gesture {
        match self.intent {
            GestureIntent::Road(_) => Gesture {
                points: ::transport::transport_planning::simplify_road_path(self.points.clone()),
                ..self
            },
            _ => self,
        }
    }
}

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub enum GestureIntent {
    Road(RoadIntent),
    Zone(ZoneIntent),
    Building(BuildingIntent),
    Plant(PlantIntent),
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct GestureID(pub Uuid);

impl GestureID {
    pub fn new() -> GestureID {
        GestureID(uuid())
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StepID(pub Uuid);

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub step_id: StepID,
    pub gestures: CHashMap<GestureID, Gesture>,
}

impl Plan {
    pub fn new() -> Plan {
        Plan {
            step_id: StepID(uuid()),
            gestures: CHashMap::new(),
        }
    }

    pub fn from_gestures<I: IntoIterator<Item = (GestureID, Gesture)>>(gestures: I) -> Plan {
        Plan {
            step_id: StepID(uuid()),
            gestures: gestures.into_iter().collect(),
        }
    }
}

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub struct VersionedGesture(pub Gesture, pub StepID);

#[derive(Compact, Clone, Serialize, Deserialize)]
pub struct PlanHistory {
    pub gestures: CHashMap<GestureID, VersionedGesture>,
    pub steps: CVec<StepID>,
}

impl PlanHistory {
    pub fn new() -> PlanHistory {
        PlanHistory {
            gestures: CHashMap::new(),
            steps: vec![StepID(uuid())].into(),
        }
    }

    pub fn and_then<'a, I: IntoIterator<Item = &'a Plan>>(&self, plans: I) -> PlanHistory {
        let mut history = self.clone();

        for plan in plans {
            for (gesture_id, gesture) in plan.gestures.pairs() {
                history
                    .gestures
                    .insert(*gesture_id, VersionedGesture(gesture.clone(), plan.step_id));
                history.steps.push(plan.step_id);
            }
        }

        history
    }

    pub fn latest_step_id(&self) -> StepID {
        *self.steps.last().expect("should always have a step")
    }

    pub fn in_order(&self, step_a: &StepID, step_b: &StepID) -> Option<bool> {
        let mut saw_a = false;
        let mut saw_b = false;

        for step in &self.steps {
            if step == step_b {
                if saw_a {
                    return Some(true);
                } else {
                    saw_b = true;
                }
            } else if step == step_a {
                if saw_b {
                    return Some(false);
                } else {
                    saw_a = true;
                }
            }
        }

        None
    }

    pub fn newer_step(&self, step_a: &StepID, step_b: &StepID) -> StepID {
        if step_a == step_b {
            *step_a
        } else if self
            .in_order(step_a, step_b)
            .expect("both steps should be in history")
        {
            *step_b
        } else {
            *step_a
        }
    }

    pub fn as_known_state(&self) -> KnownHistoryState {
        KnownHistoryState {
            known_steps: self.steps.clone(),
        }
    }

    pub fn update_for(&self, known_state: &KnownHistoryState) -> PlanHistoryUpdate {
        let first_different_index = self
            .steps
            .iter()
            .zip(known_state.known_steps.iter())
            .take_while(|(a, b)| a == b)
            .count();

        PlanHistoryUpdate {
            steps_to_drop: known_state.known_steps[first_different_index..]
                .iter()
                .cloned()
                .collect(),
            steps_to_add: self.steps[first_different_index..]
                .iter()
                .cloned()
                .collect(),
            gestures_to_add: self
                .gestures
                .pairs()
                .filter_map(|(gesture_id, versioned_gesture)| {
                    if self.steps[first_different_index..].contains(&versioned_gesture.1) {
                        Some((*gesture_id, versioned_gesture.clone()))
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }

    pub fn apply_update(&mut self, update: &PlanHistoryUpdate) {
        let gestures_to_drop = self
            .gestures
            .pairs()
            .filter_map(|(gesture_id, versioned_gesture)| {
                if update.steps_to_drop.contains(&versioned_gesture.1) {
                    Some(*gesture_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for gesture_id in gestures_to_drop {
            self.gestures.remove(gesture_id);
        }

        self.steps
            .retain(|step| !update.steps_to_drop.contains(step));
        self.steps.extend(update.steps_to_add.iter().cloned());

        for (new_gesture_id, new_gesture) in update.gestures_to_add.pairs() {
            self.gestures.insert(*new_gesture_id, new_gesture.clone());
        }
    }
}

#[derive(Compact, Clone)]
pub struct KnownHistoryState {
    known_steps: CVec<StepID>,
}

#[derive(Compact, Clone, Debug)]
pub struct PlanHistoryUpdate {
    steps_to_drop: CVec<StepID>,
    steps_to_add: CVec<StepID>,
    gestures_to_add: CHashMap<GestureID, VersionedGesture>,
}

impl PlanHistoryUpdate {
    pub fn is_empty(&self) -> bool {
        self.steps_to_drop.is_empty()
            && self.steps_to_add.is_empty()
            && self.gestures_to_add.is_empty()
    }
}

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    undoable_history: CVec<Plan>,
    ongoing: Plan,
    redoable_history: CVec<Plan>,
}

impl Project {
    pub fn new() -> Project {
        Project {
            undoable_history: CVec::new(),
            ongoing: Plan::new(),
            redoable_history: CVec::new(),
        }
    }

    pub fn from_plan(plan: Plan) -> Project {
        Project {
            undoable_history: vec![plan].into(),
            ongoing: Plan::new(),
            redoable_history: CVec::new(),
        }
    }

    pub fn start_new_step(&mut self) {
        self.undoable_history.push(self.ongoing.clone());
        self.ongoing = Plan::new();
    }

    pub fn set_ongoing_step(&mut self, current_change: Plan) {
        self.ongoing = current_change;
        self.redoable_history.clear();
    }

    pub fn undo(&mut self) {
        if let Some(most_recent_step) = self.undoable_history.pop() {
            self.redoable_history.push(most_recent_step);
            self.ongoing = Plan::new();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next_step_to_redo) = self.redoable_history.pop() {
            self.undoable_history.push(next_step_to_redo);
            self.ongoing = Plan::new();
        }
    }

    pub fn current_history(&self) -> &[Plan] {
        &self.undoable_history
    }

    fn apply_to(&self, base: &PlanHistory) -> PlanHistory {
        base.and_then(&self.undoable_history)
    }

    fn apply_to_with_ongoing(&self, base: &PlanHistory) -> PlanHistory {
        base.and_then(self.undoable_history.iter().chain(Some(&self.ongoing)))
    }

    pub fn as_known_state(&self) -> KnownProjectState {
        KnownProjectState {
            known_last_undoable: COption(self.undoable_history.last().map(|plan| plan.step_id)),
            known_ongoing: COption(Some(self.ongoing.step_id)),
            known_first_redoable: COption(self.redoable_history.first().map(|plan| plan.step_id)),
        }
    }

    pub fn update_for(&self, known_state: &KnownProjectState) -> ProjectUpdate {
        if known_state.known_first_redoable.0.is_none()
            && known_state.known_ongoing.0.is_none()
            && known_state.known_first_redoable.0.is_none()
        {
            ProjectUpdate::ChangedCompletely(self.clone())
        } else if self.undoable_history.last().map(|plan| plan.step_id)
            == known_state.known_last_undoable.0
            && self.redoable_history.first().map(|plan| plan.step_id)
                == known_state.known_first_redoable.0
        {
            if known_state.known_ongoing.0 == Some(self.ongoing.step_id) {
                ProjectUpdate::None
            } else {
                ProjectUpdate::ChangedOngoing(self.ongoing.clone())
            }
        } else {
            ProjectUpdate::ChangedCompletely(self.clone())
        }
    }

    pub fn apply_update(&mut self, update: &ProjectUpdate) {
        match *update {
            ProjectUpdate::ChangedOngoing(ref ongoing) => self.set_ongoing_step(ongoing.clone()),
            ProjectUpdate::ChangedCompletely(ref new_project) => *self = new_project.clone(),
            ProjectUpdate::None => {}
            ProjectUpdate::Removed => {
                panic!("Should handle project removal before applying it to a project")
            }
        }
    }
}

#[derive(Compact, Clone)]
pub struct KnownProjectState {
    known_last_undoable: COption<StepID>,
    known_ongoing: COption<StepID>,
    known_first_redoable: COption<StepID>,
}

impl Default for KnownProjectState {
    fn default() -> KnownProjectState {
        KnownProjectState {
            known_last_undoable: COption(None),
            known_ongoing: COption(None),
            known_first_redoable: COption(None),
        }
    }
}

#[derive(Compact, Clone, Debug)]
pub enum ProjectUpdate {
    None,
    ChangedOngoing(Plan),
    ChangedCompletely(Project),
    Removed,
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub struct PrototypesSpatialCell {
    content_hash: PrototypeID,
    members: CVec<PrototypeID>,
}

const PROTO_SPATIAL_GRID_CELL_SIZE: N = 100.0;

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub struct PrototypesSpatialGrid {
    cells: CHashMap<(i32, i32), PrototypesSpatialCell>,
}

impl PrototypesSpatialGrid {
    pub fn new() -> Self {
        PrototypesSpatialGrid {
            cells: CHashMap::new(),
        }
    }

    pub fn add_protoype(&mut self, proto: &Prototype) {
        let pos = proto.representative_position;
        let grid_coords = (
            (pos.x / PROTO_SPATIAL_GRID_CELL_SIZE) as i32,
            (pos.y / PROTO_SPATIAL_GRID_CELL_SIZE) as i32,
        );
        let found_cell = if let Some(grid_cell) = self.cells.get_mut(grid_coords) {
            match grid_cell.members.binary_search(&proto.id) {
                Err(empty_pos) => grid_cell.members.insert(empty_pos, proto.id),
                Ok(_existing_pos) => println!("Member {:?} already exists in grid!", proto.id),
            }
            grid_cell.content_hash = PrototypeID::from_influences(&grid_cell.members);
            true
        } else {
            false
        };
        if !found_cell {
            self.cells.insert(
                grid_coords,
                PrototypesSpatialCell {
                    content_hash: PrototypeID::from_influences(proto.id),
                    members: Some(proto.id).into_iter().collect(),
                },
            );
        }
    }

    pub fn remove_prototype(&mut self, proto: &Prototype) {
        let pos = proto.representative_position;
        let grid_coords = (
            (pos.x / PROTO_SPATIAL_GRID_CELL_SIZE) as i32,
            (pos.y / PROTO_SPATIAL_GRID_CELL_SIZE) as i32,
        );
        let grid_cell = self
            .cells
            .get_mut(grid_coords)
            .expect("Should have cell for existing proto");
        let idx = grid_cell
            .members
            .binary_search(&proto.id)
            .expect("Should be in cell");
        grid_cell.members.remove(idx);
        grid_cell.members.sort();
        grid_cell.content_hash = PrototypeID::from_influences(&grid_cell.members);
    }

    pub fn difference(&self, other: &Self) -> (Vec<PrototypeID>, Vec<PrototypeID>) {
        let mut only_in_self = Vec::new();
        let mut only_in_other = Vec::new();

        let mut visited_coords = ::std::collections::HashSet::new();

        for (grid_coord, self_cell) in self.cells.pairs() {
            if let Some(other_cell) = other.cells.get(*grid_coord) {
                if self_cell.content_hash != other_cell.content_hash {
                    let mut i = 0;
                    let mut j = 0;

                    while i < self_cell.members.len() && j < other_cell.members.len() {
                        use std::cmp::Ordering;
                        let self_member = self_cell.members[i];
                        let other_member = other_cell.members[j];
                        match self_member.cmp(&other_member) {
                            Ordering::Less => {
                                i += 1;
                                only_in_self.push(self_member);
                            }
                            Ordering::Greater => {
                                j += 1;
                                only_in_other.push(other_member);
                            }
                            Ordering::Equal => {
                                i += 1;
                                j += 1;
                            }
                        }
                    }

                    only_in_self.extend(self_cell.members[i..].iter().cloned());
                    only_in_other.extend(other_cell.members[j..].iter().cloned());
                }
            } else {
                only_in_self.extend(self_cell.members.clone())
            }
            visited_coords.insert(grid_coord);
        }

        for (other_coord, other_cell) in other.cells.pairs() {
            if !visited_coords.contains(&other_coord) {
                only_in_other.extend(other_cell.members.clone())
            }
        }

        (only_in_self, only_in_other)
    }
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub struct PlanResult {
    pub prototypes: CHashMap<PrototypeID, Prototype>,
    pub grid: PrototypesSpatialGrid,
}

impl PlanResult {
    pub fn new() -> PlanResult {
        PlanResult {
            prototypes: CHashMap::new(),
            grid: PrototypesSpatialGrid::new(),
        }
    }

    pub fn actions_to(&self, other: &PlanResult) -> (ActionGroups, CVec<Prototype>) {
        // let mut unmatched_existing_ids: CHashMap<_, ()> =
        //     self.prototypes.keys().map(|key| (*key, ())).collect();
        // let mut to_be_morphed = CVec::new();
        // let mut to_be_constructed = CVec::new();
        // let mut new_prototypes = CVec::new();

        // for (new_prototype_id, new_prototype) in other.prototypes.pairs() {
        //     if unmatched_existing_ids.contains_key(*new_prototype_id) {
        //         // identical prototype, does not need to change at all
        //         unmatched_existing_ids.remove(*new_prototype_id);
        //     } else {
        //         let maybe_morphable_id = unmatched_existing_ids
        //             .keys()
        //             .find(|&other_prototype_id| {
        //                 new_prototype.morphable_from(
        //                     self.prototypes
        //                         .get(*other_prototype_id)
        //                         .expect("should exist since unmatched ids are created from
        // this"),                 )
        //             })
        //             .cloned();
        //         if let Some(morphable_id) = maybe_morphable_id {
        //             unmatched_existing_ids.remove(morphable_id);
        //             to_be_morphed.push(Action::Morph(morphable_id, *new_prototype_id));
        //         } else {
        //             to_be_constructed.push(Action::Construct(*new_prototype_id))
        //         }
        //         new_prototypes.push(new_prototype.clone());
        //     }
        // }

        // let to_be_destructed: CVec<_> = unmatched_existing_ids
        //     .keys()
        //     .map(|unmatched_id| Action::Destruct(*unmatched_id))
        //     .collect();

        let mut to_be_morphed = CVec::new();
        let mut new_prototypes = CVec::new();

        let (mut unmatched_existing_ids, mut unmatched_new_ids) = self.grid.difference(&other.grid);

        unmatched_existing_ids.retain(|unmatched_existing_id| {
            let maybe_morphable_into_id_idx =
                unmatched_new_ids.iter().position(|unmatched_new_id| {
                    other
                        .prototypes
                        .get(*unmatched_new_id)
                        .expect("should have it (new)")
                        .morphable_from(
                            self.prototypes
                                .get(*unmatched_existing_id)
                                .expect("should have it (existing)"),
                        )
                });

            let (remove_both, remove_new_idx) =
                if let Some(morphable_into_id_idx) = maybe_morphable_into_id_idx {
                    let morphable_into = unmatched_new_ids[morphable_into_id_idx];
                    to_be_morphed.push(Action::Morph(*unmatched_existing_id, morphable_into));
                    new_prototypes.push(
                        other
                            .prototypes
                            .get(morphable_into)
                            .expect("should have it (morphable)")
                            .clone(),
                    );
                    (true, morphable_into_id_idx)
                } else {
                    (false, 0)
                };
            if remove_both {
                unmatched_new_ids.remove(remove_new_idx);
                false
            } else {
                true
            }
        });

        let to_be_constructed: CVec<_> = unmatched_new_ids
            .into_iter()
            .map(|id| {
                new_prototypes.push(
                    other
                        .prototypes
                        .get(id)
                        .expect("should have it (constructable)")
                        .clone(),
                );
                Action::Construct(id)
            })
            .collect();
        let to_be_destructed: CVec<_> = unmatched_existing_ids
            .into_iter()
            .map(Action::Destruct)
            .collect();

        // println!(
        //     "Actions to: C {}, M {}, D {}",
        //     to_be_constructed.len(),
        //     to_be_morphed.len(),
        //     to_be_destructed.len()
        // );

        (
            ActionGroups(
                vec![
                    IndependentActions(to_be_destructed),
                    IndependentActions(to_be_morphed),
                    IndependentActions(to_be_constructed),
                ]
                .into(),
            ),
            new_prototypes,
        )
    }

    pub fn as_known_state(&self) -> KnownPlanResultState {
        KnownPlanResultState {
            known_prototype_ids: self.grid.clone(),
        }
    }

    pub fn update_for(&self, known_state: &KnownPlanResultState) -> PlanResultUpdate {
        // let prototypes_to_drop = known_state
        //     .known_prototype_ids
        //     .keys()
        //     .filter_map(|known_prototype_id| {
        //         if self.prototypes.contains_key(*known_prototype_id) {
        //             None
        //         } else {
        //             Some(*known_prototype_id)
        //         }
        //     })
        //     .collect();

        // let new_prototypes = self
        //     .prototypes
        //     .values()
        //     .filter_map(|prototype| {
        //         if known_state.known_prototype_ids.contains_key(prototype.id) {
        //             None
        //         } else {
        //             Some(prototype.clone())
        //         }
        //     })
        //     .collect();

        let (only_in_self, only_in_other) = self.grid.difference(&known_state.known_prototype_ids);

        // println!(
        //     "Update to: Dr {}, N {}",
        //     only_in_other.len(),
        //     only_in_self.len(),
        // );

        PlanResultUpdate {
            prototypes_to_drop: only_in_other.into(),
            new_prototypes: only_in_self
                .into_iter()
                .map(|id| self.prototypes.get(id).expect("Should have it").clone())
                .collect(),
        }
    }

    pub fn apply_update(&mut self, update: &PlanResultUpdate) {
        for prototype_to_drop_id in &update.prototypes_to_drop {
            let proto = self
                .prototypes
                .remove(*prototype_to_drop_id)
                .expect("Should have had proto to remove");
            self.grid.remove_prototype(&proto);
        }

        for new_prototype in &update.new_prototypes {
            self.grid.add_protoype(new_prototype);
            self.prototypes
                .insert(new_prototype.id, new_prototype.clone());
        }
    }
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub struct Prototype {
    pub id: PrototypeID,
    pub kind: PrototypeKind,
    pub representative_position: P2,
}

impl Prototype {
    pub fn new_with_influences<H: Hash>(
        influences: H,
        kind: PrototypeKind,
        representative_position: P2,
    ) -> Prototype {
        Prototype {
            id: PrototypeID::from_influences(influences),
            kind,
            representative_position,
        }
    }
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub enum PrototypeKind {
    Road(RoadPrototype),
    Lot(LotPrototype),
    Plant(PlantPrototype),
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub struct PrototypeID(u64);

impl PrototypeID {
    pub fn from_influences<H: Hash>(influences: H) -> PrototypeID {
        PrototypeID(seed(influences).next_u64())
    }

    pub fn add_influences<H: Hash>(self, influences: H) -> PrototypeID {
        PrototypeID(seed((self.0, influences)).next_u64())
    }
}

#[derive(Compact, Clone, Debug)]
pub struct KnownPlanResultState {
    known_prototype_ids: PrototypesSpatialGrid,
}

#[derive(Compact, Clone, Debug)]
pub struct PlanResultUpdate {
    pub prototypes_to_drop: CVec<PrototypeID>,
    pub new_prototypes: CVec<Prototype>,
}

#[derive(Compact, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Action {
    Construct(PrototypeID),
    Morph(PrototypeID, PrototypeID),
    Destruct(PrototypeID),
}

impl Action {
    pub fn is_construct(&self) -> bool {
        if let Action::Construct(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_morph(&self) -> bool {
        if let Action::Morph(..) = self {
            true
        } else {
            false
        }
    }

    pub fn is_destruct(&self) -> bool {
        if let Action::Destruct(_) = self {
            true
        } else {
            false
        }
    }
}

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub struct IndependentActions(pub CVec<Action>);

impl IndependentActions {
    pub fn new() -> IndependentActions {
        IndependentActions(CVec::new())
    }

    pub fn as_known_state(&self) -> Self {
        self.clone()
    }
}

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub struct ActionGroups(pub CVec<IndependentActions>);

impl ActionGroups {
    pub fn new() -> ActionGroups {
        ActionGroups(CVec::new())
    }

    pub fn corresponding_action(&self, prototype_id: PrototypeID) -> Option<Action> {
        self.0
            .iter()
            .filter_map(|action_group| {
                action_group
                    .0
                    .iter()
                    .filter_map(|action| match *action {
                        Action::Construct(constructed_prototype_id) => {
                            if constructed_prototype_id == prototype_id {
                                Some(action.clone())
                            } else {
                                None
                            }
                        }
                        Action::Morph(_, new_prototype_id) => {
                            if new_prototype_id == prototype_id {
                                Some(action.clone())
                            } else {
                                None
                            }
                        }
                        Action::Destruct(destructed_prototype_id) => {
                            if destructed_prototype_id == prototype_id {
                                Some(action.clone())
                            } else {
                                None
                            }
                        }
                    })
                    .next()
            })
            .next()
    }
}

impl PlanHistory {
    pub fn calculate_result(&self) -> Result<PlanResult, AreaError> {
        let mut result = PlanResult::new();

        for prototype_fn in &[
            ::transport::transport_planning::calculate_prototypes,
            ::land_use::zone_planning::calculate_prototypes,
            ::environment::vegetation::calculate_prototypes,
        ] {
            let new_prototypes = prototype_fn(self, &result)?;

            for (id, prototype) in new_prototypes
                .into_iter()
                .map(|prototype| (prototype.id, prototype))
            {
                result.grid.add_protoype(&prototype);
                result.prototypes.insert(id, prototype);
            }
        }

        Ok(result)
    }
}

use self::interaction::PlanManagerUIState;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct ProjectID(pub Uuid);

impl ProjectID {
    pub fn new() -> ProjectID {
        ProjectID(uuid())
    }
}

// #[derive(Compact, Clone)]
#[derive(Clone)]
pub struct PlanManager {
    id: PlanManagerID,
    master_plan: PlanHistory,
    master_result: PlanResult,
    projects: CHashMap<ProjectID, Project>,
    implemented_projects: CHashMap<ProjectID, Project>,
    ui_state: CHashMap<MachineID, PlanManagerUIState>,
}

mod compact_workaround;

impl PlanManager {
    pub fn spawn(id: PlanManagerID, initial_project_id: ProjectID, _: &mut World) -> PlanManager {
        PlanManager {
            id,
            master_plan: PlanHistory::new(),
            master_result: PlanResult::new(),
            projects: Some((initial_project_id, Project::new()))
                .into_iter()
                .collect(),
            implemented_projects: CHashMap::new(),
            ui_state: CHashMap::new(),
        }
    }

    pub fn get_current_version_of(&self, gesture_id: GestureID, project_id: ProjectID) -> &Gesture {
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

    pub fn implement(&mut self, project_id: ProjectID, world: &mut World) {
        let project = self
            .projects
            .remove(project_id)
            .expect("Project should exist");

        self.master_plan = project.apply_to(&self.master_plan);

        match self.master_plan.calculate_result() {
            Ok(result) => {
                let (actions, new_prototypes) = self.master_result.actions_to(&result);
                ConstructionID::global_first(world).implement(actions, new_prototypes, world);
                self.implemented_projects.insert(project_id, project);
                self.master_result = result;

                let potentially_affected_ui_states = self
                    .ui_state
                    .pairs()
                    .map(|(machine, state)| (*machine, state.current_project))
                    .collect::<Vec<_>>();

                for (machine, current_project) in potentially_affected_ui_states {
                    if current_project == project_id {
                        let new_project_id = ProjectID::new();

                        self.projects.insert(new_project_id, Project::new());

                        self.switch_to(machine, new_project_id, world);
                    }
                }

                let all_project_ids = self.projects.keys().cloned().collect::<Vec<_>>();
                for old_project_id in all_project_ids {
                    if old_project_id != project_id {
                        self.clear_previews(old_project_id);
                    }
                }
            }
            Err(err) => match err {
                ::descartes::AreaError::LeftOver(string) => {
                    error(
                        LOG_T,
                        format!("Implement Plan Error: {}", string),
                        self.id,
                        world,
                    );
                }
                _ => {
                    error(
                        LOG_T,
                        format!("Implement Plan Error: {:?}", err),
                        self.id,
                        world,
                    );
                }
            },
        }
    }

    pub fn implement_artificial_project(
        &mut self,
        project: &Project,
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

pub fn setup(system: &mut ActorSystem) {
    system.register::<PlanManager>();
    auto_setup(system);
    interaction::setup(system);
    ui::auto_setup(system);
}

pub fn spawn(world: &mut World) -> PlanManagerID {
    let initial_project_id = ProjectID::new();
    let plan_manager = PlanManagerID::spawn(initial_project_id, world);
    plan_manager.switch_to(MachineID(0), initial_project_id, world);
    plan_manager
}

pub mod kay_auto;
pub use self::kay_auto::*;
