//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct FrameListenerID {
    _raw_id: RawID
}

impl Copy for FrameListenerID {}
impl Clone for FrameListenerID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for FrameListenerID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "FrameListenerID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for FrameListenerID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for FrameListenerID {
    fn eq(&self, other: &FrameListenerID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for FrameListenerID {}

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

impl<Act: Actor + FrameListener> TraitIDFrom<Act> for FrameListenerID {}

impl FrameListenerID {
    pub fn on_frame(self, world: &mut World) {
        world.send(self.as_raw(), MSG_FrameListener_on_frame());
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<FrameListenerRepresentative>();
        system.register_trait_message::<MSG_FrameListener_on_frame>();
    }

    pub fn register_implementor<Act: Actor + FrameListener>(system: &mut ActorSystem) {
        system.register_implementor::<Act, FrameListenerRepresentative>();
        system.add_handler::<Act, _, _>(
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