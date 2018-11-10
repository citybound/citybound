//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct TimeUIID {
    _raw_id: RawID
}

pub struct TimeUIRepresentative;

impl ActorOrActorTrait for TimeUIRepresentative {
    type ID = TimeUIID;
}

impl TypedID for TimeUIID {
    type Target = TimeUIRepresentative;

    fn from_raw(id: RawID) -> Self {
        TimeUIID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<A: Actor + TimeUI> TraitIDFrom<A> for TimeUIID {}

impl TimeUIID {
    pub fn on_time_info(&self, current_instant: :: time :: Instant, speed: u16, world: &mut World) {
        world.send(self.as_raw(), MSG_TimeUI_on_time_info(current_instant, speed));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<TimeUIRepresentative>();
        system.register_trait_message::<MSG_TimeUI_on_time_info>();
    }

    pub fn register_implementor<A: Actor + TimeUI>(system: &mut ActorSystem) {
        system.register_implementor::<A, TimeUIRepresentative>();
        system.add_handler::<A, _, _>(
            |&MSG_TimeUI_on_time_info(current_instant, speed), instance, world| {
                instance.on_time_info(current_instant, speed, world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_TimeUI_on_time_info(pub :: time :: Instant, pub u16);



impl TimeID {
    pub fn get_info(&self, requester: TimeUIID, world: &mut World) {
        world.send(self.as_raw(), MSG_Time_get_info(requester));
    }
    
    pub fn set_speed(&self, speed: u16, world: &mut World) {
        world.send(self.as_raw(), MSG_Time_set_speed(speed));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Time_get_info(pub TimeUIID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Time_set_speed(pub u16);


#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    TimeUIID::register_trait(system);
    
    system.add_handler::<Time, _, _>(
        |&MSG_Time_get_info(requester), instance, world| {
            instance.get_info(requester, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Time, _, _>(
        |&MSG_Time_set_speed(speed), instance, world| {
            instance.set_speed(speed, world); Fate::Live
        }, false
    );
}