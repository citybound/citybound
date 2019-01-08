//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct ConstructableID {
    _raw_id: RawID
}

pub struct ConstructableRepresentative;

impl ActorOrActorTrait for ConstructableRepresentative {
    type ID = ConstructableID;
}

impl TypedID for ConstructableID {
    type Target = ConstructableRepresentative;

    fn from_raw(id: RawID) -> Self {
        ConstructableID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<A: Actor + Constructable> TraitIDFrom<A> for ConstructableID {}

impl ConstructableID {
    pub fn morph(self, new_prototype: Prototype, report_to: ConstructionID, world: &mut World) {
        world.send(self.as_raw(), MSG_Constructable_morph(new_prototype, report_to));
    }
    
    pub fn destruct(self, report_to: ConstructionID, world: &mut World) {
        world.send(self.as_raw(), MSG_Constructable_destruct(report_to));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<ConstructableRepresentative>();
        system.register_trait_message::<MSG_Constructable_morph>();
        system.register_trait_message::<MSG_Constructable_destruct>();
    }

    pub fn register_implementor<A: Actor + Constructable>(system: &mut ActorSystem) {
        system.register_implementor::<A, ConstructableRepresentative>();
        system.add_handler::<A, _, _>(
            |&MSG_Constructable_morph(ref new_prototype, report_to), instance, world| {
                instance.morph(new_prototype, report_to, world); Fate::Live
            }, false
        );
        
        system.add_handler::<A, _, _>(
            |&MSG_Constructable_destruct(report_to), instance, world| {
                instance.destruct(report_to, world)
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Constructable_morph(pub Prototype, pub ConstructionID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Constructable_destruct(pub ConstructionID);

impl Actor for Construction {
    type ID = ConstructionID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct ConstructionID {
    _raw_id: RawID
}

impl TypedID for ConstructionID {
    type Target = Construction;

    fn from_raw(id: RawID) -> Self {
        ConstructionID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl ConstructionID {
    pub fn spawn(world: &mut World) -> Self {
        let id = ConstructionID::from_raw(world.allocate_instance_id::<Construction>());
        let swarm = world.local_broadcast::<Construction>();
        world.send(swarm, MSG_Construction_spawn(id, ));
        id
    }
    
    pub fn action_done(self, id: ConstructableID, world: &mut World) {
        world.send(self.as_raw(), MSG_Construction_action_done(id));
    }
    
    pub fn implement(self, actions_to_implement: ActionGroups, new_prototypes: CVec < Prototype >, world: &mut World) {
        world.send(self.as_raw(), MSG_Construction_implement(actions_to_implement, new_prototypes));
    }
}

#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_Construction_spawn(pub ConstructionID, );
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Construction_action_done(pub ConstructableID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Construction_implement(pub ActionGroups, pub CVec < Prototype >);

impl Into<TemporalID> for ConstructionID {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    ConstructableID::register_trait(system);
    TemporalID::register_implementor::<Construction>(system);
    system.add_spawner::<Construction, _, _>(
        |&MSG_Construction_spawn(id, ), world| {
            Construction::spawn(id, world)
        }, false
    );
    
    system.add_handler::<Construction, _, _>(
        |&MSG_Construction_action_done(id), instance, world| {
            instance.action_done(id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Construction, _, _>(
        |&MSG_Construction_implement(ref actions_to_implement, ref new_prototypes), instance, world| {
            instance.implement(actions_to_implement, new_prototypes, world); Fate::Live
        }, false
    );
}