//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for ArchitectureRuleManager {
    type ID = ArchitectureRuleManagerID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct ArchitectureRuleManagerID {
    _raw_id: RawID
}

impl TypedID for ArchitectureRuleManagerID {
    type Target = ArchitectureRuleManager;

    fn from_raw(id: RawID) -> Self {
        ArchitectureRuleManagerID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl ArchitectureRuleManagerID {
    pub fn init(world: &mut World) -> Self {
        let id = ArchitectureRuleManagerID::from_raw(world.allocate_instance_id::<ArchitectureRuleManager>());
        let swarm = world.local_broadcast::<ArchitectureRuleManager>();
        world.send(swarm, MSG_ArchitectureRuleManager_init(id, ));
        id
    }
}

#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_ArchitectureRuleManager_init(pub ArchitectureRuleManagerID, );


#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    
    system.add_spawner::<ArchitectureRuleManager, _, _>(
        |&MSG_ArchitectureRuleManager_init(id, ), world| {
            ArchitectureRuleManager::init(id, world)
        }, false
    );
}