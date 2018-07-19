use kay::{World, Fate, ActorSystem};
use compact::{CVec, CHashMap};
use planning::{PlanResult, PrototypeID, Prototype, PlanManagerID, ProposalID};
use simulation::{Simulatable, SimulatableID, Instant};

pub trait Constructable {
    fn morph(&mut self, new_prototype: &Prototype, report_to: ConstructionID, world: &mut World);
    fn destruct(&mut self, report_to: ConstructionID, world: &mut World) -> Fate;
}

impl Prototype {
    fn construct(&self, report_to: ConstructionID, world: &mut World) -> CVec<ConstructableID> {
        match *self {
            Prototype::Road(ref road_prototype) => road_prototype.construct(report_to, world),
            Prototype::Lot(ref lot_prototype) => lot_prototype.construct(report_to, world),
        }
    }

    fn morphable_from(&self, other: &Self) -> bool {
        match (self, other) {
            (&Prototype::Road(ref self_road), &Prototype::Road(ref other_road)) => {
                self_road.morphable_from(other_road)
            }
            (&Prototype::Lot(ref self_lot), &Prototype::Lot(ref other_lot)) => {
                self_lot.morphable_from(other_lot)
            }
            _ => false,
        }
    }
}

#[derive(Compact, Clone)]
pub struct Construction {
    id: ConstructionID,
    constructed: CHashMap<PrototypeID, CVec<ConstructableID>>,
    current_prototypes: CHashMap<PrototypeID, Prototype>,
    pending_constructables: CVec<ConstructableID>,
    queued_actions: CVec<CVec<Action>>,
}

#[derive(Compact, Clone, Serialize, Deserialize)]
pub enum Action {
    Construct(PrototypeID, Prototype),
    Morph(PrototypeID, PrototypeID, Prototype),
    Destruct(PrototypeID),
}

impl Construction {
    pub fn spawn(id: ConstructionID, _world: &mut World) -> Construction {
        Construction {
            id,
            constructed: CHashMap::new(),
            current_prototypes: CHashMap::new(),
            pending_constructables: CVec::new(),
            queued_actions: CVec::new(),
        }
    }

    pub fn action_done(&mut self, id: ConstructableID, _world: &mut World) {
        self.pending_constructables
            .retain(|pending_constructable| *pending_constructable != id);
    }

    fn start_action(&mut self, action: Action, world: &mut World) {
        let new_pending_constructables = match action {
            Action::Construct(prototype_id, prototype) => {
                print!("C ");
                let ids = prototype.construct(self.id, world);
                self.constructed.insert(prototype_id, ids.clone());
                self.current_prototypes
                    .insert(prototype_id, prototype.clone());
                ids
            }
            Action::Morph(old_protoype_id, new_prototype_id, new_prototype) => {
                print!("M ");
                let ids = self
                    .constructed
                    .remove(old_protoype_id)
                    .expect("Tried to morph non-constructed prototype");
                for id in &ids {
                    id.morph(new_prototype.clone(), self.id, world);
                }
                self.constructed.insert(new_prototype_id, ids.clone());
                self.current_prototypes.remove(old_protoype_id);
                self.current_prototypes
                    .insert(new_prototype_id, new_prototype);
                ids
            }
            Action::Destruct(prototype_id) => {
                print!("D ");
                let ids = self
                    .constructed
                    .remove(prototype_id)
                    .expect("Tried to destruct non-constructed prototype");
                for id in &ids {
                    id.destruct(self.id, world);
                }
                self.current_prototypes.remove(prototype_id);
                ids
            }
        };

        self.pending_constructables
            .extend(new_pending_constructables);
    }

    fn actions_to_implement(&self, new_result: &PlanResult) -> CVec<CVec<Action>> {
        // TODO: compare this not to the currently constructed state
        // but to the current construction GOAL!!
        let mut unmatched_existing = self.current_prototypes.clone();
        let mut to_be_morphed = CVec::new();
        let mut to_be_constructed = CVec::new();

        for (new_prototype_id, new_prototype) in new_result.prototypes.pairs() {
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
                to_be_constructed.push(Action::Construct(*new_prototype_id, new_prototype.clone()))
            }
        }

        let to_be_destructed = unmatched_existing
            .keys()
            .map(|unmatched_id| Action::Destruct(*unmatched_id))
            .collect();

        vec![to_be_destructed, to_be_morphed, to_be_constructed].into()
    }

    pub fn implement(&mut self, new_result: &PlanResult, _world: &mut World) {
        let actions_to_implement = self.actions_to_implement(new_result);
        self.queued_actions.extend(actions_to_implement);
    }

    pub fn simulate(
        &mut self,
        result: &PlanResult,
        plan_manager: PlanManagerID,
        proposal_id: ProposalID,
        world: &mut World,
    ) {
        plan_manager.on_simulated_actions(proposal_id, self.actions_to_implement(result), world);
    }
}

impl Simulatable for Construction {
    fn tick(&mut self, _dt: f32, _current_instant: Instant, world: &mut World) {
        if self.pending_constructables.is_empty() {
            if !self.queued_actions.is_empty() {
                println!("Starting construction group:");
                let next_action_group = self.queued_actions.remove(0);
                for action in next_action_group {
                    self.start_action(action, world);
                }
                println!("\nFinished construction group:");
            }
        } else {
            println!(
                "Construction pending: {} - queued groups: {}",
                self.pending_constructables.len(),
                self.queued_actions.len()
            );
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Construction>();
    auto_setup(system);
}

pub fn spawn(world: &mut World) {
    ConstructionID::spawn(world);
}

mod kay_auto;
pub use self::kay_auto::*;
