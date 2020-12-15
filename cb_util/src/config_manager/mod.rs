use kay::{World, Actor, TypedID};
use compact::{CHashMap, COption, CString, Compact};
use arrayvec::ArrayString;
use serde::de::DeserializeOwned;

pub type Name = ArrayString<[u8; 16]>;
pub trait Config: Compact + 'static {}

#[derive(Compact, Clone)]
pub struct ConfigManager<C: Config> {
    id: ConfigManagerID<C>,
    entries: CHashMap<Name, C>,
}

impl<C: Config> ConfigManager<C> {
    pub fn spawn(
        id: ConfigManagerID<C>,
        initial_entries: &CHashMap<Name, C>,
        _: &mut World,
    ) -> ConfigManager<C> {
        ConfigManager {
            id,
            entries: initial_entries.clone(),
        }
    }

    pub fn request_current(&self, requester: ConfigUserID<C>, world: &mut World) {
        for (name, value) in self.entries.pairs() {
            requester.on_config_change(*name, COption(Some(value.clone())), world);
        }
    }

    pub fn update_entry(&mut self, name: Name, maybe_value: &COption<C>, world: &mut World) {
        if let COption(Some(ref value)) = *maybe_value {
            ConfigUserID::<C>::global_broadcast(world).on_config_change(
                name,
                COption(Some(value.clone())),
                world,
            );
            self.entries.insert(name, value.clone());
        } else {
            ConfigUserID::<C>::global_broadcast(world).on_config_change(name, COption(None), world);
            self.entries.remove(name);
        }
    }

    pub fn update_all_entries(&mut self, entries: &CHashMap<Name, C>, world: &mut World) {
        // TODO: handle disappearing entries?
        self.entries = entries.clone();
        for (name, value) in self.entries.pairs() {
            ConfigUserID::<C>::global_broadcast(world).on_config_change(
                name.clone(),
                COption(Some(value.clone())),
                world,
            );
        }
    }
}

use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    sync::mpsc::{Receiver, channel},
};
use cb_time::actors::{Temporal, TemporalID};
use cb_time::units::{Instant};
#[cfg(feature = "server")]
use notify::Watcher;

#[cfg(feature = "server")]
struct ConfigFileWatcherState {
    receiver: Receiver<notify::DebouncedEvent>,
    watcher: notify::RecommendedWatcher,
}

#[derive(Compact, Clone)]
pub struct ConfigFileWatcher<CD: Config + DeserializeOwned> {
    id: ConfigFileWatcherID<CD>,
    target: ConfigManagerID<CD>,
    file: CString,
    #[cfg(feature = "server")]
    state: kay::External<Option<ConfigFileWatcherState>>,
}

impl<CD: Config + DeserializeOwned> ConfigFileWatcher<CD> {
    pub fn spawn(
        id: ConfigFileWatcherID<CD>,
        target: ConfigManagerID<CD>,
        file: &CString,
        _: &mut World,
    ) -> ConfigFileWatcher<CD> {
        ConfigFileWatcher {
            id,
            target,
            file: file.clone(),
            #[cfg(feature = "server")]
            state: kay::External::new(None),
        }
    }

    pub fn reload(&mut self, world: &mut World) {
        #[cfg(feature = "server")]
        {
            let file = File::open(&*(self.file))
                .expect(&format!("Couldn't find config file {:?}", &*self.file));
            let reader = BufReader::new(file);
            let new_entries: HashMap<Name, CD> =
                serde_yaml::from_reader(reader).expect("parsing failed");
            self.target
                .update_all_entries(new_entries.into_iter().collect(), world);
        }
    }
}

impl<CD: Config + DeserializeOwned> Temporal for ConfigFileWatcher<CD> {
    fn tick(&mut self, _dt: f32, _current_instant: Instant, world: &mut World) {
        #[cfg(feature = "server")]
        {
            let file_path = (*self.file).to_owned();
            if self.state.is_none() {
                self.reload(world);
            };
            let state = self.state.get_or_insert_with(|| {
                let (tx, rx) = channel();
                let mut watcher = notify::watcher(tx, std::time::Duration::from_secs(1)).unwrap();
                watcher
                    .watch(&file_path, notify::RecursiveMode::Recursive)
                    .unwrap();
                println!("Started watching config file . {:?}", &file_path);

                ConfigFileWatcherState {
                    receiver: rx,
                    watcher,
                }
            });

            match state.receiver.try_recv() {
                Ok(event) => {
                    println!("Config file updated. {:?} {:?}", &file_path, event);
                    self.reload(world);
                }
                Err(_e) => {}
            }
        }
    }
}

pub trait ConfigUser<C: Config>: Actor {
    fn local_cache(&mut self) -> &mut CHashMap<Name, C>;
    fn apply_config_change(&mut self, name: Name, maybe_value: &COption<C>, _: &mut World) {
        if let COption(Some(ref value)) = *maybe_value {
            self.local_cache().insert(name, value.clone());
        } else {
            self.local_cache().remove(name);
        }
    }
    fn on_config_change(&mut self, name: Name, maybe_value: &COption<C>, world: &mut World) {
        self.apply_config_change(name, maybe_value, world);
    }
    fn get_initial_config(&self, world: &mut World) {
        ConfigManagerID::<C>::global_first(world).request_current(self.id_as(), world);
    }
}

mod kay_auto;
pub use self::kay_auto::*;
