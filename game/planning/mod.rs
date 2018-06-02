use kay::{World, MachineID, ActorSystem, Actor};
use compact::{CVec, CHashMap};
use descartes::P2;
use stagemaster::UserInterfaceID;
use uuid::Uuid;

use transport::transport_planning::{RoadIntent, RoadPrototype};
use land_use::zone_planning::{ZoneIntent, BuildingIntent, LotPrototype};
use construction::Construction;

pub mod rendering;
pub mod interaction;

#[derive(Compact, Clone)]
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

#[derive(Compact, Clone)]
pub enum GestureIntent {
    Road(RoadIntent),
    Zone(ZoneIntent),
    Building(BuildingIntent),
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct GestureID(Uuid);

impl GestureID {
    pub fn new() -> GestureID {
        GestureID(Uuid::new_v4())
    }
}

#[derive(Compact, Clone, Default)]
pub struct Plan {
    pub gestures: CHashMap<GestureID, Gesture>,
}

impl Plan {
    pub fn merge<'a, I: IntoIterator<Item = &'a Plan>>(&self, others: I) -> Plan {
        let mut new_plan = self.clone();
        for other in others {
            for (key, value) in other.gestures.pairs() {
                new_plan.gestures.insert(*key, value.clone());
            }
        }
        new_plan
    }
}

// TODO: when applied, proposals can be flattened into the last
// version of each gesture and all intermediate gestures can be completely removed
#[derive(Compact, Clone, Default)]
pub struct Proposal {
    undoable_history: CVec<Plan>,
    ongoing: Plan,
    redoable_history: CVec<Plan>,
}

impl Proposal {
    pub fn new() -> Proposal {
        Proposal::default()
    }

    pub fn from_plan(plan: Plan) -> Proposal {
        Proposal {
            undoable_history: vec![plan].into(),
            ongoing: Plan::default(),
            redoable_history: CVec::new(),
        }
    }

    pub fn start_new_step(&mut self) {
        self.undoable_history.push(self.ongoing.clone());
        self.ongoing = Plan::default();
    }

    pub fn set_ongoing_step(&mut self, current_change: Plan) {
        self.ongoing = current_change;
        self.redoable_history.clear();
    }

    pub fn undo(&mut self) {
        if let Some(most_recent_step) = self.undoable_history.pop() {
            self.redoable_history.push(most_recent_step);
            self.ongoing = Plan::default();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next_step_to_redo) = self.redoable_history.pop() {
            self.undoable_history.push(next_step_to_redo);
            self.ongoing = Plan::default();
        }
    }

    pub fn current_history(&self) -> &[Plan] {
        &self.undoable_history
    }

    fn apply_to(&self, base: &Plan) -> Plan {
        base.merge(&self.undoable_history)
    }

    fn apply_to_with_ongoing(&self, base: &Plan) -> Plan {
        base.merge(self.undoable_history.iter().chain(Some(&self.ongoing)))
    }
}

#[derive(Compact, Clone)]
pub struct PlanResult {
    pub prototypes: CHashMap<PrototypeID, Prototype>,
}

#[derive(Compact, Clone)]
pub enum Prototype {
    Road(RoadPrototype),
    Lot(LotPrototype),
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct PrototypeID(Uuid);

impl PrototypeID {
    pub fn new() -> PrototypeID {
        PrototypeID(Uuid::new_v4())
    }
}

impl Plan {
    pub fn calculate_result(&self, based_on: Version) -> PlanResult {
        let mut result = PlanResult {
            prototypes: CHashMap::new(),
        };

        for prototype_fn in &[
            ::transport::transport_planning::calculate_prototypes,
            ::land_use::zone_planning::calculate_prototypes,
        ] {
            let new_prototypes = prototype_fn(self, &result, based_on);

            for (id, prototype) in new_prototypes
                .into_iter()
                .map(|prototype| (PrototypeID::new(), prototype))
            {
                result.prototypes.insert(id, prototype);
            }
        }

        result
    }
}

use self::interaction::PlanManagerUIState;

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct ProposalID(Uuid);

impl ProposalID {
    pub fn new() -> ProposalID {
        ProposalID(Uuid::new_v4())
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Version(ProposalID);

#[derive(Compact, Clone)]
pub struct PlanManager {
    id: PlanManagerID,
    master_plan: Plan,
    master_version: Version,
    proposals: CHashMap<ProposalID, Proposal>,
    implemented_proposals: CHashMap<ProposalID, Proposal>,
    ui_state: CHashMap<MachineID, PlanManagerUIState>,
}

impl PlanManager {
    pub fn spawn(id: PlanManagerID, initial_proposal_id: ProposalID, _: &mut World) -> PlanManager {
        PlanManager {
            id,
            master_plan: Plan::default(),
            master_version: Version(ProposalID::new()),
            proposals: Some((initial_proposal_id, Proposal::default()))
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
            })
            .into_iter()
            .chain(self.master_plan.gestures.get(gesture_id))
            .next()
            .expect("Expected gesture (that point should be added to) to exist!")
    }

    pub fn implement(&mut self, proposal_id: ProposalID, world: &mut World) {
        let proposal = self
            .proposals
            .remove(proposal_id)
            .expect("Proposal should exist");

        self.master_plan = self.master_plan.merge(proposal.current_history());
        self.master_version = Version(proposal_id);

        Construction::global_first(world).implement(
            self.master_plan.calculate_result(self.master_version),
            world,
        );

        self.implemented_proposals.insert(proposal_id, proposal);

        let potentially_affected_ui_states = self
            .ui_state
            .values()
            .map(|state| (state.current_proposal, state.user_interface))
            .collect::<Vec<_>>();

        for (current_proposal, user_interface) in potentially_affected_ui_states {
            if current_proposal == proposal_id {
                let new_proposal_id = ProposalID::new();

                self.proposals.insert(new_proposal_id, Proposal::new());

                self.switch_to(user_interface, new_proposal_id, world);
            }
        }

        let all_proposal_ids = self.proposals.keys().cloned().collect::<Vec<_>>();
        for old_proposal_id in all_proposal_ids {
            if old_proposal_id != proposal_id {
                self.clear_previews(old_proposal_id);
                self.recreate_gesture_interactables(old_proposal_id, world);
            }
        }
    }

    pub fn implement_artificial_proposal(
        &mut self,
        proposal: &Proposal,
        based_on: Version,
        world: &mut World,
    ) {
        if based_on == self.master_version {
            let proposal_id = ProposalID::new();
            self.proposals.insert(proposal_id, proposal.clone());
            self.implement(proposal_id, world);
        }
    }
}

pub fn setup(system: &mut ActorSystem, user_interface: UserInterfaceID) -> PlanManagerID {
    system.register::<PlanManager>();
    auto_setup(system);
    rendering::auto_setup(system);
    interaction::setup(system);

    let initial_proposal_id = ProposalID::new();
    let plan_manager = PlanManagerID::spawn(initial_proposal_id, &mut system.world());
    plan_manager.switch_to(user_interface, initial_proposal_id, &mut system.world());
    plan_manager
}

pub mod kay_auto;
pub use self::kay_auto::*;
