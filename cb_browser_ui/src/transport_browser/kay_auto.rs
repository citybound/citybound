//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for BrowserTransportUI {
    type ID = BrowserTransportUIID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct BrowserTransportUIID {
    _raw_id: RawID
}

impl Copy for BrowserTransportUIID {}
impl Clone for BrowserTransportUIID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for BrowserTransportUIID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "BrowserTransportUIID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for BrowserTransportUIID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for BrowserTransportUIID {
    fn eq(&self, other: &BrowserTransportUIID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for BrowserTransportUIID {}

impl TypedID for BrowserTransportUIID {
    type Target = BrowserTransportUI;

    fn from_raw(id: RawID) -> Self {
        BrowserTransportUIID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl BrowserTransportUIID {
    pub fn spawn(world: &mut World) -> Self {
        let id = BrowserTransportUIID::from_raw(world.allocate_instance_id::<BrowserTransportUI>());
        let swarm = world.local_broadcast::<BrowserTransportUI>();
        world.send(swarm, MSG_BrowserTransportUI_spawn(id, ));
        id
    }
}

#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_BrowserTransportUI_spawn(pub BrowserTransportUIID, );

impl Into<FrameListenerID> for BrowserTransportUIID {
    fn into(self) -> FrameListenerID {
        FrameListenerID::from_raw(self.as_raw())
    }
}

impl Into<TransportUIID> for BrowserTransportUIID {
    fn into(self) -> TransportUIID {
        TransportUIID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    FrameListenerID::register_implementor::<BrowserTransportUI>(system);
    TransportUIID::register_implementor::<BrowserTransportUI>(system);
    system.add_spawner::<BrowserTransportUI, _, _>(
        |&MSG_BrowserTransportUI_spawn(id, ), world| {
            BrowserTransportUI::spawn(id, world)
        }, false
    );
}