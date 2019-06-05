//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct ConstructableID<PK: PrototypeKind> {
    _raw_id: RawID, _marker: ::std::marker::PhantomData<Box<(PK)>>
}

impl<PK: PrototypeKind> Copy for ConstructableID<PK> {}
impl<PK: PrototypeKind> Clone for ConstructableID<PK> { fn clone(&self) -> Self { *self } }
impl<PK: PrototypeKind> ::std::fmt::Debug for ConstructableID<PK> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "ConstructableID<PK>({:?})", self._raw_id)
    }
}
impl<PK: PrototypeKind> ::std::hash::Hash for ConstructableID<PK> {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl<PK: PrototypeKind> PartialEq for ConstructableID<PK> {
    fn eq(&self, other: &ConstructableID<PK>) -> bool {
        self._raw_id == other._raw_id
    }
}
impl<PK: PrototypeKind> Eq for ConstructableID<PK> {}

pub struct ConstructableRepresentative<PK: PrototypeKind>{ _marker: ::std::marker::PhantomData<Box<(PK)>> }

impl<PK: PrototypeKind> ActorOrActorTrait for ConstructableRepresentative<PK> {
    type ID = ConstructableID<PK>;
}

impl<PK: PrototypeKind> TypedID for ConstructableID<PK> {
    type Target = ConstructableRepresentative<PK>;

    fn from_raw(id: RawID) -> Self {
        ConstructableID { _raw_id: id, _marker: ::std::marker::PhantomData }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<PK: PrototypeKind, Act: Actor + Constructable<PK>> TraitIDFrom<Act> for ConstructableID<PK> {}

impl<PK: PrototypeKind> ConstructableID<PK> {
    pub fn morph(self, new_prototype: Prototype < PK >, report_to: ConstructionID < PK >, world: &mut World) {
        world.send(self.as_raw(), MSG_Constructable_morph::<PK>(new_prototype, report_to));
    }
    
    pub fn destruct(self, report_to: ConstructionID < PK >, world: &mut World) {
        world.send(self.as_raw(), MSG_Constructable_destruct::<PK>(report_to));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<ConstructableRepresentative<PK>>();
        system.register_trait_message::<MSG_Constructable_morph<PK>>();
        system.register_trait_message::<MSG_Constructable_destruct<PK>>();
    }

    pub fn register_implementor<Act: Actor + Constructable<PK>>(system: &mut ActorSystem) {
        system.register_implementor::<Act, ConstructableRepresentative<PK>>();
        system.add_handler::<Act, _, _>(
            |&MSG_Constructable_morph::<PK>(ref new_prototype, report_to), instance, world| {
                instance.morph(new_prototype, report_to, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Constructable_destruct::<PK>(report_to), instance, world| {
                instance.destruct(report_to, world)
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Constructable_morph<PK: PrototypeKind>(pub Prototype < PK >, pub ConstructionID < PK >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Constructable_destruct<PK: PrototypeKind>(pub ConstructionID < PK >);

impl<PK: PrototypeKind> Actor for Construction<PK> {
    type ID = ConstructionID<PK>;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct ConstructionID<PK: PrototypeKind> {
    _raw_id: RawID, _marker: ::std::marker::PhantomData<Box<(PK)>>
}

impl<PK: PrototypeKind> Copy for ConstructionID<PK> {}
impl<PK: PrototypeKind> Clone for ConstructionID<PK> { fn clone(&self) -> Self { *self } }
impl<PK: PrototypeKind> ::std::fmt::Debug for ConstructionID<PK> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "ConstructionID<PK>({:?})", self._raw_id)
    }
}
impl<PK: PrototypeKind> ::std::hash::Hash for ConstructionID<PK> {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl<PK: PrototypeKind> PartialEq for ConstructionID<PK> {
    fn eq(&self, other: &ConstructionID<PK>) -> bool {
        self._raw_id == other._raw_id
    }
}
impl<PK: PrototypeKind> Eq for ConstructionID<PK> {}

impl<PK: PrototypeKind> TypedID for ConstructionID<PK> {
    type Target = Construction<PK>;

    fn from_raw(id: RawID) -> Self {
        ConstructionID { _raw_id: id, _marker: ::std::marker::PhantomData }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<PK: PrototypeKind> ConstructionID<PK> {
    pub fn spawn(world: &mut World) -> Self {
        let id = ConstructionID::<PK>::from_raw(world.allocate_instance_id::<Construction<PK>>());
        let swarm = world.local_broadcast::<Construction<PK>>();
        world.send(swarm, MSG_Construction_spawn::<PK>(id, ));
        id
    }
    
    pub fn action_done(self, id: ConstructableID < PK >, world: &mut World) {
        world.send(self.as_raw(), MSG_Construction_action_done::<PK>(id));
    }
    
    pub fn implement(self, actions_to_implement: ActionGroups, new_prototypes: CVec < Prototype < PK > >, world: &mut World) {
        world.send(self.as_raw(), MSG_Construction_implement::<PK>(actions_to_implement, new_prototypes));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Construction_spawn<PK: PrototypeKind>(pub ConstructionID<PK>, );
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Construction_action_done<PK: PrototypeKind>(pub ConstructableID < PK >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Construction_implement<PK: PrototypeKind>(pub ActionGroups, pub CVec < Prototype < PK > >);

impl<PK: PrototypeKind> Into<TemporalID> for ConstructionID<PK> {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup<PK: PrototypeKind>(system: &mut ActorSystem) {
    ConstructableID::<PK>::register_trait(system);
    TemporalID::register_implementor::<Construction<PK>>(system);
    system.add_spawner::<Construction<PK>, _, _>(
        |&MSG_Construction_spawn::<PK>(id, ), world| {
            Construction::<PK>::spawn(id, world)
        }, false
    );
    
    system.add_handler::<Construction<PK>, _, _>(
        |&MSG_Construction_action_done::<PK>(id), instance, world| {
            instance.action_done(id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Construction<PK>, _, _>(
        |&MSG_Construction_implement::<PK>(ref actions_to_implement, ref new_prototypes), instance, world| {
            instance.implement(actions_to_implement, new_prototypes, world); Fate::Live
        }, false
    );
}