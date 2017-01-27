use monet::glium::{DisplayBuild, glutin};
use kay::{ID, Actor};
use descartes::{N, P2};
use monet::{Renderer, Scene, GlutinFacade, Projected3d};
use monet::Movement::{Shift, Yaw, Pitch};
use monet::glium::glutin::{Event, MouseScrollDelta, ElementState, MouseButton};
pub use monet::glium::glutin::VirtualKeyCode;
use core::geometry::AnyShape;
use core::user_interface::{UserInterface, Add, Remove, Focus, UIUpdate};
use std::collections::HashMap;
use compact::CVec;
use core::settings::{Settings, Action};
use serde_json;
use serde;
use serde::{Serializer, Serialize, Deserialize, Deserializer};
use std::mem::{transmute, forget};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeyOrButton {
    Key(VirtualKeyCode),
    Button(MouseButton),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct KeyCombination {
    pub keys: Vec<Vec<KeyOrButton>>,
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

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Mouse {
    Moved(P2),
    Scrolled(P2),
    Down(MouseButton),
    Up(MouseButton),
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
    UserInterface::handle_critically::<Action>();
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

pub fn process_events(window: &GlutinFacade, keys_held: &mut Vec<KeyOrButton>) -> bool {
    let mut mouse = Vec::<Mouse>::new();
    let mut new_keys = Vec::<KeyOrButton>::new();
    println!("Frame start:");
    println!("Current keys held: {:?}", keys_held);
    for event in window.poll_events().collect::<Vec<_>>() {
        match event {
            Event::Closed => return false,
            Event::MouseWheel(delta, _) => {
                mouse.push(Mouse::Scrolled(match delta {
                    MouseScrollDelta::LineDelta(x, y) => P2::new(x * 50 as N, y * 50 as N),
                    MouseScrollDelta::PixelDelta(x, y) => P2::new(x as N, y as N),
                }))
            }
            Event::MouseMoved(x, y) => {
                mouse.push(Mouse::Moved(P2::new(x as N, y as N)));
            }
            Event::MouseInput(ElementState::Pressed, button) => {
                mouse.push(Mouse::Down(button));
                new_keys.push(KeyOrButton::Button(button));
            }
            Event::MouseInput(ElementState::Released, button) => {
                mouse.push(Mouse::Up(button));
                if let Some(index) = keys_held.iter()
                    .position(|x| *x == KeyOrButton::Button(button)) {
                    keys_held.remove(index);
                }
                if let Some(index) = new_keys.iter()
                    .position(|x| *x == KeyOrButton::Button(button)) {
                    new_keys.remove(index);
                }
            }
            Event::KeyboardInput(ElementState::Pressed, _, Some(key_code)) => {
                // to deal with key repeat
                if !keys_held.contains(&KeyOrButton::Key(key_code)) &&
                   !new_keys.contains(&KeyOrButton::Key(key_code)) {
                    new_keys.push(KeyOrButton::Key(key_code))
                }
            }
            Event::KeyboardInput(ElementState::Released, _, Some(key_code)) => {
                keys_held.retain(|x| *x != KeyOrButton::Key(key_code));
                new_keys.retain(|x| *x != KeyOrButton::Key(key_code));
            }
            _ => {}
        }
    }
    println!("New keys: {:?}", new_keys);
    Settings::send(UserInterface::id(), &keys_held, &new_keys, &mouse);
    keys_held.extend(new_keys);
    UserInterface::id() << UIUpdate {};
    true
}
