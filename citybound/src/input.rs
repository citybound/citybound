use ::monet::glium::glutin::{Event, ElementState, VirtualKeyCode as Key, MouseScrollDelta};
use ::monet::glium;
use ::std::f32::consts::PI;

pub enum InputCommand {
    RotateEye(f32),
    TiltEye(f32),
    MoveEyeForwards(f32),
    MoveEyeSideways(f32),
    LetEyeApproach(f32)
}

pub enum InputResult {
    ContinueWithInputCommands(Vec<InputCommand>),
    Exit
}

#[derive(Default)]
pub struct InputState {
    rotating_eye_left: bool,
    rotating_eye_right: bool,
    tilting_eye_up: bool,
    tilting_eye_down: bool,
    moving_eye_forward: bool,
    moving_eye_backward: bool,
    moving_eye_left: bool,
    moving_eye_right: bool,
    eye_approaching: bool,
    eye_receding: bool
}

pub fn interpret_events (events: glium::backend::glutin_backend::PollEventsIter, input_state: &mut InputState) -> InputResult {
    let mut immediate_inputs = Vec::<InputCommand>::new();
    for event in events {
        match event {
            Event::KeyboardInput(_, _, Some(Key::Escape)) |
            Event::Closed => return InputResult::Exit,
            _ => match interpret_event(event, input_state) {
                Some(mut input) => immediate_inputs.append(&mut input),
                None => {}
            },
        }
    }

    let mut inputs = immediate_inputs;
    inputs.append(&mut recurring_inputs(input_state));

    InputResult::ContinueWithInputCommands(inputs)
}

fn interpret_event (event: Event, input_state: &mut InputState) -> Option<Vec<InputCommand>> {
    match event {
        Event::KeyboardInput(element_state, _, Some(key_code)) => {
            let pressed = element_state == ElementState::Pressed;
            match key_code {
                Key::Q => {input_state.rotating_eye_left = pressed; None},
                Key::E => {input_state.rotating_eye_right = pressed; None},
                Key::R => {input_state.tilting_eye_up = pressed; None},
                Key::F => {input_state.tilting_eye_down = pressed; None},
                Key::W => {input_state.moving_eye_forward = pressed; None},
                Key::S => {input_state.moving_eye_backward = pressed; None},
                Key::A => {input_state.moving_eye_left = pressed; None},
                Key::D => {input_state.moving_eye_right = pressed; None},
                Key::T => {input_state.eye_approaching = pressed; None},
                Key::G => {input_state.eye_receding = pressed; None}
                _ => None
        }},
        Event::MouseWheel(MouseScrollDelta::PixelDelta(x, y), _) => Some(vec![
            InputCommand::MoveEyeForwards(y * 0.005), InputCommand::MoveEyeSideways(x * -0.005)
        ]),
        _ => None
    }
}

fn recurring_inputs (input_state: &InputState) -> Vec<InputCommand> {
    let mut inputs = Vec::<InputCommand>::new();
    if input_state.rotating_eye_left {inputs.push(InputCommand::RotateEye(-0.02))};
    if input_state.rotating_eye_right {inputs.push(InputCommand::RotateEye(0.02))};
    if input_state.tilting_eye_up {inputs.push(InputCommand::TiltEye(0.02))};
    if input_state.tilting_eye_down {inputs.push(InputCommand::TiltEye(-0.02))};
    if input_state.moving_eye_forward {inputs.push(InputCommand::MoveEyeForwards(0.05))};
    if input_state.moving_eye_backward {inputs.push(InputCommand::MoveEyeForwards(-0.05))};
    if input_state.moving_eye_right {inputs.push(InputCommand::MoveEyeSideways(0.05))};
    if input_state.moving_eye_left {inputs.push(InputCommand::MoveEyeSideways(-0.05))};
    if input_state.eye_approaching {inputs.push(InputCommand::LetEyeApproach(0.05))};
    if input_state.eye_receding {inputs.push(InputCommand::LetEyeApproach(-0.05))};
    inputs
}

pub fn apply_input_command (command: InputCommand, past: &::models::State, future: &mut ::models::State) {
    let eye = past.ui_state.eye;
    let mut new_eye = future.ui_state.eye;
    match command {
        InputCommand::RotateEye(amount) => {
            new_eye.azimuth += amount;
        },
        InputCommand::TiltEye(amount) => {
            new_eye.inclination += amount;
            new_eye.inclination = new_eye.inclination.max(0.0).min(PI / 2.1);
        },
        InputCommand::MoveEyeForwards(amount) => {
            new_eye.target += eye.direction_2d() * amount;
        },
        InputCommand::MoveEyeSideways(amount) => {
            new_eye.target += eye.right_direction_2d() * amount;
        },
        InputCommand::LetEyeApproach(amount) => {
            new_eye.distance -= amount;
            new_eye.distance = new_eye.distance.max(0.3);
        }
    }
    future.ui_state.eye = new_eye;
}