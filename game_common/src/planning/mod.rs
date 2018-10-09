#![cfg_attr(feature = "cargo-clippy", allow(new_without_default_derive))]
#![cfg_attr(feature = "cargo-clippy", allow(new_without_default))]
use kay::{World, MachineID, ActorSystem, Actor};
use compact::{CVec, COption, CHashMap};
use descartes::{P2, AreaError};
use util::random::{seed, RngCore, Uuid, uuid};
use std::hash::Hash;

use transport::transport_planning::{RoadIntent, RoadPrototype};
use land_use::zone_planning::{ZoneIntent, BuildingIntent, LotPrototype};
use construction::Construction;

pub mod rendering;
pub mod interaction;

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
                }).collect(),
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
            }).collect::<Vec<_>>();

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
pub struct Proposal {
    undoable_history: CVec<Plan>,
    ongoing: Plan,
    redoable_history: CVec<Plan>,
}

impl Proposal {
    pub fn new() -> Proposal {
        Proposal {
            undoable_history: CVec::new(),
            ongoing: Plan::new(),
            redoable_history: CVec::new(),
        }
    }

    pub fn from_plan(plan: Plan) -> Proposal {
        Proposal {
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

    pub fn as_known_state(&self) -> KnownProposalState {
        KnownProposalState {
            known_last_undoable: COption(self.undoable_history.last().map(|plan| plan.step_id)),
            known_ongoing: COption(Some(self.ongoing.step_id)),
            known_first_redoable: COption(self.redoable_history.first().map(|plan| plan.step_id)),
        }
    }

    pub fn update_for(&self, known_state: &KnownProposalState) -> ProposalUpdate {
        if known_state.known_first_redoable.0.is_none()
            && known_state.known_ongoing.0.is_none()
            && known_state.known_first_redoable.0.is_none()
        {
            ProposalUpdate::ChangedCompletely(self.clone())
        } else if self.undoable_history.last().map(|plan| plan.step_id)
            == known_state.known_last_undoable.0
            && self.redoable_history.first().map(|plan| plan.step_id)
                == known_state.known_first_redoable.0
        {
            if known_state.known_ongoing.0 == Some(self.ongoing.step_id) {
                ProposalUpdate::None
            } else {
                ProposalUpdate::ChangedOngoing(self.ongoing.clone())
            }
        } else {
            ProposalUpdate::ChangedCompletely(self.clone())
        }
    }

    pub fn apply_update(&mut self, update: &ProposalUpdate) {
        match *update {
            ProposalUpdate::ChangedOngoing(ref ongoing) => self.set_ongoing_step(ongoing.clone()),
            ProposalUpdate::ChangedCompletely(ref new_proposal) => *self = new_proposal.clone(),
            ProposalUpdate::None => {}
            ProposalUpdate::Removed => {
                panic!("Should handle proposal removal before applying it to a proposal")
            }
        }
    }
}

#[derive(Compact, Clone)]
pub struct KnownProposalState {
    known_last_undoable: COption<StepID>,
    known_ongoing: COption<StepID>,
    known_first_redoable: COption<StepID>,
}

impl Default for KnownProposalState {
    fn default() -> KnownProposalState {
        KnownProposalState {
            known_last_undoable: COption(None),
            known_ongoing: COption(None),
            known_first_redoable: COption(None),
        }
    }
}

#[derive(Compact, Clone, Debug)]
pub enum ProposalUpdate {
    None,
    ChangedOngoing(Plan),
    ChangedCompletely(Proposal),
    Removed,
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub struct PlanResult {
    pub prototypes: CHashMap<PrototypeID, Prototype>,
}

impl PlanResult {
    pub fn new() -> PlanResult {
        PlanResult {
            prototypes: CHashMap::new(),
        }
    }

    pub fn actions_to(&self, other: &PlanResult) -> (ActionGroups, CVec<Prototype>) {
        let mut unmatched_existing = self.prototypes.clone();
        let mut to_be_morphed = CVec::new();
        let mut to_be_constructed = CVec::new();
        let mut new_prototypes = CVec::new();

        for (new_prototype_id, new_prototype) in other.prototypes.pairs() {
            if unmatched_existing.contains_key(*new_prototype_id) {
                // identical prototype, does not need to change at all
                unmatched_existing.remove(*new_prototype_id);
            } else {
                let maybe_morphable_id = unmatched_existing
                    .pairs()
                    .find(|&(_, other_prototype)| new_prototype.morphable_from(other_prototype))
                    .map(|(id, _)| *id);
                if let Some(morphable_id) = maybe_morphable_id {
                    unmatched_existing.remove(morphable_id);
                    to_be_morphed.push(Action::Morph(morphable_id, *new_prototype_id));
                } else {
                    to_be_constructed.push(Action::Construct(*new_prototype_id))
                }
                new_prototypes.push(new_prototype.clone());
            }
        }

        let to_be_destructed = unmatched_existing
            .keys()
            .map(|unmatched_id| Action::Destruct(*unmatched_id))
            .collect();

        (
            ActionGroups(
                vec![
                    IndependentActions(to_be_destructed),
                    IndependentActions(to_be_morphed),
                    IndependentActions(to_be_constructed),
                ].into(),
            ),
            new_prototypes,
        )
    }

    pub fn as_known_state(&self) -> KnownPlanResultState {
        KnownPlanResultState {
            known_prototype_ids: self.prototypes.keys().cloned().collect(),
        }
    }

    pub fn update_for(&self, known_state: &KnownPlanResultState) -> PlanResultUpdate {
        let prototypes_to_drop = known_state
            .known_prototype_ids
            .iter()
            .filter_map(|known_prototype_id| {
                if self.prototypes.contains_key(*known_prototype_id) {
                    None
                } else {
                    Some(*known_prototype_id)
                }
            }).collect();

        let new_prototypes = self
            .prototypes
            .values()
            .filter_map(|prototype| {
                if known_state.known_prototype_ids.contains(&prototype.id) {
                    None
                } else {
                    Some(prototype.clone())
                }
            }).collect();

        PlanResultUpdate {
            prototypes_to_drop,
            new_prototypes,
        }
    }

    pub fn apply_update(&mut self, update: &PlanResultUpdate) {
        for prototype_to_drop in &update.prototypes_to_drop {
            self.prototypes.remove(*prototype_to_drop);
        }

        for new_prototype in &update.new_prototypes {
            self.prototypes
                .insert(new_prototype.id, new_prototype.clone());
        }
    }
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub struct Prototype {
    pub id: PrototypeID,
    pub kind: PrototypeKind,
}

impl Prototype {
    pub fn new_with_influences<H: Hash>(influences: H, kind: PrototypeKind) -> Prototype {
        Prototype {
            id: PrototypeID::from_influences(influences),
            kind,
        }
    }
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub enum PrototypeKind {
    Road(RoadPrototype),
    Lot(LotPrototype),
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct PrototypeID(u64);

impl PrototypeID {
    pub fn from_influences<H: Hash>(influences: H) -> PrototypeID {
        PrototypeID(seed(influences).next_u64())
    }

    pub fn add_influences<H: Hash>(&self, influences: H) -> PrototypeID {
        PrototypeID(seed((self.0, influences)).next_u64())
    }
}

#[derive(Compact, Clone, Debug)]
pub struct KnownPlanResultState {
    known_prototype_ids: CVec<PrototypeID>,
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
                    }).next()
            }).next()
    }
}

impl PlanHistory {
    pub fn calculate_result(&self) -> Result<PlanResult, AreaError> {
        let mut result = PlanResult {
            prototypes: CHashMap::new(),
        };

        for prototype_fn in &[
            ::transport::transport_planning::calculate_prototypes,
            ::land_use::zone_planning::calculate_prototypes,
        ] {
            let new_prototypes = prototype_fn(self, &result)?;

            for (id, prototype) in new_prototypes
                .into_iter()
                .map(|prototype| (prototype.id, prototype))
            {
                result.prototypes.insert(id, prototype);
            }
        }

        Ok(result)
    }
}

use self::interaction::PlanManagerUIState;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct ProposalID(pub Uuid);

impl ProposalID {
    pub fn new() -> ProposalID {
        ProposalID(uuid())
    }
}

#[derive(Compact, Clone)]
pub struct PlanManager {
    id: PlanManagerID,
    master_plan: PlanHistory,
    master_result: PlanResult,
    proposals: CHashMap<ProposalID, Proposal>,
    implemented_proposals: CHashMap<ProposalID, Proposal>,
    ui_state: CHashMap<MachineID, PlanManagerUIState>,
}

impl PlanManager {
    pub fn spawn(id: PlanManagerID, initial_proposal_id: ProposalID, _: &mut World) -> PlanManager {
        PlanManager {
            id,
            master_plan: PlanHistory::new(),
            master_result: PlanResult::new(),
            proposals: Some((initial_proposal_id, Proposal::new()))
                .into_iter()
                .collect(),
            implemented_proposals: CHashMap::new(),
            ui_state: CHashMap::new(),
        }
    }

