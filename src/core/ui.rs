use ::monet::glium::{DisplayBuild, glutin};
use kay::{ActorSystem, ID, Individual, Recipient, Fate};
use descartes::{N, P2, P3, V3, Into2d, Shape};
use ::monet::{Renderer, Scene, GlutinFacade, MoveEye};
use ::monet::glium::glutin::{Event, MouseScrollDelta, ElementState, MouseButton};
pub use ::monet::glium::glutin::{VirtualKeyCode};
use core::geometry::AnyShape;
use ::std::collections::HashMap;
use ::core::settings::Settings;
use serde_json;
use serde;
use serde::{Serializer, Serialize, Deserialize, Deserializer};
use std::mem::transmute;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum KeyOrButton{
    Key(VirtualKeyCode),
    Button(MouseButton),
}

impl Serialize for KeyOrButton{
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer
    {
        match *self{
            KeyOrButton::Key(code) => serializer.serialize_u64(code as u64),
            KeyOrButton::Button(code) => match code {
                MouseButton::Other(code) => serializer.serialize_u64(code as u64 + 2000),
                MouseButton::Left => serializer.serialize_u64(1001),
                MouseButton::Middle => serializer.serialize_u64(1002),
                MouseButton::Right => serializer.serialize_u64(1003),
            }
        }
    }
}

