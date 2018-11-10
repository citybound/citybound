//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct FrameListenerID {
    _raw_id: RawID
}

pub struct FrameListenerRepresentative;

impl ActorOrActorTrait for FrameListenerRepresentative {
    type ID = FrameListenerID;
}

impl TypedID for FrameListenerID {
    type Target = FrameListenerRepresentative;

    fn from_raw(id: RawID) -> Self {
        FrameListenerID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<A: Actor + FrameListener> TraitIDFrom<A> for FrameListenerID {}

impl FrameListenerID {
    pub fn on_frame(&self, world: &mut World) {
        world.send(self.as_raw(), MSG_FrameListener_on_frame());
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<FrameListenerRepresentative>();
        system.register_trait_message::<MSG_FrameListener_on_frame>();
    }

    pub fn register_implementor<A: Actor + FrameListener>(system: &mut ActorSystem) {
        system.register_implementor::<A, FrameListenerRepresentative>();
        system.add_handler::<A, _, _>(
            |&MSG_FrameListener_on_frame(), instance, world| {
                instance.on_frame(world); Fate::Live
            }, false
        );
    }
}

#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_FrameListener_on_frame();



#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    FrameListenerID::register_trait(system);
    
}