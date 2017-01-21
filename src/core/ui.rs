use monet::glium::{DisplayBuild, glutin};
use kay::{ID, Actor, Recipient, Fate};
use descartes::{N, P2, P3, V3, Into2d, Shape};
use monet::{Renderer, Scene, GlutinFacade, MoveEye};
use monet::Movement::{Shift, Yaw, Pitch};
use monet::glium::glutin::{Event, MouseScrollDelta, ElementState, MouseButton};
pub use monet::glium::glutin::VirtualKeyCode;
use core::geometry::AnyShape;
use core::user_interface::UserInterface;
use std::collections::HashMap;
use compact::CVec;
use core::settings::Settings;
use serde_json;
use serde;
use serde::{Serializer, Serialize, Deserialize, Deserializer};
use std::mem::transmute;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum KeyOrButton {
    Key(VirtualKeyCode),
    Button(MouseButton),
}

#[derive(Clone, Debug, Compact)]
pub struct KeyCombination {
    keys: Vec<Vec<KeyOrButton>>,
}

impl Serialize for KeyOrButton {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer
    {
        match *self {
            KeyOrButton::Key(code) => serializer.serialize_u64(code as u64),
            KeyOrButton::Button(code) => {
                match code {
                    MouseButton::Other(code) => serializer.serialize_u64(code as u64 + 2000),
                    MouseButton::Left => serializer.serialize_u64(1001),
                    MouseButton::Middle => serializer.serialize_u64(1002),
                    MouseButton::Right => serializer.serialize_u64(1003),
                }
            }
        }
    }
}

impl Deserialize for KeyOrButton {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer
    {
        let deser_result: serde_json::Value = serde::Deserialize::deserialize(deserializer)?;
        match deser_result {
            serde_json::Value::U64(b) if b > 1000 => {
                match b {
                    1001 => Ok(KeyOrButton::Button(MouseButton::Left)),
                    1002 => Ok(KeyOrButton::Button(MouseButton::Middle)),
                    1003 => Ok(KeyOrButton::Button(MouseButton::Right)),
                    _ => Ok(KeyOrButton::Button(MouseButton::Other((b - 2000) as u8))),
                }
            }
            serde_json::Value::U64(b) => Ok(KeyOrButton::Key(unsafe { transmute(b as u8) })),
            _ => Err(serde::de::Error::custom("Unexpected value")),
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Mouse {
    Moved(P2),
    Scrolled(P2),
    Down(MouseButton),
    Up(MouseButton),
}

pub struct InputState {
    forward: bool,
    backward: bool,

    left: bool,
    right: bool,

    yaw_mod: bool,
    pan_mod: bool,
    pitch_mod: bool,

    mouse: Vec<Mouse>,
}

impl InputState {
    fn new() -> InputState {
        InputState {
            forward: false,
            backward: false,
            left: false,
            right: false,

            yaw_mod: false,
            pan_mod: false,
            pitch_mod: false,

            mouse: vec![],
        }
    }
}

pub fn setup_window_and_renderer(renderables: Vec<ID>) -> GlutinFacade {
    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(1024, 512)
        .with_multitouch()
        .with_vsync()
        .build_glium()
        .unwrap();

    UserInterface::register_default();
    UserInterface::handle::<Add>();
    UserInterface::handle::<Remove>();
    UserInterface::handle::<Focus>();
    UserInterface::handle_critically::<Mouse>();
    UserInterface::handle_critically::<Key>();
    UserInterface::handle_critically::<UIUpdate>();
    UserInterface::handle_critically::<Projected3d>();

    let mut renderer = Renderer::new(window.clone());
    let mut scene = Scene::new();
    scene.eye.position *= 30.0;
    scene.renderables = renderables;
    renderer.scenes.insert(0, scene);

    ::monet::setup(renderer);

    window
}

pub fn process_events(window: &GlutinFacade) -> bool {
    for event in window.poll_events().collect::<Vec<_>>() {
        match event {
            Event::Closed => return false,
            Event::MouseWheel(delta, _) => {
                UserInterface::id() <<
                Mouse::Scrolled(match delta {
                    MouseScrollDelta::LineDelta(x, y) => P2::new(x * 50 as N, y * 50 as N),
                    MouseScrollDelta::PixelDelta(x, y) => P2::new(x as N, y as N),
                })
            }
            Event::MouseMoved(x, y) => UserInterface::id() << Mouse::Moved(P2::new(x as N, y as N)),
            Event::MouseInput(ElementState::Pressed, button) => {
                UserInterface::id() << Mouse::Down(button)
            }
            Event::MouseInput(ElementState::Released, button) => {
                UserInterface::id() << Mouse::Up(button)
            }
            Event::KeyboardInput(ElementState::Pressed, _, Some(key_code)) => {
                UserInterface::id() << Key::Down(key_code)
            }
            Event::KeyboardInput(ElementState::Released, _, Some(key_code)) => {
                UserInterface::id() << Key::Up(key_code)
            }
            _ => {}
        }
    }
    UserInterface::id() << UIUpdate {};
    true
}
