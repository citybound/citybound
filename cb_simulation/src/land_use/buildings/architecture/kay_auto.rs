//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl<Config: Compact + 'static> Actor for ConfigManager<Config> {
    type ID = ConfigManagerID<Config>;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct ConfigManagerID<Config: Compact + 'static> {
    _raw_id: RawID, _marker: ::std::marker::PhantomData<Box<(Config)>>
}

impl<Config: Compact + 'static> Copy for ConfigManagerID<Config> {}
impl<Config: Compact + 'static> Clone for ConfigManagerID<Config> { fn clone(&self) -> Self { *self } }
impl<Config: Compact + 'static> ::std::fmt::Debug for ConfigManagerID<Config> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "ConfigManagerID<Config>({:?})", self._raw_id)
    }
}
impl<Config: Compact + 'static> ::std::hash::Hash for ConfigManagerID<Config> {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl<Config: Compact + 'static> PartialEq for ConfigManagerID<Config> {
    fn eq(&self, other: &ConfigManagerID<Config>) -> bool {
        self._raw_id == other._raw_id
    }
}
impl<Config: Compact + 'static> Eq for ConfigManagerID<Config> {}

impl<Config: Compact + 'static> TypedID for ConfigManagerID<Config> {
    type Target = ConfigManager<Config>;

    fn from_raw(id: RawID) -> Self {
        ConfigManagerID { _raw_id: id, _marker: ::std::marker::PhantomData }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<Config: Compact + 'static> ConfigManagerID<Config> {
    pub fn init(world: &mut World) -> Self {
        let id = ConfigManagerID::<Config>::from_raw(world.allocate_instance_id::<ConfigManager<Config>>());
        let swarm = world.local_broadcast::<ConfigManager<Config>>();
        world.send(swarm, MSG_ConfigManager_init(id, ));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ConfigManager_init<Config: Compact + 'static>(pub ConfigManagerID<Config>, );


impl Actor for ArchitectureRuleManager {
    type ID = ArchitectureRuleManagerID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct ArchitectureRuleManagerID {
    _raw_id: RawID
}

impl Copy for ArchitectureRuleManagerID {}
impl Clone for ArchitectureRuleManagerID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for ArchitectureRuleManagerID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "ArchitectureRuleManagerID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for ArchitectureRuleManagerID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for ArchitectureRuleManagerID {
    fn eq(&self, other: &ArchitectureRuleManagerID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for ArchitectureRuleManagerID {}

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
pub fn auto_setup<Config: Compact + 'static>(system: &mut ActorSystem) {
    
    
    system.add_spawner::<ConfigManager<Config>, _, _>(
        |&MSG_ConfigManager_init::<Config>(id, ), world| {
            ConfigManager::<Config>::init(id, world)
        }, false
    );
    
    system.add_spawner::<ArchitectureRuleManager, _, _>(
        |&MSG_ArchitectureRuleManager_init(id, ), world| {
            ArchitectureRuleManager::init(id, world)
        }, false
    );
}