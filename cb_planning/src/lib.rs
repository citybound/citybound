// TODO: remove once https://github.com/rust-lang/rust/issues/54726 is resolved
#![feature(custom_inner_attributes)]
#![allow(clippy::new_without_default)]
extern crate kay;
extern crate compact;
#[macro_use]
extern crate compact_macros;
#[macro_use]
extern crate serde_derive;
extern crate descartes;
extern crate cb_util;
extern crate cb_time;

use compact::{CVec, COption, CHashMap, Compact};
use descartes::{N, P2, AreaError};
use cb_util::random::{seed, RngCore, Uuid, uuid};
use std::hash::Hash;

pub mod construction;
use construction::{PrototypeKind, GestureIntent};
pub mod plan_manager;

// idea for improvement:
// - everything (Gestures, Prototypes) immutable (helps caching)
// - everything separated by staggered grid (to save work)

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub struct Gesture<GI: GestureIntent> {
    pub intent: GI,
}

impl<GI: GestureIntent> Gesture<GI> {
    pub fn new(intent: GI) -> Self {
        Gesture { intent }
    }
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
pub struct Plan<GI: GestureIntent> {
    pub step_id: StepID,
    pub gestures: CHashMap<GestureID, Gesture<GI>>,
}

impl<GI: GestureIntent> Plan<GI> {
    pub fn new() -> Plan<GI> {
        Plan {
            step_id: StepID(uuid()),
            gestures: CHashMap::new(),
        }
    }

    pub fn from_gestures<I: IntoIterator<Item = (GestureID, Gesture<GI>)>>(
        gestures: I,
    ) -> Plan<GI> {
        Plan {
            step_id: StepID(uuid()),
            gestures: gestures.into_iter().collect(),
        }
    }
}

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub struct VersionedGesture<GI: GestureIntent>(pub Gesture<GI>, pub StepID);

#[derive(Compact, Clone, Serialize, Deserialize)]
pub struct PlanHistory<GI: GestureIntent> {
    pub gestures: CHashMap<GestureID, VersionedGesture<GI>>,
    pub steps: CVec<StepID>,
}

impl<GI: GestureIntent> PlanHistory<GI> {
    pub fn new() -> PlanHistory<GI> {
        PlanHistory {
            gestures: CHashMap::new(),
            steps: vec![StepID(uuid())].into(),
        }
    }

