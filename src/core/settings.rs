use ::core::ui::{KeyCombination};
use ::monet::glium::glutin::{MouseButton, VirtualKeyCode};
use serde_json;

use std::error::Error;
use std::fs::{File, remove_file};
use std::io::prelude::*;

use app_dirs;



#[derive(Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Settings {
    //Controls
    #[serde(default = "Settings::default_rotation_speed")]
    pub rotation_speed: f32,

    #[serde(default = "Settings::default_move_speed")]
    pub move_speed: f32,

    #[serde(default = "Settings::default_zoom_speed")]
    pub zoom_speed: f32,

    #[serde(default = "Settings::default_invert_y")]
    pub invert_y: bool,

    pub key_mappings: Vec<(KeyCombination, &'static str)>,
    pub mouse_modifier_mappings: Vec<(KeyCombination, &'static str)>,
}

impl Settings{
    pub fn new() -> Settings{
        Settings{
            rotation_speed: Settings::default_rotation_speed(),
            zoom_speed: Settings::default_zoom_speed(),
            move_speed: Settings::default_move_speed(),
            invert_y: Settings::default_invert_y(),

            key_mappings: Vec::new(),
            mouse_modifier_mappings: Vec::new(),
        }
    }

    fn default_rotation_speed() -> f32{
        1.0f32
    }

    fn default_zoom_speed() -> f32{
        1.0f32
    }

    fn default_move_speed() -> f32{
        1.0f32
    }

    fn default_invert_y() -> bool{
        false
    }

    pub fn register_key(&mut self, keys: KeyCombination, name: String){
        self.key_mappings.push((keys, name))
    }

    pub fn register_mouse_modifier(&mut self, keys: KeyCombination, name: String){
        self.mouse_modifier_mappings.push((keys, name))
    }

    pub fn load() -> Settings{
        let path = app_dirs::app_root(app_dirs::AppDataType::UserConfig, &::APP_INFO).unwrap().join("config.json");
        let display = path.display();
        let settings = Settings::new();

        let mut file = match File::open(&path) {
            Err(_) => {
                println!("Config file does not exist, creating new one");
                match File::create(&path) {
                    Err(why) => panic!("couldn't create {}: {}", display, why.description()),
                    Ok(mut file) => {
                        let serialized = serde_json::to_string(&settings).expect("Could not serialise Settings");
                        match file.write_all(serialized.as_bytes()) {
                            Err(why) => {
                                panic!("couldn't write config to {}: {}", display, why.description())
                            },
                            Ok(_) => println!("successfully wrote new config to {}", display),
                        }
                        file = match File::open(&path) {
                            Err(why) => panic!("couldn't open {}, which was just written to: {}", display, why.description()),
                            Ok(file) => file,
                        };
                        file
                    },
                }
            }
            Ok(file) => file,
        };
        let mut s = String::new();
        if let Err(why) = file.read_to_string(&mut s) {
            panic!("couldn't read {}: {}", display, why.description())
        }
        match serde_json::from_str::<Settings>(&s){
            Err(_) => {
                println!("Config file exists, but cannot be read, removing old config file and using default settings");
                remove_file(&path).expect("couldn't delete old config file");
                Settings::load()
            }
            Ok(s) => s
        }
    }
}