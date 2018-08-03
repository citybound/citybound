use kay::{World, Fate, ActorSystem};
use compact::{CVec, CHashMap};
use planning::{PrototypeID, Prototype, PrototypeKind};
use simulation::{Simulatable, SimulatableID, Instant};

pub trait Constructable {
    fn morph(&mut self, new_prototype: &Prototype, report_to: ConstructionID, world: &mut World);
    fn destruct(&mut self, report_to: ConstructionID, world: &mut World) -> Fate;
}

impl Prototype {
    fn construct(&self, report_to: ConstructionID, world: &mut World) -> CVec<ConstructableID> {
        match self.kind {
            PrototypeKind::Road(ref road_prototype) => road_prototype.construct(report_to, world),
            PrototypeKind::Lot(ref lot_prototype) => lot_prototype.construct(report_to, world),
        }
    }

    pub fn morphable_from(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (&PrototypeKind::Road(ref self_road), &PrototypeKind::Road(ref other_road)) => {
                self_road.morphable_from(other_road)
            }
            (&PrototypeKind::Lot(ref self_lot), &PrototypeKind::Lot(ref other_lot)) => {
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
                ids
            }
        };

        self.pending_constructables
            .extend(new_pending_constructables);
    }

    pub fn implement(&mut self, actions_to_implement: &CVec<CVec<Action>>, _world: &mut World) {
        self.queued_actions.extend(actions_to_implement.clone());
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
