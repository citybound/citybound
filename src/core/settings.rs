use ::core::ui::KeyOrButton;
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

    #[serde(default = "Settings::default_mouse_main")]
    pub mouse_main: Vec<KeyOrButton>,

    #[serde(default = "Settings::default_forward_key")]
    pub forward_key: Vec<KeyOrButton>,
    #[serde(default = "Settings::default_backward_key")]
    pub backward_key: Vec<KeyOrButton>,
    #[serde(default = "Settings::default_left_key")]
    pub left_key: Vec<KeyOrButton>,
    #[serde(default = "Settings::default_right_key")]
    pub right_key: Vec<KeyOrButton>,

    #[serde(default = "Settings::default_pan_modifier")]
    pub pan_modifier_key: Vec<KeyOrButton>,
    #[serde(default = "Settings::default_yaw_modifier")]
    pub yaw_modifier_key: Vec<KeyOrButton>,
    #[serde(default = "Settings::default_pitch_modifier")]
    pub pitch_modifier_key: Vec<KeyOrButton>,

    #[serde(default = "Settings::default_undo_key")]
    pub undo_key: Vec<KeyOrButton>,
    #[serde(default = "Settings::default_undo_modifier")]
    pub undo_modifier_key: Vec<KeyOrButton>,
    #[serde(default = "Settings::default_undo_to_redo_modifier")]
    pub redo_modifier_key: Vec<KeyOrButton>,

    #[serde(default = "Settings::default_car_spawn_key")]
    pub car_spawning_key: Vec<KeyOrButton>,
    #[serde(default = "Settings::default_delete_selection_key")]
    pub delete_selection_key: Vec<KeyOrButton>,

    #[serde(default = "Settings::default_finalize_key")]
    pub finalize_key: Vec<KeyOrButton>,

    #[serde(default = "Settings::default_grid_key")]
    pub grid_key: Vec<KeyOrButton>,
    #[serde(default = "Settings::default_grid_modifier")]
    pub grid_modifier_key: Vec<KeyOrButton>,
}

impl Settings{
    pub fn new() -> Settings{
        Settings{
            rotation_speed: Settings::default_rotation_speed(),
            zoom_speed: Settings::default_zoom_speed(),
            move_speed: Settings::default_move_speed(),
            invert_y: Settings::default_invert_y(),

            mouse_main: Settings::default_mouse_main(),
            forward_key: Settings::default_forward_key(),
            backward_key: Settings::default_backward_key(),
            left_key: Settings::default_left_key(),
            right_key: Settings::default_right_key(),

            pan_modifier_key: Settings::default_pan_modifier(),
            yaw_modifier_key: Settings::default_yaw_modifier(),
            pitch_modifier_key: Settings::default_pitch_modifier(),

            undo_key: Settings::default_undo_key(),
            undo_modifier_key: Settings::default_undo_modifier(),
            redo_modifier_key: Settings::default_undo_to_redo_modifier(),

            finalize_key: Settings::default_finalize_key(),

            delete_selection_key: Settings::default_delete_selection_key(),
            car_spawning_key: Settings::default_car_spawn_key(),

            grid_key: Settings::default_grid_key(),
            grid_modifier_key: Settings::default_grid_modifier(),
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

    fn default_mouse_main() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Button(MouseButton::Left)]
    }

    fn default_forward_key() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Key(VirtualKeyCode::W), KeyOrButton::Key(VirtualKeyCode::Up)]
    }

    fn default_backward_key() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Key(VirtualKeyCode::S), KeyOrButton::Key(VirtualKeyCode::Down)]
    }

    fn default_left_key() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Key(VirtualKeyCode::A), KeyOrButton::Key(VirtualKeyCode::Left)]
    }

    fn default_right_key() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Key(VirtualKeyCode::D), KeyOrButton::Key(VirtualKeyCode::Right)]
    }

    fn default_pan_modifier() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Key(VirtualKeyCode::LShift), KeyOrButton::Key(VirtualKeyCode::RShift)]
    }

    fn default_yaw_modifier() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Button(MouseButton::Middle), KeyOrButton::Key(VirtualKeyCode::LAlt), KeyOrButton::Key(VirtualKeyCode::RAlt)]
    }

    fn default_pitch_modifier() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Button(MouseButton::Middle), KeyOrButton::Key(VirtualKeyCode::LAlt), KeyOrButton::Key(VirtualKeyCode::RAlt)]
    }

    fn default_undo_key() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Key(VirtualKeyCode::Z)]
    }

    fn default_undo_modifier() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Key(VirtualKeyCode::LControl), KeyOrButton::Key(VirtualKeyCode::RControl), KeyOrButton::Key(VirtualKeyCode::LWin), KeyOrButton::Key(VirtualKeyCode::RWin)]
    }

    fn default_undo_to_redo_modifier() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Key(VirtualKeyCode::LShift), KeyOrButton::Key(VirtualKeyCode::RShift)]
    }

    fn default_delete_selection_key() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Key(VirtualKeyCode::Escape), KeyOrButton::Key(VirtualKeyCode::Back)]
    }

    fn default_finalize_key() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Key(VirtualKeyCode::Return)]
    }

    fn default_car_spawn_key() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Key(VirtualKeyCode::C)]
    }

    fn default_grid_key() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Key(VirtualKeyCode::C)]
    }

    fn default_grid_modifier() -> Vec<KeyOrButton>{
        vec![KeyOrButton::Key(VirtualKeyCode::C)]
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