//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct ConfigUserID<C: Config> {
    _raw_id: RawID, _marker: ::std::marker::PhantomData<Box<(C)>>
}

impl<C: Config> Copy for ConfigUserID<C> {}
impl<C: Config> Clone for ConfigUserID<C> { fn clone(&self) -> Self { *self } }
impl<C: Config> ::std::fmt::Debug for ConfigUserID<C> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "ConfigUserID<C>({:?})", self._raw_id)
    }
}
impl<C: Config> ::std::hash::Hash for ConfigUserID<C> {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl<C: Config> PartialEq for ConfigUserID<C> {
    fn eq(&self, other: &ConfigUserID<C>) -> bool {
        self._raw_id == other._raw_id
    }
}
impl<C: Config> Eq for ConfigUserID<C> {}

pub struct ConfigUserRepresentative<C: Config>{ _marker: ::std::marker::PhantomData<Box<(C)>> }

impl<C: Config> ActorOrActorTrait for ConfigUserRepresentative<C> {
    type ID = ConfigUserID<C>;
}

impl<C: Config> TypedID for ConfigUserID<C> {
    type Target = ConfigUserRepresentative<C>;

    fn from_raw(id: RawID) -> Self {
        ConfigUserID { _raw_id: id, _marker: ::std::marker::PhantomData }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<C: Config, Act: Actor + ConfigUser<C>> TraitIDFrom<Act> for ConfigUserID<C> {}

impl<C: Config> ConfigUserID<C> {
    pub fn apply_config_change(self, name: Name, maybe_value: COption < C >, world: &mut World) {
        world.send(self.as_raw(), MSG_ConfigUser_apply_config_change::<C>(name, maybe_value));
    }
    
    pub fn on_config_change(self, name: Name, maybe_value: COption < C >, world: &mut World) {
        world.send(self.as_raw(), MSG_ConfigUser_on_config_change::<C>(name, maybe_value));
    }
    
    pub fn get_initial_config(self, world: &mut World) {
        world.send(self.as_raw(), MSG_ConfigUser_get_initial_config());
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<ConfigUserRepresentative<C>>();
        system.register_trait_message::<MSG_ConfigUser_apply_config_change<C>>();
        system.register_trait_message::<MSG_ConfigUser_on_config_change<C>>();
        system.register_trait_message::<MSG_ConfigUser_get_initial_config>();
    }

    pub fn register_implementor<Act: Actor + ConfigUser<C>>(system: &mut ActorSystem) {
        system.register_implementor::<Act, ConfigUserRepresentative<C>>();
        system.add_handler::<Act, _, _>(
            |&MSG_ConfigUser_apply_config_change::<C>(name, ref maybe_value), instance, world| {
                instance.apply_config_change(name, maybe_value, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_ConfigUser_on_config_change::<C>(name, ref maybe_value), instance, world| {
                instance.on_config_change(name, maybe_value, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_ConfigUser_get_initial_config(), instance, world| {
                instance.get_initial_config(world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ConfigUser_apply_config_change<C: Config>(pub Name, pub COption < C >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ConfigUser_on_config_change<C: Config>(pub Name, pub COption < C >);
#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_ConfigUser_get_initial_config();

impl<C: Config> Actor for ConfigManager<C> {
    type ID = ConfigManagerID<C>;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct ConfigManagerID<C: Config> {
    _raw_id: RawID, _marker: ::std::marker::PhantomData<Box<(C)>>
}

impl<C: Config> Copy for ConfigManagerID<C> {}
impl<C: Config> Clone for ConfigManagerID<C> { fn clone(&self) -> Self { *self } }
impl<C: Config> ::std::fmt::Debug for ConfigManagerID<C> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "ConfigManagerID<C>({:?})", self._raw_id)
    }
}
impl<C: Config> ::std::hash::Hash for ConfigManagerID<C> {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl<C: Config> PartialEq for ConfigManagerID<C> {
    fn eq(&self, other: &ConfigManagerID<C>) -> bool {
        self._raw_id == other._raw_id
    }
}
impl<C: Config> Eq for ConfigManagerID<C> {}

impl<C: Config> TypedID for ConfigManagerID<C> {
    type Target = ConfigManager<C>;

    fn from_raw(id: RawID) -> Self {
        ConfigManagerID { _raw_id: id, _marker: ::std::marker::PhantomData }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<C: Config> ConfigManagerID<C> {
    pub fn spawn(initial_entries: CHashMap < Name , C >, world: &mut World) -> Self {
        let id = ConfigManagerID::<C>::from_raw(world.allocate_instance_id::<ConfigManager<C>>());
        let swarm = world.local_broadcast::<ConfigManager<C>>();
        world.send(swarm, MSG_ConfigManager_spawn::<C>(id, initial_entries));
        id
    }
    
    pub fn request_current(self, requester: ConfigUserID < C >, world: &mut World) {
        world.send(self.as_raw(), MSG_ConfigManager_request_current::<C>(requester));
    }
    
    pub fn update_entry(self, name: Name, maybe_value: COption < C >, world: &mut World) {
        world.send(self.as_raw(), MSG_ConfigManager_update_entry::<C>(name, maybe_value));
    }
    
    pub fn update_all_entries(self, entries: CHashMap < Name , C >, world: &mut World) {
        world.send(self.as_raw(), MSG_ConfigManager_update_all_entries::<C>(entries));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ConfigManager_spawn<C: Config>(pub ConfigManagerID<C>, pub CHashMap < Name , C >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ConfigManager_request_current<C: Config>(pub ConfigUserID < C >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ConfigManager_update_entry<C: Config>(pub Name, pub COption < C >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ConfigManager_update_all_entries<C: Config>(pub CHashMap < Name , C >);


impl<CD: Config + DeserializeOwned> Actor for ConfigFileWatcher<CD> {
    type ID = ConfigFileWatcherID<CD>;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct ConfigFileWatcherID<CD: Config + DeserializeOwned> {
    _raw_id: RawID, _marker: ::std::marker::PhantomData<Box<(CD)>>
}

impl<CD: Config + DeserializeOwned> Copy for ConfigFileWatcherID<CD> {}
impl<CD: Config + DeserializeOwned> Clone for ConfigFileWatcherID<CD> { fn clone(&self) -> Self { *self } }
impl<CD: Config + DeserializeOwned> ::std::fmt::Debug for ConfigFileWatcherID<CD> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "ConfigFileWatcherID<CD>({:?})", self._raw_id)
    }
}
impl<CD: Config + DeserializeOwned> ::std::hash::Hash for ConfigFileWatcherID<CD> {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl<CD: Config + DeserializeOwned> PartialEq for ConfigFileWatcherID<CD> {
    fn eq(&self, other: &ConfigFileWatcherID<CD>) -> bool {
        self._raw_id == other._raw_id
    }
}
impl<CD: Config + DeserializeOwned> Eq for ConfigFileWatcherID<CD> {}

impl<CD: Config + DeserializeOwned> TypedID for ConfigFileWatcherID<CD> {
    type Target = ConfigFileWatcher<CD>;

    fn from_raw(id: RawID) -> Self {
        ConfigFileWatcherID { _raw_id: id, _marker: ::std::marker::PhantomData }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<CD: Config + DeserializeOwned> ConfigFileWatcherID<CD> {
    pub fn spawn(target: ConfigManagerID < CD >, file: CString, world: &mut World) -> Self {
        let id = ConfigFileWatcherID::<CD>::from_raw(world.allocate_instance_id::<ConfigFileWatcher<CD>>());
        let swarm = world.local_broadcast::<ConfigFileWatcher<CD>>();
        world.send(swarm, MSG_ConfigFileWatcher_spawn::<CD>(id, target, file));
        id
    }
    
    pub fn reload(self, world: &mut World) {
        world.send(self.as_raw(), MSG_ConfigFileWatcher_reload());
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ConfigFileWatcher_spawn<CD: Config + DeserializeOwned>(pub ConfigFileWatcherID<CD>, pub ConfigManagerID < CD >, pub CString);
#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_ConfigFileWatcher_reload();

impl<CD: Config + DeserializeOwned> Into<TemporalID> for ConfigFileWatcherID<CD> {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup<C: Config, CD: Config + DeserializeOwned>(system: &mut ActorSystem) {
    ConfigUserID::<C>::register_trait(system);
    
    system.add_spawner::<ConfigManager<C>, _, _>(
        |&MSG_ConfigManager_spawn::<C>(id, ref initial_entries), world| {
            ConfigManager::<C>::spawn(id, initial_entries, world)
        }, false
    );
    
    system.add_handler::<ConfigManager<C>, _, _>(
        |&MSG_ConfigManager_request_current::<C>(requester), instance, world| {
            instance.request_current(requester, world); Fate::Live
        }, false
    );
    
    system.add_handler::<ConfigManager<C>, _, _>(
        |&MSG_ConfigManager_update_entry::<C>(name, ref maybe_value), instance, world| {
            instance.update_entry(name, maybe_value, world); Fate::Live
        }, false
    );
    
    system.add_handler::<ConfigManager<C>, _, _>(
        |&MSG_ConfigManager_update_all_entries::<C>(ref entries), instance, world| {
            instance.update_all_entries(entries, world); Fate::Live
        }, false
    );
    TemporalID::register_implementor::<ConfigFileWatcher<CD>>(system);
    system.add_spawner::<ConfigFileWatcher<CD>, _, _>(
        |&MSG_ConfigFileWatcher_spawn::<CD>(id, target, ref file), world| {
            ConfigFileWatcher::<CD>::spawn(id, target, file, world)
        }, false
    );
    
    system.add_handler::<ConfigFileWatcher<CD>, _, _>(
        |&MSG_ConfigFileWatcher_reload(), instance, world| {
            instance.reload(world); Fate::Live
        }, false
    );
}