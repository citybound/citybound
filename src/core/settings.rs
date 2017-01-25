use core::ui::{KeyOrButton, KeyCombination, Mouse};
use core::user_interface::Event3d;
use monet::glium::glutin::{MouseButton, VirtualKeyCode};
use kay::ID;

use serde_json;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::collections::hash_map::HashMap;
use std::sync::RwLock;
use std::vec::Vec;

use app_dirs;

lazy_static!{
    pub static ref SETTINGS: RwLock<Settings> = RwLock::new(Settings::new());
}

#[derive(Clone, Debug, Copy)]
pub struct KeyAction {
    pub action_id: usize,
}

#[derive(Clone, Debug, Copy)]
pub struct MouseAction {
    pub action_id: usize,
    pub mouse: Mouse,
}

#[derive(Clone, Debug, Copy)]
pub enum Action{
    KeyHeld(KeyAction),
    KeyDown(KeyAction),
    KeyUp(KeyAction),
    Mouse(MouseAction),

    Event3d(Event3d),
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Settings {
    // Controls
    pub rotation_speed: f32,
    pub move_speed: f32,
    pub zoom_speed: f32,
    pub invert_y: bool,

    pub key_triggers: Vec<(KeyCombination, usize)>,
    pub key_excludes: Vec<(usize, usize)>,
    pub mouse_triggers: Vec<(KeyCombination, usize)>,
}

impl Settings {
    pub fn new() -> Settings {
        Settings {
            rotation_speed: 1.0f32,
            zoom_speed: 1.0f32,
            move_speed: 1.0f32,
            invert_y: false,

            key_triggers: Vec::new(),
            mouse_triggers: Vec::new(),
            key_excludes: Vec::new()
        }
    }

    pub fn register_key(trigger: KeyCombination) -> usize {
        let mut settings = SETTINGS.write().unwrap();
        let id = settings.key_triggers.len();
        settings.key_triggers.push((trigger, id));
        id
    }

    pub fn register_exclusiveness(a: usize, b: usize) {
        let mut settings = SETTINGS.write().unwrap();
        settings.key_excludes.push((a, b));
    }

    pub fn filter_events(events: &mut Vec<usize>, settings: &mut Settings) {
        let mut exc = Vec::<usize>::new();
        for i in events.clone() {
            for e in &settings.key_excludes {
                if e.0 == i {
                    exc.push(e.1)
                }
            }
        }
        events.retain(|x| !exc.contains(x));
    }

    pub fn register_mouse(trigger: KeyCombination) -> usize {
        let mut settings = SETTINGS.write().unwrap();
        let id = settings.mouse_triggers.len();
        settings.mouse_triggers.push((trigger, id));
        id
    }

    pub fn send_helper(keys: &Vec<KeyOrButton>, settings: &mut Settings) -> Vec<usize> {
        let mut ret = Vec::<usize>::new();
        for tup in &settings.key_triggers {
            if Settings::comb_intersection(keys, tup.0.clone()) {
                ret.push(tup.1)
            }
        }
        ret.clone()
    }

    pub fn send(id: ID, keys: &Vec<KeyOrButton>, new_keys: &Vec<KeyOrButton>, mouse: &Vec<Mouse>) {
        let mut settings = SETTINGS.write().unwrap();
        let mut total = Vec::<KeyOrButton>::new();
        total.extend(keys);
        total.extend(new_keys);

        let mut all_events = Settings::send_helper(&total, &mut settings);
        //println!("All events: {:?}", all_events);
        Settings::filter_events(&mut all_events, &mut settings);
        //println!("All filtered events: {:?}", all_events);
        let mut original_events = Settings::send_helper(&keys, &mut settings);
        //println!("All original events{:?}", original_events);
        Settings::filter_events(&mut original_events, &mut settings);
        //println!("All filtered events{:?}", original_events);
        let mut new_events = Vec::<usize>::new();
        for i in &all_events {
            if !original_events.contains(i) {
                new_events.push(*i)
            }
        }

        for i in &all_events {
            id << Action::KeyHeld(KeyAction { action_id: *i});
        }
        for i in &new_events {
            id << Action::KeyDown(KeyAction { action_id: *i});
        }

        for tup in &settings.mouse_triggers {
            if Settings::comb_intersection(keys, tup.0.clone()) {
                for &m in mouse {
                    id << Action::Mouse(MouseAction { action_id: tup.1, mouse: m })
                }
            }
        }
    }

    fn intersection(a: &Vec<KeyOrButton>, b: &Vec<KeyOrButton>) -> bool {
        for k1 in a {
            for k2 in b {
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
        let settings = SETTINGS.read().unwrap();
        settings.rotation_speed
    }

    pub fn get_zoom_speed() -> f32 {
        let settings = SETTINGS.read().unwrap();
        settings.zoom_speed
    }

    pub fn get_move_speed() -> f32 {
        let settings = SETTINGS.read().unwrap();
        settings.move_speed
    }

    pub fn get_invert_y() -> f32 {
        let settings = SETTINGS.read().unwrap();
        if settings.invert_y { -1.0f32 } else { 1.0f32 }
    }
}
