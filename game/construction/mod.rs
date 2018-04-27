use kay::{World, Fate};
use compact::{CVec, CHashMap};
use planning_new::{PlanResult, PrototypeID, Prototype};
use core::simulation::{Simulatable, SimulatableID, Instant};

pub trait Constructable {
    fn morph(&mut self, new_prototype: Prototype, world: &mut World);
    fn destruct(&mut self, world: &mut World) -> Fate;
}

impl Prototype {
    fn construct(&self, world: &mut World) -> CVec<ConstructableID> {
        match *self {
            Prototype::Road(road_prototype) => road_prototype.construct(world),
            Prototype::Lot(lot_prototype) => lot_prototype.construct(world),
        }
    }

    fn morphable_from(&self, other: &Self) -> bool {
        match (*self, *other) {
            (Prototype::Lot(self_lot), Prototype::Lot(other_lot)) => {
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

#[derive(Compact, Clone)]
pub enum Action {
    Construct(PrototypeID, Prototype),
    Morph(PrototypeID, PrototypeID, Prototype),
    Destruct(PrototypeID),
}

impl Construction {
    pub fn action_done(&mut self, id: ConstructableID, world: &mut World) {
        self.pending_constructables.retain(
            |pending_constructable| {
                *pending_constructable != id
            },
        );
    }

    fn start_action(&mut self, action: Action, world: &mut World) {
        let new_pending_constructables = match action {
            Action::Construct(prototype_id, prototype) => {
                let ids = prototype.construct(world);
                self.constructed.insert(prototype_id, ids.clone());
                self.current_prototypes.insert(
                    prototype_id,
                    prototype.clone(),
                );
                ids
            }
            Action::Morph(old_protoype_id, new_prototype_id, new_prototype) => {
                let ids = self.constructed.remove(old_protoype_id).expect(
                    "Tried to morph non-constructed prototype",
                );
                for id in ids {
                    id.morph(new_prototype.clone(), world);
                }
                self.constructed.insert(new_prototype_id, ids.clone());
                self.current_prototypes.remove(old_protoype_id);
                self.current_prototypes.insert(
                    new_prototype_id,
                    new_prototype,
                );
                ids
            }
            Action::Destruct(prototype_id) => {
                let ids = self.constructed.remove(prototype_id).expect(
                    "Tried to destruct non-constructed prototype",
                );
                for id in ids {
                    id.destruct(world);
                }
                self.current_prototypes.remove(prototype_id);
                ids
            }
        };

        self.pending_constructables.extend(
            new_pending_constructables,
        );
    }

    fn actions_to_implement(&self, new_result: &PlanResult) -> CVec<CVec<Action>> {
        let mut unmatched_existing = self.current_prototypes.clone();
        let mut to_be_morphed = CVec::new();
        let mut to_be_constructed = CVec::new();

        for (new_prototype_id, new_prototype) in new_result.prototypes.pairs() {
            if let Some((morphable_id, _)) =
                unmatched_existing.pairs().find(|&(_, other_prototype)| {
                    new_prototype.morphable_from(other_prototype)
                })
            {
                unmatched_existing.remove(*morphable_id);
                to_be_morphed.push(Action::Morph(
                    *morphable_id,
                    *new_prototype_id,
                    new_prototype.clone(),
                ));
            } else {
                to_be_constructed.push(Action::Construct(*new_prototype_id, new_prototype.clone()))
            }
        }

        let mut to_be_destructed = unmatched_existing
            .keys()
            .map(|unmatched_id| Action::Destruct(*unmatched_id))
            .collect();

        vec![to_be_destructed, to_be_morphed, to_be_constructed].into()
    }

    pub fn implement(&self, new_result: &PlanResult, world: &mut World) {
        self.queued_actions.extend(
            self.actions_to_implement(new_result),
        );
    }
}

impl Simulatable for Construction {
    fn tick(&mut self, _dt: f32, _current_instant: Instant, world: &mut World) {
        if self.pending_constructables.is_empty() && !self.queued_actions.is_empty() {
            let next_action_group = self.queued_actions.remove(0);
            for action in next_action_group {
                self.start_action(action, world);
            }
        }
    }
}

mod kay_auto;
use self::kay_auto::*;