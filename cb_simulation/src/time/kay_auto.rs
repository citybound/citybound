//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct TemporalID {
    _raw_id: RawID
}

pub struct TemporalRepresentative;

impl ActorOrActorTrait for TemporalRepresentative {
    type ID = TemporalID;
}

impl TypedID for TemporalID {
    type Target = TemporalRepresentative;

    fn from_raw(id: RawID) -> Self {
        TemporalID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<A: Actor + Temporal> TraitIDFrom<A> for TemporalID {}

impl TemporalID {
    pub fn tick(self, dt: f32, current_instant: Instant, world: &mut World) {
        world.send(self.as_raw(), MSG_Temporal_tick(dt, current_instant));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<TemporalRepresentative>();
        system.register_trait_message::<MSG_Temporal_tick>();
    }

    pub fn register_implementor<A: Actor + Temporal>(system: &mut ActorSystem) {
        system.register_implementor::<A, TemporalRepresentative>();
        system.add_handler::<A, _, _>(
            |&MSG_Temporal_tick(dt, current_instant), instance, world| {
                instance.tick(dt, current_instant, world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Temporal_tick(pub f32, pub Instant);
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct SleeperID {
    _raw_id: RawID
}

pub struct SleeperRepresentative;

impl ActorOrActorTrait for SleeperRepresentative {
    type ID = SleeperID;
}

impl TypedID for SleeperID {
    type Target = SleeperRepresentative;

    fn from_raw(id: RawID) -> Self {
        SleeperID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<A: Actor + Sleeper> TraitIDFrom<A> for SleeperID {}

impl SleeperID {
    pub fn wake(self, current_instant: Instant, world: &mut World) {
        world.send(self.as_raw(), MSG_Sleeper_wake(current_instant));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<SleeperRepresentative>();
        system.register_trait_message::<MSG_Sleeper_wake>();
    }

    pub fn register_implementor<A: Actor + Sleeper>(system: &mut ActorSystem) {
        system.register_implementor::<A, SleeperRepresentative>();
        system.add_handler::<A, _, _>(
            |&MSG_Sleeper_wake(current_instant), instance, world| {
                instance.wake(current_instant, world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Sleeper_wake(pub Instant);

impl Actor for Time {
    type ID = TimeID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct TimeID {
    _raw_id: RawID
}

impl TypedID for TimeID {
    type Target = Time;

    fn from_raw(id: RawID) -> Self {
        TimeID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl TimeID {
    pub fn spawn(world: &mut World) -> Self {
        let id = TimeID::from_raw(world.allocate_instance_id::<Time>());
        let swarm = world.local_broadcast::<Time>();
        world.send(swarm, MSG_Time_spawn(id, ));
        id
    }
    
    pub fn progress(self, world: &mut World) {
        world.send(self.as_raw(), MSG_Time_progress());
    }
    
    pub fn wake_up_in(self, remaining_ticks: Ticks, sleeper_id: SleeperID, world: &mut World) {
        world.send(self.as_raw(), MSG_Time_wake_up_in(remaining_ticks, sleeper_id));
    }
}

#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_Time_spawn(pub TimeID, );
#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_Time_progress();
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Time_wake_up_in(pub Ticks, pub SleeperID);


#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    TemporalID::register_trait(system);
    SleeperID::register_trait(system);
    
    system.add_spawner::<Time, _, _>(
        |&MSG_Time_spawn(id, ), world| {
            Time::spawn(id, world)
        }, false
    );
    
    system.add_handler::<Time, _, _>(
        |&MSG_Time_progress(), instance, world| {
            instance.progress(world); Fate::Live
        }, false
    );
    
    system.add_handler::<Time, _, _>(
        |&MSG_Time_wake_up_in(remaining_ticks, sleeper_id), instance, world| {
            instance.wake_up_in(remaining_ticks, sleeper_id, world); Fate::Live
        }, false
    );
}