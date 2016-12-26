#![feature(custom_derive)]
use ::core::ui::KeyOrButton;

#[derive(Serialize, Deserialize, Debug)]
struct Settings {
    //Controls
    rotation_speed: f32,
    zoom_speed: f32,
    invert_y: bool,

    mouse_main: Vec<KeyOrButton>,
    
    forward_key: Vec<KeyOrButton>,
    backward_key: Vec<KeyOrButton>,
    left_key: Vec<KeyOrButton>,
    right_key: Vec<KeyOrButton>,
    pan_modifier_key: Vec<KeyOrButton>,
}