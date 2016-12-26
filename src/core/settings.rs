use ::monet::glium::glutin::{Event, MouseScrollDelta, ElementState, MouseButton};

struct Settings {
    //Video settings
    resolution: (i32, i32),
    vsync: bool,

    //Controls
    rotation_speed: f32,
    zoom_speed: f32,
    invert_y: bool,

    mouse1: MouseButton,
    mouse2: MouseButton,
    mouse3: MouseButton,

    forward_key: VirtualKeyCode,
    backward_key: VirtualKeyCode,
    left_key: VirtualKeyCode,
    right_key: VirtualKeyCode,
    pan_modifier_key: VirtualKeyCode,

    //Behavioural - Cars
    car_length: f32,
    acceleration: f32,
    max_deceleration: f32,
    acceleration_exponent: f32,
    minimum_spacing: f32,
}