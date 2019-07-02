use kay::{World, Actor, TypedID};
use compact::{CHashMap, Compact, COption};
use arrayvec::ArrayString;

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
