use std::fs::File;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

pub struct Environment {
    pub name: &'static str,
    pub version: &'static str,
    pub author: &'static str,
}

impl Environment {
    fn setting_path(&self, category: &str) -> PathBuf {
        let app_info = ::app_dirs::AppInfo {
            name: self.name,
            author: self.author,
        };
        ::app_dirs::app_root(::app_dirs::AppDataType::UserConfig, &app_info)
            .expect("Expected settings dir to exist")
            .join(self.version)
            .join([category, ".json"].concat())
    }

    pub fn load_settings<S>(&self, category: &str) -> S
        where for<'a> S: Deserialize<'a> + Default
    {
        match File::open(self.setting_path(category))
                  .as_mut()
                  .map_err(|err| format!("{}", err))
                  .and_then(|file| {
                                ::serde_json::from_reader(file).map_err(|err| format!("{}", err))
                            }) {
            Ok(settings) => settings,
            Err(err) => {
                println!("Error loading {} settings: {}", category, err);
                S::default()
            }
        }
    }

    pub fn write_settings<S>(&self, category: &str, settings: &S)
        where S: Serialize + Default
    {
        if let Err(err) = File::open(self.setting_path(category))
            .as_mut()
            .map_err(|err| format!("{}", err))
            .and_then(|file| {
                ::serde_json::to_writer_pretty(file, settings).map_err(|err| format!("{}", err))
            }) {
            println!("Error writing {} settings: {}", category, err);
        }
    }
}
