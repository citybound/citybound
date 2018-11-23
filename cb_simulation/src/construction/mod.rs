use kay::{World, Fate, ActorSystem};
use compact::{CVec, CHashMap};
use planning::{PrototypeID, Prototype, PrototypeKind, Action, ActionGroups};
use time::{Temporal, TemporalID, Instant};
use log::debug;
const LOG_T: &str = "Construction";

pub trait Constructable {
    fn morph(&mut self, new_prototype: &Prototype, report_to: ConstructionID, world: &mut World);
    fn destruct(&mut self, report_to: ConstructionID, world: &mut World) -> Fate;
}

impl Prototype {
    fn construct(&self, report_to: ConstructionID, world: &mut World) -> CVec<ConstructableID> {
        match self.kind {
            PrototypeKind::Road(ref road_prototype) => road_prototype.construct(report_to, world),
            PrototypeKind::Lot(ref lot_prototype) => {
                lot_prototype.construct(self.id, report_to, world)
            }
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
    queued_action_groups: ActionGroups,
    new_prototypes: CHashMap<PrototypeID, Prototype>,
}

impl Construction {
    pub fn spawn(id: ConstructionID, _world: &mut World) -> Construction {
        Construction {
            id,
            constructed: CHashMap::new(),
            pending_constructables: CVec::new(),
            queued_action_groups: ActionGroups(CVec::new()),
            new_prototypes: CHashMap::new(),
        }
    }

    pub fn action_done(&mut self, id: ConstructableID, _world: &mut World) {
        self.pending_constructables
            .retain(|pending_constructable| *pending_constructable != id);
    }

    fn start_action(&mut self, action: &Action, world: &mut World) {
        let new_pending_constructables = match *action {
            Action::Construct(prototype_id) => {
                debug(LOG_T, "C ", self.id, world);
                let new_prototype = self
                    .new_prototypes
                    .remove(prototype_id)
                    .expect("Should have prototype to be constructed");
                let ids = new_prototype.construct(self.id, world);
                self.constructed.insert(prototype_id, ids.clone());
                ids
            }
            Action::Morph(old_protoype_id, new_prototype_id) => {
                debug(LOG_T, "M ", self.id, world);
                let ids = self
                    .constructed
                    .remove(old_protoype_id)
                    .expect("Tried to morph non-constructed prototype");
                let new_prototype = self
                    .new_prototypes
                    .remove(new_prototype_id)
                    .expect("Should have prototype to be morphed to");
                for id in &ids {
                    id.morph(new_prototype.clone(), self.id, world);
                }
                self.constructed.insert(new_prototype_id, ids.clone());
                ids
            }
            Action::Destruct(prototype_id) => {
                debug(LOG_T, "D ", self.id, world);
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

    pub fn implement(
        &mut self,
        actions_to_implement: &ActionGroups,
        new_prototypes: &CVec<Prototype>,
        _world: &mut World,
    ) {
        self.queued_action_groups
            .0
            .extend(actions_to_implement.0.clone());
        for new_prototype in new_prototypes {
            self.new_prototypes
                .insert(new_prototype.id, new_prototype.clone());
        }
    }
}

impl Temporal for Construction {
    fn tick(&mut self, _dt: f32, _current_instant: Instant, world: &mut World) {
        if self.pending_constructables.is_empty() {
            if !self.queued_action_groups.0.is_empty() {
                debug(LOG_T, "Starting construction group:", self.id, world);
                let next_action_group = self.queued_action_groups.0.remove(0);
                for action in next_action_group.0 {
                    self.start_action(&action, world);
                }
                debug(LOG_T, "Finished construction group:", self.id, world);
            }
        } else {
            debug(
                LOG_T,
                format!(
                    "Construction pending: {} - queued groups: {}",
                    self.pending_constructables.len(),
                    self.queued_action_groups.0.len()
                ),
                self.id,
                world,
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
