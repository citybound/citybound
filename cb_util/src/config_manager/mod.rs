use kay::{World};
use compact::{CHashMap, Compact};
use arrayvec::ArrayString;

#[derive(Compact, Clone)]
pub struct ConfigManager<Config: Compact + 'static> {
    id: ConfigManagerID<Config>,
    entries: CHashMap<ArrayString<[u8; 16]>, Config>
}

impl<Config: Compact + 'static> ConfigManager<Config> {
    pub fn init(id: ConfigManagerID<Config>, _: &mut World) -> ConfigManager<Config> {
        ConfigManager {
            id,
            entries: CHashMap::new()
        }
    }
}

mod kay_auto;
pub use self::kay_auto::*;