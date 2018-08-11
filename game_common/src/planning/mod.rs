#![cfg_attr(feature = "cargo-clippy", allow(new_without_default_derive))]
#![cfg_attr(feature = "cargo-clippy", allow(new_without_default))]
use kay::{World, MachineID, ActorSystem, Actor};
use compact::{CVec, CHashMap};
use descartes::{P2, AreaError};
use uuid::Uuid;
use util::random::{seed, Rng};
use std::hash::Hash;

use transport::transport_planning::{RoadIntent, RoadPrototype};
use land_use::zone_planning::{ZoneIntent, BuildingIntent, LotPrototype};
use construction::Construction;

pub mod rendering;
pub mod interaction;

// idea for improvement:
// - everything (Gestures, Prototypes) immutable (helps caching)
// - everything separated by staggered grid (to save work)

#[derive(Compact, Clone, Serialize, Deserialize)]
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
}

#[derive(Compact, Clone, Serialize, Deserialize)]
pub enum GestureIntent {
    Road(RoadIntent),
    Zone(ZoneIntent),
    Building(BuildingIntent),
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct GestureID(pub Uuid);

impl GestureID {
    pub fn new() -> GestureID {
        GestureID(Uuid::new_v4())
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StepID(pub Uuid);

#[derive(Compact, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub step_id: StepID,
    pub gestures: CHashMap<GestureID, Gesture>,
}

impl Plan {
    pub fn new() -> Plan {
        Plan {
            step_id: StepID(Uuid::new_v4()),
            gestures: CHashMap::new(),
        }
    }

    pub fn from_gestures<I: IntoIterator<Item = (GestureID, Gesture)>>(gestures: I) -> Plan {
        Plan {
            step_id: StepID(Uuid::new_v4()),
            gestures: gestures.into_iter().collect(),
        }
    }
}

#[derive(Compact, Clone, Serialize, Deserialize)]
pub struct VersionedGesture(pub Gesture, pub StepID);

#[derive(Compact, Clone, Serialize, Deserialize)]
pub struct PlanHistory {
    pub gestures: CHashMap<GestureID, VersionedGesture>,
    steps: CVec<StepID>,
}

impl PlanHistory {
    pub fn new() -> PlanHistory {
        PlanHistory {
            gestures: CHashMap::new(),
            steps: vec![StepID(Uuid::new_v4())].into(),
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
}

// TODO: when applied, proposals can be flattened into the last
// version of each gesture and all intermediate gestures can be completely removed
#[derive(Compact, Clone, Serialize, Deserialize)]
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
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub struct PlanResult {
    pub prototypes: CHashMap<PrototypeID, Prototype>,
}

use construction::Action;

impl PlanResult {
    pub fn new() -> PlanResult {
        PlanResult {
            prototypes: CHashMap::new(),
        }
    }

    pub fn actions_to(&self, other: &PlanResult) -> CVec<CVec<Action>> {
        let mut unmatched_existing = self.prototypes.clone();
        let mut to_be_morphed = CVec::new();
        let mut to_be_constructed = CVec::new();

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
                    to_be_morphed.push(Action::Morph(
                        morphable_id,
                        *new_prototype_id,
                        new_prototype.clone(),
                    ));
                } else {
                    to_be_constructed
                        .push(Action::Construct(*new_prototype_id, new_prototype.clone()))
                }
            }
        }

        let to_be_destructed = unmatched_existing
            .keys()
            .map(|unmatched_id| Action::Destruct(*unmatched_id))
            .collect();

        vec![to_be_destructed, to_be_morphed, to_be_constructed].into()
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
        ProposalID(Uuid::new_v4())
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
                let actions = self.master_result.actions_to(&result);
                Construction::global_first(world).implement(actions, world);
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
        based_on: StepID,
        world: &mut World,
    ) {
        if based_on == self.master_plan.latest_step_id() {
            let proposal_id = ProposalID::new();
            self.proposals.insert(proposal_id, proposal.clone());
            self.implement(proposal_id, world);
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
