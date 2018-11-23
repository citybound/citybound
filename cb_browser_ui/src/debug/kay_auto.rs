//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for LogUI {
    type ID = LogUIID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct LogUIID {
    _raw_id: RawID
}

impl TypedID for LogUIID {
    type Target = LogUI;

    fn from_raw(id: RawID) -> Self {
        LogUIID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl LogUIID {
    pub fn spawn(world: &mut World) -> Self {
        let id = LogUIID::from_raw(world.allocate_instance_id::<LogUI>());
        let swarm = world.local_broadcast::<LogUI>();
        world.send(swarm, MSG_LogUI_spawn(id, ));
        id
    }
}

#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_LogUI_spawn(pub LogUIID, );

impl Into<LogRecipientID> for LogUIID {
    fn into(self) -> LogRecipientID {
        LogRecipientID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    LogRecipientID::register_implementor::<LogUI>(system);
    system.add_spawner::<LogUI, _, _>(
        |&MSG_LogUI_spawn(id, ), world| {
            LogUI::spawn(id, world)
        }, false
    );
}