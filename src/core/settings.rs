use core::ui::{KeyOrButton, KeyCombination, Mouse};
use monet::glium::glutin::{MouseButton, VirtualKeyCode};
use kay::ID;

use serde_json;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::collections::hash_map::HashMap;
use std::sync::Arc;
use std::vec::Vec;

use app_dirs;

pub static mut SETTINGS: Option<Arc<Settings>> = None;

pub struct KeyAction {
    action_id: usize,
}

pub struct MouseAction {
    action_id: usize,
    mouse: Mouse,
}

enum Action{
    Key(KeyAction),
    Mouse(MouseAction),
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Settings {
    // Controls
    pub rotation_speed: f32,
    pub move_speed: f32,
    pub zoom_speed: f32,
    pub invert_y: bool,

    pub key_triggers: HashMap<KeyCombination, usize>,
    pub mouse_triggers: HashMap<KeyCombination, usize>,
}

impl Settings {
    pub fn new() -> Settings {
        Settings {
            rotation_speed: 1.0f32,
            zoom_speed: 1.0f32,
            move_speed: 1.0f32,
            invert_y: false,

            key_triggers: HashMap::new(),
        }
    }

    fn initialize() -> Arc<Settings> {
        if SETTINGS == None {
            SETTINGS = Some(Arc::new(Settings::new()));
        }
        SETTINGS.unwrap().clone()
    }

    pub fn register_key(trigger: KeyCombination) -> usize {
        let settings = Settings::initialize();

        let id = settings.key_triggers.len();
        self.key_triggers.insert(trigger, id);
        id
    }

    pub fn register_mouse(trigger: KeyCombination) -> usize {
        let settings = Settings::initialize();

        let id = settings.mouse_triggers.len();
        self.mouse_triggers.insert(trigger, id);
        id
    }

    pub fn send(id: ID, keys: &Vec<KeyOrButton>, mouse: &Vec<Mouse>) {
        let settings = SETTINGS.expect("Global settings uninitialized").clone();
        for (comb, action_id) in settings.key_triggers {
            if comb_intersection(keys, comb) {
                id << Action::Key(KeyAction{action_id: action_id})
            }
        }
        for (comb, action_id) in settings.key_triggers {
            if comb_intersection(keys, comb) {
                for m in mouse {
                    id << Action::Mouse(MouseAction {action_id: action_id, mouse: m})
                }
            }
        }
    }

    fn intersection(a: &Vec<KeyOrButton>, b: &Vec<KeyOrButton>) -> bool {
        for k1 in a {
            for k2 in a {
                if k2 == k1 {
                    return true;
                }
            }
        }
        false
    }

    fn comb_intersection(keys: &Vec<KeyOrButton>, comb: KeyCombination) -> bool {
        let hit = true;
        for interchangeable_keys in KeyCombination.keys {
            hit = hit && Settings::intersection(interchangeable_keys, keys);
        }
        hit
    }

    pub fn load() -> Settings {
        let path = app_dirs::app_root(app_dirs::AppDataType::UserConfig, &::APP_INFO)
            .unwrap()
            .join("config.json");
        let display = path.display();
        let settings = Settings::new();

        let mut file = match File::open(&path) {
            Err(_) => {
                println!("Config file does not exist, creating new one");
                match File::create(&path) {
                    Err(why) => panic!("couldn't create {}: {}", display, why.description()),
                    Ok(mut file) => {
                        let serialized = serde_json::to_string(&settings)
                            .expect("Could not serialise Settings");
                        match file.write_all(serialized.as_bytes()) {
                            Err(why) => {
                                panic!("couldn't write config to {}: {}",
                                       display,
                                       why.description())
                            }
                            Ok(_) => println!("successfully wrote new config to {}", display),
                        }
                        file = match File::open(&path) {
                            Err(why) => {
                                panic!("couldn't open {}, which was just written to: {}",
                                       display,
                                       why.description())
                            }
                            Ok(file) => file,
                        };
                        file
                    }
                }
            }
            Ok(file) => file,
        };
        let mut s = String::new();
        if let Err(why) = file.read_to_string(&mut s) {
            panic!("couldn't read {}: {}", display, why.description())
        }
        serde_json::from_str::<Settings>(&s).unwrap()
    }

    pub fn get_rotation_speed() -> f32 {
        let settings = Settings::initialize();
        settings.rotation_speed
    }

    pub fn get_zoom_speed() -> f32 {
        let settings = Settings::initialize();
        settings.zoom_speed
    }

    pub fn get_move_speed() -> f32 {
        let settings = Settings::initialize();
        settings.move_speed
    }

    pub fn get_invert_y() -> bool {
        let settings = Settings::initialize();
        settings.invert_y
    }
}
