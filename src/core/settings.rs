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

//TODO: FIX THIS UGLY AND UNSAFE HACK
pub static mut SETTINGS: *mut Settings = 0 as *mut Settings;

#[derive(Compact, Clone)]
pub struct KeyAction {
    pub action_id: usize,
}

#[derive(Compact, Clone)]
pub struct MouseAction {
    pub action_id: usize,
    pub mouse: Mouse,
}

#[derive(Compact, Clone)]
pub enum Action{
    KeyHeld(KeyAction),
    KeyDown(KeyAction),
    KeyUp(KeyAction),
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
            mouse_triggers: HashMap::new(),
        }
    }

    pub fn initialize() {
        unsafe {
            let mut settings = Box::new(Settings::new());
            SETTINGS = &mut *settings as *mut Settings;
        }
    }

    pub fn register_key(trigger: KeyCombination) -> usize {
        unsafe {
            let id = (*SETTINGS).key_triggers.len();
            (*SETTINGS).key_triggers.insert(trigger, id);
            id
        }
    }

    pub fn register_mouse(trigger: KeyCombination) -> usize {
        unsafe {
            let id = (*SETTINGS).mouse_triggers.len();
            (*SETTINGS).mouse_triggers.insert(trigger, id);
            id
        }
    }

    pub fn send(id: ID, keys: &Vec<KeyOrButton>, mouse: &Vec<Mouse>) {
        unsafe {
            for (comb, action_id) in &(*SETTINGS).key_triggers {
                if Settings::comb_intersection(keys, (*comb).clone()) {
                    id << Action::KeyHeld(KeyAction { action_id: *action_id })
                }
            }

            for (comb, action_id) in &(*SETTINGS).key_triggers {
                if Settings::comb_intersection(keys, (*comb).clone()) {
                    for &m in mouse {
                        id << Action::Mouse(MouseAction { action_id: *action_id, mouse: m })
                    }
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
        let mut hit = true;
        for interchangeable_keys in comb.keys {
            hit = hit && Settings::intersection(&interchangeable_keys, keys);
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
        unsafe {
            (*SETTINGS).rotation_speed
        }
    }

    pub fn get_zoom_speed() -> f32 {
        unsafe {
            (*SETTINGS).zoom_speed
        }
    }

    pub fn get_move_speed() -> f32 {
        unsafe {
            (*SETTINGS).move_speed
        }
    }

    pub fn get_invert_y() -> f32 {
        unsafe {
            if (*SETTINGS).invert_y { -1.0f32 } else { 1.0f32 }
        }
    }
}
