use kay::{World, MachineID, ActorSystem};
use compact::{CVec, CHashMap};
use descartes::{N, P2, Into2d, Circle, Path};
use stagemaster::UserInterfaceID;
use uuid::Uuid;

use transport::transport_planning_new;
use transport::transport_planning_new::{RoadIntent, RoadPrototype};

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
        Gesture { points, intent, deleted: false }
    }
}

#[derive(Compact, Clone)]
pub enum GestureIntent {
    Road(RoadIntent),
    Zone(ZoneIntent),
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
    redoable_history: CVec<Plan>,
}

impl Proposal {
    pub fn new() -> Proposal {
        Proposal::default()
    }

    pub fn start_new_step(&mut self) {
        self.undoable_history.push(Plan::default());
    }

    pub fn set_ongoing_step(&mut self, current_change: Plan) {
        if self.undoable_history.is_empty() {
            self.undoable_history.push(current_change);
        } else {
            *self.undoable_history.last_mut().unwrap() = current_change;
        }
        self.redoable_history.clear();
    }

    pub fn undo(&mut self) {
        if let Some(most_recent_step) = self.undoable_history.pop() {
            self.redoable_history.push(most_recent_step);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next_step_to_redo) = self.redoable_history.pop() {
            self.undoable_history.push(next_step_to_redo);
        }
    }

    pub fn current_history(&self) -> &[Plan] {
        &self.undoable_history
    }

    fn apply_to(&self, base: &Plan) -> Plan {
        base.merge(&self.undoable_history)
    }
}

#[derive(Compact, Clone)]
pub struct PlanResult {
    pub prototypes: CVec<Prototype>,
}

#[derive(Compact, Clone)]
pub enum Prototype {
    Road(RoadPrototype),
    Zone,
}

impl Plan {
    pub fn calculate_result(&self) -> PlanResult {
        let lane_prototypes = transport_planning_new::calculate_prototypes(&self);

        PlanResult { prototypes: lane_prototypes.into() }
    }
}

use self::interaction::PlanManagerUIState;

#[derive(Compact, Clone)]
pub struct PlanManager {
    id: PlanManagerID,
    master_plan: Plan,
    proposals: CVec<Proposal>,
    implemented_proposals: CVec<Proposal>,
    ui_state: CHashMap<MachineID, PlanManagerUIState>,
}

impl PlanManager {
    pub fn spawn(id: PlanManagerID, _: &mut World) -> PlanManager {
        PlanManager {
            id,
            master_plan: Plan::default(),
            proposals: vec![Proposal::default()].into(),
            implemented_proposals: CVec::new(),
            ui_state: CHashMap::new(),
        }
    }

    pub fn get_current_version_of(&self, gesture_id: GestureID, proposal_id: usize) -> &Gesture {
        self.proposals[proposal_id]
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
}

// Specific stuff

#[derive(Compact, Clone)]
pub enum ZoneIntent {
    LandUse(LandUse),
    MaxHeight(u8),
    SetBack(u8),
}

#[derive(Copy, Clone)]
pub enum LandUse {
    Residential,
    Commercial,
    Industrial,
    Agricultural,
    Recreational,
    Official,
}

pub fn setup(system: &mut ActorSystem, user_interface: UserInterfaceID) {
    system.register::<PlanManager>();
    auto_setup(system);
    rendering::auto_setup(system);
    interaction::setup(system);

    let plan_manager = PlanManagerID::spawn(&mut system.world());
    plan_manager.switch_to(user_interface, 0, &mut system.world());
}

pub mod kay_auto;
use self::kay_auto::*;