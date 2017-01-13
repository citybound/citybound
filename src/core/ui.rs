use ::monet::glium::{DisplayBuild, glutin};
use kay::{ActorSystem, ID, Individual, Recipient, Fate};
use compact::CVec;
use descartes::{N, P2, P3, V3, Into2d, Shape};
use ::monet::{Renderer, Scene, GlutinFacade, MoveEye};
use ::monet::glium::glutin::{Event, MouseScrollDelta, ElementState, MouseButton};
pub use ::monet::glium::glutin::VirtualKeyCode;
use core::geometry::AnyShape;
use ::std::collections::HashMap;
use ::core::settings::Settings;
use serde_json;
use serde;
use serde::{Serializer, Serialize, Deserialize, Deserializer};
use std::mem::transmute;

pub static mut USER_INTERFACE: Option<ID> = None;

#[derive(Clone, Debug, PartialEq)]
pub struct KeyCombination{
    keys: Vec<InterchangeableKeys>,
}

impl KeyCombination{
    fn triggered(&self, keys_held: &Vec<KeyOrButton>) -> bool{
        for i in self.keys{
            if !i.triggered(keys_held){
                return false
            }
        }
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InterchangeableKeys{
    keys: Vec<KeyOrButton>,
}

impl InterchangeableKeys{
    fn triggered(&self, keys_held: &Vec<KeyOrButton>) -> bool{
        for i in self.keys{
            for j in keys_held{
                if i == *j{
                    return true
                }
            }
        }
        false
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum KeyOrButton {
    Key(VirtualKeyCode),
    Button(MouseButton),
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
pub enum Mouse {
    Moved(P2),
    Scrolled(P2),
    Down(MouseButton),
    Up(MouseButton),
}

pub struct InputState {
    keys_down: Vec<KeyOrButton>,
    mouse: Vec<Mouse>,
}

impl InputState {
    fn new() -> InputState {
        InputState {
            keys_down: Vec::new(),
            mouse: Vec::new(),
        }
    }
}

pub struct UserInterface {
    interactables_2d: HashMap<ID, (AnyShape, usize)>,
    interactables_3d: HashMap<ID, (AnyShape, usize)>,
    /// Interactables which are always in focus e.g. Camera
    any_interactable: Vec<ID>,
    outbox: HashMap<ID, Box<UIInput>>,

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

#[derive(Compact, Clone)]
pub struct UIInput{
    /// Custom button events
    button_events: CVec<&'static str>,
    /// Drag and stuff
    mouse_actions: CVec<Event3d>,
    /// Mouse move and stuff, with possible key combination
    mouse_events: CVec<(Mouse, &'static str)>,
    /// Camera time tick
    dt: f32,
    //TypingEvent, //TODO: Actually implement this
}

impl UIInput{
    pub fn new() -> UIInput{
        UIInput {
            button_events: CVec::new(),
            mouse_actions: CVec::new(),
            mouse_events: CVec::new(),
            dt: 1.0f32/60.0f32,
        }
    }
}

impl Individual for UserInterface {}

impl UserInterface {
    fn new() -> UserInterface {
        UserInterface {
            any_interactable: Vec::new(),
            outbox: HashMap::new(),
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
pub enum UpdateCursor {
    Cursor2D(P2),
    Cursor3D(P3),
}


impl Recipient<UpdateCursor> for UserInterface {
    fn receive(&mut self, msg: &UpdateCursor) -> Fate {
        match *msg{
            UpdateCursor::Cursor2D(p) => {
                self.cursor_2d = p
            }
            UpdateCursor::Cursor3D(p) => {
                self.cursor_3d = p
            }
        }
        Fate::Live
    }
}

#[derive(Compact, Clone)]
pub enum AddInteractable {
    Interactable2d(ID, AnyShape, usize),
    Interactable3d(ID, AnyShape, usize),
}

impl Recipient<AddInteractable> for UserInterface {
    fn receive(&mut self, msg: &AddInteractable) -> Fate {
        match *msg {
            AddInteractable::Interactable2d(_id, ref _shape, _z_index) => unimplemented!(),
            AddInteractable::Interactable3d(id, ref shape, z_index) => {
                self.interactables_3d.insert(id, (shape.clone(), z_index));
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub enum Remove {
    Interactable2d(ID),
    Interactable3d(ID),
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

#[derive(Clone, Copy)]
pub enum Event3d {
    DragStarted { at: P3 },
    DragOngoing { from: P3, to: P3 },
    DragFinished { from: P3, to: P3 },
    DragAborted,
    HoverStarted { at: P3 },
    HoverOngoing { at: P3 },
    HoverStopped,
}

#[derive(Clone, Compact)]
pub struct KeysHeld{
    pub keys: CVec<KeyOrButton>,
}

impl Recipient<Mouse> for UserInterface {
    fn receive(&mut self, msg: &Mouse) -> Fate {
        self.input_state.mouse.push(*msg);
        match *msg {
            Mouse::Down(button)=> {
                self.input_state.keys_down.push(KeyOrButton::Button(button))
            }
            Mouse::Up(button) => {
                let index = self.input_state.keys_down.iter().position(|x| *x == KeyOrButton::Button(button)).unwrap();
                self.input_state.keys_down.remove(index);
            },
            _ => ()
        }
        Fate::Live
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum Key {
    Up(VirtualKeyCode),
    Down(VirtualKeyCode),
}

impl Recipient<Key> for UserInterface {
    fn receive(&mut self, msg: &Key) -> Fate {
        match *msg {
            Key::Down(key_code)=> {
                if self.input_state.keys_down.iter().position(|x| *x == KeyOrButton::Key(key_code)) == None{
                    self.input_state.keys_down.push(KeyOrButton::Key(key_code));
                }
            }
            Key::Up(key_code) => {
                let index = self.input_state.keys_down.iter().position(|x| *x == KeyOrButton::Key(key_code)).unwrap();
                self.input_state.keys_down.remove(index);
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
                    let message = self.outbox.entry(active_interactable).or_insert(box UIInput::new());
                    (*message).mouse_actions.push(Event3d::DragOngoing {
                        from: self.drag_start_3d.expect("active interactable but no drag start"),
                        to: position_3d
                    });
                } else {
                    let new_hovered_interactable = self.interactables_3d
                        .iter()
                        .filter(|&(_id, &(ref shape, _z_index))| {
                            shape.contains(position_3d.into_2d())
                        })
                        .max_by_key(|&(_id, &(ref _shape, z_index))| z_index)
                        .map(|(id, _shape)| *id);

                    if self.hovered_interactable != new_hovered_interactable {
                        if let Some(previous) = self.hovered_interactable {
                            let message = self.outbox.entry(previous).or_insert(box UIInput::new());
                            (*message).mouse_actions.push(Event3d::HoverStopped);
                        }
                        if let Some(next) = new_hovered_interactable {
                            let message = self.outbox.entry(next).or_insert(box UIInput::new());
                            (*message).mouse_actions.push(Event3d::HoverStarted { at: self.cursor_3d });
                        }
                    } else if let Some(hovered_interactable) = self.hovered_interactable {
                        let message = self.outbox.entry(hovered_interactable).or_insert(box UIInput::new());
                        (*message).mouse_actions.push(Event3d::HoverOngoing { at: self.cursor_3d });
                    }
                    self.hovered_interactable = new_hovered_interactable;
                }
                Fate::Live
            }
        }
    }
}

/// Return true if there is at least one common element between a and b
pub fn intersection(a: &[KeyOrButton], b: &[KeyOrButton]) -> bool{
    for i in a{
        for j in b{
            if i == j {return true}
        }
    }
    false
}

#[derive(Copy, Clone)]
pub struct UITick{
    dt: f32
}

impl Recipient<UITick> for UserInterface {
    fn receive(&mut self, msg: &UITick) -> Fate {
        let button_events: CVec<&String> = CVec::new();
        for (keys, name) in self.settings.key_mappings{
            if keys.triggered(&self.input_state.keys_down){
                button_events.push(&name.clone());
            }
        }

        let mouse_events: CVec<(Mouse, &String)> = CVec::new();

        for mouse_action in &self.input_state.mouse.clone() {
            match *mouse_action {
                Mouse::Moved(position) => {
                    Renderer::id() << Project2dTo3d {
                        scene_id: 0,
                        position_2d: position,
                        requester: Self::id()
                    };
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
            for (keys, names) in self.settings.mouse_modifier_mappings{
                if keys.triggered(&self.input_state.keys_down){
                    mouse_events.push((*mouse_action, names))
                }
            }
        }
        if let Some(active_id) = self.active_interactable {
            let active_message = self.outbox.entry(active_id).or_insert(box UIInput::new());
            (*active_message).button_events = button_events.clone();
            (*active_message).mouse_events = mouse_events.clone();

            active_id << active_message;
            self.outbox.remove(&active_id);
        }


        for i in self.active_interactable {

            let any_message = self.outbox.entry(i).or_insert(box UIInput::new());
            (*any_message).button_events = button_events.clone();

            i << any_message;
            self.outbox.remove(i);
        }

        for (k, v) in self.outbox{
            k << v;
        }

        self.outbox.clear();
        self.input_state.mouse.clear();

        Fate::Live
    }
}

pub fn setup_window_and_renderer(system: &mut ActorSystem, renderables: Vec<ID>) -> GlutinFacade {
    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(1024, 512)
        .with_multitouch()
        .with_vsync()
        .build_glium()
        .unwrap();

    let ui = UserInterface::new();

    USER_INTERFACE = Some(ID{type_id: UserInterface::id(), version: 0, instance_id: 0});

    system.add_inbox::<AddInteractable, UserInterface>();
    system.add_inbox::<Remove, UserInterface>();
    system.add_inbox::<Focus, UserInterface>();
    system.add_unclearable_inbox::<Mouse, UserInterface>();
    system.add_unclearable_inbox::<Key, UserInterface>();
    system.add_unclearable_inbox::<UITick, UserInterface>();
    system.add_unclearable_inbox::<Projected3d, UserInterface>();
    system.add_unclearable_inbox::<UpdateCursor, UserInterface>();

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
    UserInterface::id() << UITick {};
    true
}
