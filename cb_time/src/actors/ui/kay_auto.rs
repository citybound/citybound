//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct TimeUIID {
    _raw_id: RawID
}

impl Copy for TimeUIID {}
impl Clone for TimeUIID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for TimeUIID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "TimeUIID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for TimeUIID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for TimeUIID {
    fn eq(&self, other: &TimeUIID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for TimeUIID {}

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

impl<Act: Actor + TimeUI> TraitIDFrom<Act> for TimeUIID {}

impl TimeUIID {
    pub fn on_time_info(self, current_instant: :: units :: Instant, speed: u16, world: &mut World) {
        world.send(self.as_raw(), MSG_TimeUI_on_time_info(current_instant, speed));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<TimeUIRepresentative>();
        system.register_trait_message::<MSG_TimeUI_on_time_info>();
    }

    pub fn register_implementor<Act: Actor + TimeUI>(system: &mut ActorSystem) {
        system.register_implementor::<Act, TimeUIRepresentative>();
        system.add_handler::<Act, _, _>(
            |&MSG_TimeUI_on_time_info(current_instant, speed), instance, world| {
                instance.on_time_info(current_instant, speed, world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_TimeUI_on_time_info(pub :: units :: Instant, pub u16);



impl TimeID {
    pub fn get_info(self, requester: TimeUIID, world: &mut World) {
        world.send(self.as_raw(), MSG_Time_get_info(requester));
    }
    
    pub fn set_speed(self, speed: u16, world: &mut World) {
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