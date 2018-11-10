//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for BrowserTimeUI {
    type ID = BrowserTimeUIID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct BrowserTimeUIID {
    _raw_id: RawID
}

impl TypedID for BrowserTimeUIID {
    type Target = BrowserTimeUI;

    fn from_raw(id: RawID) -> Self {
        BrowserTimeUIID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl BrowserTimeUIID {
    pub fn spawn(world: &mut World) -> Self {
        let id = BrowserTimeUIID::from_raw(world.allocate_instance_id::<BrowserTimeUI>());
        let swarm = world.local_broadcast::<BrowserTimeUI>();
        world.send(swarm, MSG_BrowserTimeUI_spawn(id, ));
        id
    }
}

#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_BrowserTimeUI_spawn(pub BrowserTimeUIID, );

impl Into<FrameListenerID> for BrowserTimeUIID {
    fn into(self) -> FrameListenerID {
        FrameListenerID::from_raw(self.as_raw())
    }
}

impl Into<TimeUIID> for BrowserTimeUIID {
    fn into(self) -> TimeUIID {
        TimeUIID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    FrameListenerID::register_implementor::<BrowserTimeUI>(system);
    TimeUIID::register_implementor::<BrowserTimeUI>(system);
    system.add_spawner::<BrowserTimeUI, _, _>(
        |&MSG_BrowserTimeUI_spawn(id, ), world| {
            BrowserTimeUI::spawn(id, world)
        }, false
    );
}