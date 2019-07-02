use kay::{World, Fate, ActorSystem};
use compact::{CVec, CHashMap, Compact};
use ::{PrototypeID, Prototype, Action, ActionGroups};
use cb_time::actors::{Temporal, TemporalID};
use cb_time::units::Instant;
use cb_util::log::debug;
const LOG_T: &str = "Construction";

pub trait PrototypeKind: Compact + 'static {
    fn construct(
        &self,
        prototype_id: PrototypeID,
        report_to: ConstructionID<Self>,
        world: &mut World,
    ) -> CVec<ConstructableID<Self>>;

    fn morphable_from(&self, other: &Self) -> bool;
}

pub trait GestureIntent: Compact + 'static {}

pub trait Constructable<PK: PrototypeKind> {
    fn morph(
        &mut self,
        new_prototype: &Prototype<PK>,
        report_to: ConstructionID<PK>,
        world: &mut World,
    );

    fn destruct(&mut self, report_to: ConstructionID<PK>, world: &mut World) -> Fate;
}

impl<PK: PrototypeKind> Prototype<PK> {
    fn construct(
        &self,
        report_to: ConstructionID<PK>,
        world: &mut World,
    ) -> CVec<ConstructableID<PK>> {
        self.kind.construct(self.id, report_to, world)
    }

    pub fn morphable_from(&self, other: &Self) -> bool {
        self.kind.morphable_from(&other.kind)
    }
}

#[derive(Compact, Clone)]
//#[derive(Clone)]
pub struct Construction<PK: PrototypeKind> {
    id: ConstructionID<PK>,
    constructed: CHashMap<PrototypeID, CVec<ConstructableID<PK>>>,
    pending_constructables: CVec<ConstructableID<PK>>,
    queued_action_groups: ActionGroups,
    new_prototypes: CHashMap<PrototypeID, Prototype<PK>>,
}

//mod compact_workaround;

impl<PK: PrototypeKind> Construction<PK> {
    pub fn spawn(id: ConstructionID<PK>, _world: &mut World) -> Construction<PK> {
        Construction {
            id,
            constructed: CHashMap::new(),
            pending_constructables: CVec::new(),
            queued_action_groups: ActionGroups(CVec::new()),
            new_prototypes: CHashMap::new(),
        }
    }

    pub fn action_done(&mut self, id: ConstructableID<PK>, _world: &mut World) {
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
        new_prototypes: &CVec<Prototype<PK>>,
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

impl<PK: PrototypeKind> Temporal for Construction<PK> {
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

pub fn setup<PK: PrototypeKind>(system: &mut ActorSystem) {
    system.register::<Construction<PK>>();
    auto_setup::<PK>(system);
}

pub fn spawn<PK: PrototypeKind>(world: &mut World) {
    ConstructionID::<PK>::spawn(world);
}

mod kay_auto;
pub use self::kay_auto::*;