    pub fn and_then<'a, I: IntoIterator<Item = &'a Plan<GI>>>(&self, plans: I) -> PlanHistory<GI> {
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

    pub fn update_for(&self, known_state: &KnownHistoryState) -> PlanHistoryUpdate<GI> {
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

    pub fn apply_update(&mut self, update: &PlanHistoryUpdate<GI>) {
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
pub struct PlanHistoryUpdate<GI: GestureIntent> {
    steps_to_drop: CVec<StepID>,
    steps_to_add: CVec<StepID>,
    gestures_to_add: CHashMap<GestureID, VersionedGesture<GI>>,
}

impl<GI: GestureIntent> PlanHistoryUpdate<GI> {
    pub fn is_empty(&self) -> bool {
        self.steps_to_drop.is_empty()
            && self.steps_to_add.is_empty()
            && self.gestures_to_add.is_empty()
    }
}

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub struct Project<GI: GestureIntent> {
    undoable_history: CVec<Plan<GI>>,
    ongoing: Plan<GI>,
    redoable_history: CVec<Plan<GI>>,
}

impl<GI: GestureIntent + 'static> Project<GI> {
    pub fn new() -> Project<GI> {
        Project {
            undoable_history: CVec::new(),
            ongoing: Plan::new(),
            redoable_history: CVec::new(),
        }
    }

    pub fn from_plan(plan: Plan<GI>) -> Project<GI> {
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

    pub fn set_ongoing_step(&mut self, current_change: Plan<GI>) {
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

    pub fn current_history(&self) -> &[Plan<GI>] {
        &self.undoable_history
    }

    fn apply_to(&self, base: &PlanHistory<GI>) -> PlanHistory<GI> {
        base.and_then(&self.undoable_history)
    }

    fn apply_to_with_ongoing(&self, base: &PlanHistory<GI>) -> PlanHistory<GI> {
        base.and_then(self.undoable_history.iter().chain(Some(&self.ongoing)))
    }

    pub fn as_known_state(&self) -> KnownProjectState {
        KnownProjectState {
            known_last_undoable: COption(self.undoable_history.last().map(|plan| plan.step_id)),
            known_ongoing: COption(Some(self.ongoing.step_id)),
            known_first_redoable: COption(self.redoable_history.first().map(|plan| plan.step_id)),
        }
    }

    pub fn update_for(&self, known_state: &KnownProjectState) -> ProjectUpdate<GI> {
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

    pub fn apply_update(&mut self, update: &ProjectUpdate<GI>) {
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
pub enum ProjectUpdate<GI: GestureIntent> {
    None,
    ChangedOngoing(Plan<GI>),
    ChangedCompletely(Project<GI>),
    Removed,
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub struct PrototypesSpatialCell {
    content_hash: PrototypeID,
    members: CVec<PrototypeID>,
}

const PROTO_SPATIAL_GRID_CELL_SIZE: N = 100.0;

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub struct PrototypesSpatialGrid<PK: PrototypeKind> {
    cells: CHashMap<(i32, i32), PrototypesSpatialCell>,
    marker: ::std::marker::PhantomData<PK>,
}

impl<PK: PrototypeKind> PrototypesSpatialGrid<PK> {
    pub fn new() -> Self {
        PrototypesSpatialGrid {
            cells: CHashMap::new(),
            marker: ::std::marker::PhantomData,
        }
    }

    pub fn add_protoype(&mut self, proto: &Prototype<PK>) {
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

    pub fn remove_prototype(&mut self, proto: &Prototype<PK>) {
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
pub struct PlanResult<PK: PrototypeKind> {
    pub prototypes: CHashMap<PrototypeID, Prototype<PK>>,
    pub grid: PrototypesSpatialGrid<PK>,
}

impl<PK: PrototypeKind> PlanResult<PK> {
    pub fn new() -> PlanResult<PK> {
        PlanResult {
            prototypes: CHashMap::new(),
            grid: PrototypesSpatialGrid::new(),
        }
    }

    pub fn actions_to(&self, other: &PlanResult<PK>) -> (ActionGroups, CVec<Prototype<PK>>) {
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

    pub fn as_known_state(&self) -> KnownPlanResultState<PK> {
        KnownPlanResultState {
            known_prototype_ids: self.grid.clone(),
        }
    }

    pub fn update_for(&self, known_state: &KnownPlanResultState<PK>) -> PlanResultUpdate<PK> {
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

    pub fn apply_update(&mut self, update: &PlanResultUpdate<PK>) {
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
pub struct Prototype<PK: PrototypeKind> {
    pub id: PrototypeID,
    pub kind: PK,
    pub representative_position: P2,
}

impl<PK: PrototypeKind> Prototype<PK> {
    pub fn new_with_influences<H: Hash>(
        influences: H,
        kind: PK,
        representative_position: P2,
    ) -> Prototype<PK> {
        Prototype {
            id: PrototypeID::from_influences(influences),
            kind,
            representative_position,
        }
    }
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
pub struct KnownPlanResultState<PK: PrototypeKind> {
    known_prototype_ids: PrototypesSpatialGrid<PK>,
}

#[derive(Compact, Clone, Debug)]
pub struct PlanResultUpdate<PK: PrototypeKind> {
    pub prototypes_to_drop: CVec<PrototypeID>,
    pub new_prototypes: CVec<Prototype<PK>>,
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

#[allow(type_alias_bounds)]
pub type PlanningStepFn<PL: PlanningLogic> =
    fn(
        &PlanHistory<PL::GestureIntent>,
        &PlanResult<PL::PrototypeKind>,
    ) -> Result<Vec<Prototype<PL::PrototypeKind>>, AreaError>;

pub trait PlanningLogic: Compact + 'static {
    type GestureIntent: GestureIntent;
    type PrototypeKind: PrototypeKind;

    fn planning_step_functions() -> &'static [PlanningStepFn<Self>];
    fn calculate_result(
        history: &PlanHistory<Self::GestureIntent>,
    ) -> Result<PlanResult<Self::PrototypeKind>, AreaError> {
        let mut result = PlanResult::new();

        for prototype_fn in Self::planning_step_functions() {
            let new_prototypes = prototype_fn(history, &result)?;

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