    pub fn get_current_version_of(
        &self,
        gesture_id: GestureID,
        proposal_id: ProposalID,
    ) -> &Gesture {
        self.proposals
            .get(proposal_id)
            .expect("Expected proposal to exist")
            .current_history()
            .iter()
            .rfold(None, |found, step| {
                found.or_else(|| step.gestures.get(gesture_id))
            }).into_iter()
            .chain(
                self.master_plan
                    .gestures
                    .get(gesture_id)
                    .map(|VersionedGesture(ref g, _)| g),
            ).next()
            .expect("Expected gesture (that point should be added to) to exist!")
    }

    pub fn implement(&mut self, proposal_id: ProposalID, world: &mut World) {
        let proposal = self
            .proposals
            .remove(proposal_id)
            .expect("Proposal should exist");

        self.master_plan = proposal.apply_to(&self.master_plan);

        match self.master_plan.calculate_result() {
            Ok(result) => {
                let (actions, new_prototypes) = self.master_result.actions_to(&result);
                Construction::global_first(world).implement(actions, new_prototypes, world);
                self.implemented_proposals.insert(proposal_id, proposal);
                self.master_result = result;

                let potentially_affected_ui_states = self
                    .ui_state
                    .pairs()
                    .map(|(machine, state)| (*machine, state.current_proposal))
                    .collect::<Vec<_>>();

                for (machine, current_proposal) in potentially_affected_ui_states {
                    if current_proposal == proposal_id {
                        let new_proposal_id = ProposalID::new();

                        self.proposals.insert(new_proposal_id, Proposal::new());

                        self.switch_to(machine, new_proposal_id, world);
                    }
                }

                let all_proposal_ids = self.proposals.keys().cloned().collect::<Vec<_>>();
                for old_proposal_id in all_proposal_ids {
                    if old_proposal_id != proposal_id {
                        self.clear_previews(old_proposal_id);
                    }
                }
            }
            Err(err) => match err {
                ::descartes::AreaError::LeftOver(string) => {
                    println!("Implement Plan Error: {}", string);
                }
                _ => {
                    println!("Implement Plan Error: {:?}", err);
                }
            },
        }
    }

    pub fn implement_artificial_proposal(
        &mut self,
        proposal: &Proposal,
        based_on: &CVec<PrototypeID>,
        world: &mut World,
    ) {
        if based_on
            .iter()
            .all(|prototype_id| self.master_result.prototypes.contains_key(*prototype_id))
        {
            let proposal_id = ProposalID::new();
            self.proposals.insert(proposal_id, proposal.clone());
            self.implement(proposal_id, world);
        } else {
            println!("Tried to implement artificial proposal based on outdated prototypes");
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<PlanManager>();
    auto_setup(system);
    rendering::auto_setup(system);
    interaction::setup(system);
}

pub fn spawn(world: &mut World) -> PlanManagerID {
    let initial_proposal_id = ProposalID::new();
    let plan_manager = PlanManagerID::spawn(initial_proposal_id, world);
    plan_manager.switch_to(MachineID(0), initial_proposal_id, world);
    plan_manager
}

pub mod kay_auto;
pub use self::kay_auto::*;