impl Deserialize for KeyOrButton{
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer,
    {
        let deser_result: serde_json::Value = serde::Deserialize::deserialize(deserializer)?;
        match deser_result {
            serde_json::Value::U64(b) if b > 1000 => match b {
                1001 => Ok(KeyOrButton::Button(MouseButton::Left)),
                1002 => Ok(KeyOrButton::Button(MouseButton::Middle)),
                1003 => Ok(KeyOrButton::Button(MouseButton::Right)),
                _ => Ok(KeyOrButton::Button(MouseButton::Other((b - 2000) as u8))),
            },
            serde_json::Value::U64(b) => Ok(KeyOrButton::Key(unsafe{transmute(b as u8)})),
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

    rotate_mod: bool,
    pan_mod: bool,

    mouse: Vec<Mouse>,
}

impl InputState {
    fn new() -> InputState {
        InputState {
            forward: false,
            backward: false,
            left: false,
            right: false,

            rotate_mod: false,
            pan_mod: false,

            mouse: vec![],
        }
    }
}

pub struct UserInterface {
    interactables_2d: HashMap<ID, (AnyShape, usize)>,
    interactables_3d: HashMap<ID, (AnyShape, usize)>,
    cursor_2d: P2,
    cursor_3d: P3,
    drag_start_2d: Option<P2>,
    drag_start_3d: Option<P3>,
    hovered_interactable: Option<ID>,
    active_interactable: Option<ID>,
    focused_interactable: Option<ID>,
    input_state: InputState,
    settings: Settings,
}

impl Individual for UserInterface {}

impl UserInterface {
    fn new() -> UserInterface {
        UserInterface {
            interactables_2d: HashMap::new(),
            interactables_3d: HashMap::new(),
            cursor_2d: P2::new(0.0, 0.0),
            cursor_3d: P3::new(0.0, 0.0, 0.0),
            drag_start_2d: None,
            drag_start_3d: None,
            hovered_interactable: None,
            active_interactable: None,
            focused_interactable: None,
            input_state: InputState::new(),
            settings: Settings::load(),
        }
    }
}

#[derive(Compact, Clone)]
pub enum Add {
    Interactable2d(ID, AnyShape, usize),
    Interactable3d(ID, AnyShape, usize)
}

impl Recipient<Add> for UserInterface {
    fn receive(&mut self, msg: &Add) -> Fate {
        match *msg {
            Add::Interactable2d(_id, ref _shape, _z_index) => unimplemented!(),
            Add::Interactable3d(id, ref shape, z_index) => {
                self.interactables_3d.insert(id, (shape.clone(), z_index));
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub enum Remove {
    Interactable2d(ID),
    Interactable3d(ID)
}

impl Recipient<Remove> for UserInterface {
    fn receive(&mut self, msg: &Remove) -> Fate {
        match *msg {
            Remove::Interactable2d(_id) => unimplemented!(),
            Remove::Interactable3d(id) => {
                self.interactables_3d.remove(&id);
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct Focus(pub ID);

impl Recipient<Focus> for UserInterface {
    fn receive(&mut self, msg: &Focus) -> Fate {
        match *msg {
            Focus(id) => {
                self.focused_interactable = Some(id);
                Fate::Live
            }
        }
    }
}

use ::monet::Project2dTo3d;

#[derive(Copy, Clone)]
pub enum Event3d {
    DragStarted {
        at: P3
    },
    DragOngoing {
        from: P3,
        to: P3
    },
    DragFinished {
        from: P3,
        to: P3
    },
    DragAborted,
    HoverStarted {
        at: P3
    },
    HoverOngoing {
        at: P3
    },
    HoverStopped,
    KeyDown(VirtualKeyCode),
    KeyUp(VirtualKeyCode)
}

impl Recipient<Mouse> for UserInterface {
    fn receive(&mut self, msg: &Mouse) -> Fate {
        self.input_state.mouse.push(*msg);
        match *msg {
            Mouse::Down(button) | Mouse::Up(button) => {
                let down = *msg == Mouse::Down(button);
                // If you want to bind movement to mouse buttons, who am I to judge
                if self.settings.forward_key.iter().any(|x| *x == KeyOrButton::Button(button)){
                    self.input_state.forward = down;
                }
                if self.settings.backward_key.iter().any(|x| *x == KeyOrButton::Button(button)){
                    self.input_state.backward = down;
                }
                if self.settings.left_key.iter().any(|x| *x == KeyOrButton::Button(button)){
                    self.input_state.left = down;
                }
                if self.settings.right_key.iter().any(|x| *x == KeyOrButton::Button(button)){
                    self.input_state.right = down;
                }

                if self.settings.rotate_modifier_key.iter().any(|x| *x == KeyOrButton::Button(button)){
                    self.input_state.rotate_mod = down;
                }
                if self.settings.pan_modifier_key.iter().any(|x| *x == KeyOrButton::Button(button)){
                    self.input_state.pan_mod = down;
                }
            },
            _ => ()
        }
        Fate::Live
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum Key {
    Up(VirtualKeyCode),
    Down(VirtualKeyCode)
}

impl Recipient<Key> for UserInterface {
    fn receive(&mut self, msg: &Key) -> Fate {
        match *msg {
            Key::Down(key_code) | Key::Up(key_code) => {
                let down = *msg == Key::Down(key_code);
                if self.settings.forward_key.iter().any(|x| *x == KeyOrButton::Key(key_code)){
                    self.input_state.forward = down;
                };
                if self.settings.backward_key.iter().any(|x| *x == KeyOrButton::Key(key_code)){
                    self.input_state.backward = down;
                };
                if self.settings.left_key.iter().any(|x| *x == KeyOrButton::Key(key_code)){
                    self.input_state.left = down;
                };
                if self.settings.right_key.iter().any(|x| *x == KeyOrButton::Key(key_code)){
                    self.input_state.right = down;
                };
                if self.settings.rotate_modifier_key.iter().any(|x| *x == KeyOrButton::Key(key_code)){
                    self.input_state.rotate_mod = down;
                };
                if self.settings.pan_modifier_key.iter().any(|x| *x == KeyOrButton::Key(key_code)){
                    self.input_state.pan_mod = down;
                };

                self.focused_interactable.map(|interactable|
                    interactable << if down
                        {Event3d::KeyDown(key_code)}else {Event3d::KeyUp(key_code)}
                );
                ()
            },
        }
        Fate::Live
    }
}

use ::monet::Projected3d;

impl Recipient<Projected3d> for UserInterface {
    fn receive(&mut self, msg: &Projected3d) -> Fate {
        match *msg {
            Projected3d { position_3d } => {
                self.cursor_3d = position_3d;
                if let Some(active_interactable) = self.active_interactable {
                    active_interactable << Event3d::DragOngoing {
                        from: self.drag_start_3d.expect("active interactable but no drag start"),
                        to: position_3d
                    };
                } else {
                    let new_hovered_interactable = self.interactables_3d.iter().filter(|&(_id, &(ref shape, _z_index))|
                        shape.contains(position_3d.into_2d())
                    ).max_by_key(|&(_id, &(ref _shape, z_index))|
                        z_index
                    ).map(|(id, _shape)| *id);

                    if self.hovered_interactable != new_hovered_interactable {
                        if let Some(previous) = self.hovered_interactable {
                            previous << Event3d::HoverStopped;
                        }
                        if let Some(next) = new_hovered_interactable {
                            next << Event3d::HoverStarted { at: self.cursor_3d };
                        }
                    } else if let Some(hovered_interactable) = self.hovered_interactable {
                        hovered_interactable << Event3d::HoverOngoing { at: self.cursor_3d };
                    }
                    self.hovered_interactable = new_hovered_interactable;
                }
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
struct UIUpdate;

impl Recipient<UIUpdate> for UserInterface {
    fn receive(&mut self, msg: &UIUpdate) -> Fate {
        for mouse_action in &self.input_state.mouse.clone() {
            match *mouse_action {
                Mouse::Moved(position) => {
                    let delta = self.cursor_2d - position;
                    if self.input_state.rotate_mod {
                        Renderer::id() << MoveEye { scene_id: 0, movement: ::monet::Movement::Rotate(-delta.x * self.settings.rotation_speed / 300.0) };
                        self.cursor_2d = position;
                    } else if self.input_state.pan_mod {
                        Renderer::id() << MoveEye { scene_id: 0, movement: ::monet::Movement::Shift(V3::new(-delta.y * self.settings.move_speed / 3.0, delta.x * self.settings.move_speed / 3.0, 0.0)) };
                        self.cursor_2d = position;
                    } else {
                        self.cursor_2d = position;
                        Renderer::id() << Project2dTo3d {
                            scene_id: 0,
                            position_2d: position,
                            requester: Self::id()
                        };
                    }
                },
                Mouse::Scrolled(delta) => {
                    Renderer::id() << MoveEye{scene_id: 0, movement: ::monet::Movement::Zoom(delta.y * self.settings.zoom_speed)};
                },
                Mouse::Down(MouseButton::Left) => {
                    self.drag_start_2d = Some(self.cursor_2d);
                    self.drag_start_3d = Some(self.cursor_3d);
                    let cursor_3d = self.cursor_3d;
                    self.receive(&Projected3d { position_3d: cursor_3d });
                    self.active_interactable = self.hovered_interactable;
                    if let Some(active_interactable) = self.active_interactable {
                        active_interactable << Event3d::DragStarted { at: self.cursor_3d };
                    }
                },
                Mouse::Up(MouseButton::Left) => {
                    if let Some(active_interactable) = self.active_interactable {
                        active_interactable << Event3d::DragFinished {
                            from: self.drag_start_3d.expect("active interactable but no drag start"),
                            to: self.cursor_3d
                        };
                    }
                    self.drag_start_2d = None;
                    self.drag_start_3d = None;
                    self.active_interactable = None;
                },
                _ => ()
            }
        }
        if self.input_state.forward {
            Renderer::id() << MoveEye { scene_id: 0, movement: ::monet::Movement::Shift(V3::new(5.0 * self.settings.move_speed, 0.0, 0.0))};
        }
        if self.input_state.backward {
            Renderer::id() << MoveEye { scene_id: 0, movement: ::monet::Movement::Shift(V3::new(-5.0 * self.settings.move_speed, 0.0, 0.0))};
        }
        if self.input_state.left {
            Renderer::id() << MoveEye { scene_id: 0, movement: ::monet::Movement::Shift(V3::new(0.0, -5.0 * self.settings.move_speed, 0.0))};
        }
        if self.input_state.right {
            Renderer::id() << MoveEye { scene_id: 0, movement: ::monet::Movement::Shift(V3::new(0.0, 5.0 * self.settings.move_speed, 0.0))};
        }

        self.input_state.mouse.clear();
        Fate::Live
    }
}

pub fn setup_window_and_renderer(system: &mut ActorSystem, renderables: Vec<ID>) -> GlutinFacade {
    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(1024, 512)
        .with_multitouch()
        .with_vsync().build_glium().unwrap();

    let ui = UserInterface::new();

    system.add_individual(ui);
    system.add_inbox::<Add, UserInterface>();
    system.add_inbox::<Remove, UserInterface>();
    system.add_inbox::<Focus, UserInterface>();
    system.add_unclearable_inbox::<Mouse, UserInterface>();
    system.add_unclearable_inbox::<Key, UserInterface>();
    system.add_unclearable_inbox::<UIUpdate, UserInterface>();
    system.add_unclearable_inbox::<Projected3d, UserInterface>();

    let mut renderer = Renderer::new(window.clone());
    let mut scene = Scene::new();
    scene.eye.position *= 30.0;
    scene.renderables = renderables;
    renderer.scenes.insert(0, scene);

    ::monet::setup(system, renderer);

    window
}

pub fn process_events(window: &GlutinFacade) -> bool {
    for event in window.poll_events().collect::<Vec<_>>() {
        match event {
            Event::Closed => return false,
            Event::MouseWheel(delta, _) => {
                UserInterface::id() << Mouse::Scrolled(match delta{
                    MouseScrollDelta::LineDelta(x, y) => P2::new(x * 50 as N, y * 50 as N),
                    MouseScrollDelta::PixelDelta(x, y) => P2::new(x as N, y as N)
                })
            },
            Event::MouseMoved(x, y) =>
                UserInterface::id() << Mouse::Moved(P2::new(x as N, y as N)),
            Event::MouseInput(ElementState::Pressed, button) =>
                UserInterface::id() << Mouse::Down(button),
            Event::MouseInput(ElementState::Released, button) =>
                UserInterface::id() << Mouse::Up(button),
            Event::KeyboardInput(ElementState::Pressed, _, Some(key_code)) =>
                UserInterface::id() << Key::Down(key_code),
            Event::KeyboardInput(ElementState::Released, _, Some(key_code)) =>
                UserInterface::id() << Key::Up(key_code),
            _ => {}
        }
    }
    UserInterface::id() << UIUpdate {};
    true
}