//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for BrowserLandUseUI {
    type ID = BrowserLandUseUIID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct BrowserLandUseUIID {
    _raw_id: RawID
}

impl Copy for BrowserLandUseUIID {}
impl Clone for BrowserLandUseUIID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for BrowserLandUseUIID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "BrowserLandUseUIID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for BrowserLandUseUIID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for BrowserLandUseUIID {
    fn eq(&self, other: &BrowserLandUseUIID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for BrowserLandUseUIID {}

impl TypedID for BrowserLandUseUIID {
    type Target = BrowserLandUseUI;

    fn from_raw(id: RawID) -> Self {
        BrowserLandUseUIID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl BrowserLandUseUIID {
    pub fn spawn(world: &mut World) -> Self {
        let id = BrowserLandUseUIID::from_raw(world.allocate_instance_id::<BrowserLandUseUI>());
        let swarm = world.local_broadcast::<BrowserLandUseUI>();
        world.send(swarm, MSG_BrowserLandUseUI_spawn(id, ));
        id
    }
}

#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_BrowserLandUseUI_spawn(pub BrowserLandUseUIID, );

impl Into<ConfigUserID<ArchitectureRule>> for BrowserLandUseUIID {
    fn into(self) -> ConfigUserID<ArchitectureRule> {
        ConfigUserID::from_raw(self.as_raw())
    }
}

impl Into<LandUseUIID> for BrowserLandUseUIID {
    fn into(self) -> LandUseUIID {
        LandUseUIID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    ConfigUserID::<ArchitectureRule>::register_implementor::<BrowserLandUseUI>(system);
    LandUseUIID::register_implementor::<BrowserLandUseUI>(system);
    system.add_spawner::<BrowserLandUseUI, _, _>(
        |&MSG_BrowserLandUseUI_spawn(id, ), world| {
            BrowserLandUseUI::spawn(id, world)
        }, false
    );
}