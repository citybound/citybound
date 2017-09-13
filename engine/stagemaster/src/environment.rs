use std::fs::{File, OpenOptions, create_dir_all};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Copy, Clone)]
pub struct Environment {
    pub name: &'static str,
    pub version: &'static str,
    pub author: &'static str,
}

impl Environment {
    fn setting_dir(&self) -> PathBuf {
        let app_info = ::app_dirs::AppInfo { name: self.name, author: self.author };
        ::app_dirs::app_root(::app_dirs::AppDataType::UserConfig, &app_info)
            .expect("Expected settings dir to exist")
            .join(self.version)
    }
    fn setting_path(&self, category: &str) -> PathBuf {
        self.setting_dir().join([category, ".json"].concat())
    }

    pub fn load_settings<S>(&self, category: &str) -> S
    where
        for<'a> S: Deserialize<'a> + Default,
    {
        if let Err(err) = create_dir_all(self.setting_dir()) {
            println!(
                "Error creating settings dir {:?} {}",
                self.setting_dir(),
                err
            );
        };
        match File::open(self.setting_path(category))
            .as_mut()
            .map_err(|err| format!("{}", err))
            .and_then(|file| {
                ::serde_json::from_reader(file).map_err(|err| format!("{}", err))
            }) {
            Ok(settings) => settings,
            Err(err) => {
                println!(
                    "Error loading {} settings: {} from {:?}",
                    category,
                    err,
                    self.setting_path(category)
                );
                S::default()
            }
        }
    }

    pub fn write_settings<S>(&self, category: &str, settings: &S)
    where
        S: Serialize + Default,
    {
        if let Err(err) = OpenOptions::new()
            .write(true)
            .create(true)
            .open(self.setting_path(category))
            .as_mut()
            .map_err(|err| format!("{}", err))
            .and_then(|file| {
                ::serde_json::to_writer_pretty(file, settings).map_err(|err| format!("{}", err))
            })
        {
            println!(
                "Error writing {} settings: {} to {:?}",
                category,
                err,
                self.setting_path(category)
            );
        } else {
            println!("Write {} settings", category);
        }
    }
}
