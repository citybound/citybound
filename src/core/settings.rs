use ::core::ui::KeyOrButton;
use ::monet::glium::glutin::{MouseButton, VirtualKeyCode};
use serde_json;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;

use app_dirs;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct Settings {
    //Controls
    pub rotation_speed: f32,
    pub move_speed: f32,
    pub zoom_speed: f32,
    pub invert_y: bool,

    pub mouse_main: Vec<KeyOrButton>,
    
    pub forward_key: Vec<KeyOrButton>,
    pub backward_key: Vec<KeyOrButton>,
    pub left_key: Vec<KeyOrButton>,
    pub right_key: Vec<KeyOrButton>,
    pub pan_modifier_key: Vec<KeyOrButton>,
    pub rotate_modifier_key: Vec<KeyOrButton>,
}

impl Settings{
    pub fn new() -> Settings{
        Settings{
            rotation_speed: 1.0f32,
            zoom_speed: 1.0f32,
            move_speed: 1.0f32,
            invert_y: false,

            mouse_main: vec![KeyOrButton::Button(MouseButton::Left)],
            forward_key: vec![KeyOrButton::Key(VirtualKeyCode::W), KeyOrButton::Key(VirtualKeyCode::Up)],
            backward_key: vec![KeyOrButton::Key(VirtualKeyCode::S), KeyOrButton::Key(VirtualKeyCode::Down)],
            left_key: vec![KeyOrButton::Key(VirtualKeyCode::A), KeyOrButton::Key(VirtualKeyCode::Left)],
            right_key: vec![KeyOrButton::Key(VirtualKeyCode::D), KeyOrButton::Key(VirtualKeyCode::Right)],

            pan_modifier_key:    vec![KeyOrButton::Key(VirtualKeyCode::LShift), KeyOrButton::Key(VirtualKeyCode::RShift)],
            rotate_modifier_key: vec![KeyOrButton::Button(MouseButton::Middle), KeyOrButton::Key(VirtualKeyCode::LAlt), KeyOrButton::Key(VirtualKeyCode::RAlt)],
        }
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
        serde_json::from_str::<Settings>(&s).unwrap()
    }
}