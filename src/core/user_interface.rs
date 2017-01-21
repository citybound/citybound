use core::ui::{KeyCombination, KeyOrButton};
use monet::Movement::{Shift, Yaw, Pitch};
use monet::{Renderer, Scene, GlutinFacade, MoveEye};
use kay::{ID, Actor, Recipient, Fate};
use ::core::ui::{Mouse};
use descartes::{N, P2, P3, V3, Into2d, Shape};
use monet::glium::glutin::{MouseButton};
pub use monet::glium::glutin::VirtualKeyCode;
use core::geometry::AnyShape;
use std::collections::HashMap;
use compact::CVec;
use core::settings::{Settings, Action, MouseAction, KeyAction};

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

    forward_action_id: Option<usize>,
    backward_action_id: Option<usize>,
    left_action_id: Option<usize>,
    right_action_id: Option<usize>,

    zoom_modifier: Option<usize>,
    mouse_modifier: Option<usize>,
    yaw_modifier: Option<usize>,
    pan_modifier: Option<usize>,
    pitch_modifier: Option<usize>,

    mouse_drag: Option<usize>,
}

impl Actor for UserInterface {}

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

            backward_action_id: None,
            forward_action_id: None,
            left_action_id: None,
            right_action_id: None,

            zoom_modifier: None,
            mouse_modifier: None,
            yaw_modifier: None,
            pan_modifier: None,
            pitch_modifier: None,

            mouse_drag: None,
        }
    }

    fn setup(&mut self) {
        self.forward_action_id = Some(Settings::register_key(KeyCombination{
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::W),
                            KeyOrButton::Key(VirtualKeyCode::Up)]],
        }));
        self.backward_action_id = Some(Settings::register_key(KeyCombination{
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::W),
                            KeyOrButton::Key(VirtualKeyCode::Up)]],
        }));
        self.forward_action_id = Some(Settings::register_key(KeyCombination{
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::W),
                            KeyOrButton::Key(VirtualKeyCode::Up)]],
        }));
        self.forward_action_id = Some(Settings::register_key(KeyCombination{
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::W),
                            KeyOrButton::Key(VirtualKeyCode::Up)]],
        }));


        self.zoom_modifier = Some(Settings::register_key(KeyCombination{
            keys: vec![],
        }));
        self.mouse_modifier = Some(Settings::register_key(KeyCombination{
            keys: vec![],
        }));
        self.yaw_modifier = Some(Settings::register_key(KeyCombination{
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::LAlt),
                            KeyOrButton::Key(VirtualKeyCode::RAlt),
                            KeyOrButton::Button(MouseButton::Middle),
            ]],
        }));
        self.yaw_modifier = Some(Settings::register_key(KeyCombination{
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::LAlt),
                            KeyOrButton::Key(VirtualKeyCode::RAlt),
                            KeyOrButton::Button(MouseButton::Middle),
            ]],
        }));

        self.mouse_drag = Some(Settings::register_key(KeyCombination{
            keys: vec![vec![KeyOrButton::Button(MouseButton::Left)]],
        }));
    }
}

impl Default for UserInterface {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Compact, Clone)]
pub enum Add {
    Interactable2d(ID, AnyShape, usize),
    Interactable3d(ID, AnyShape, usize),
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

use monet::Project2dTo3d;

#[derive(Copy, Clone)]
pub enum Event3d {
    DragStarted { at: P3 },
    DragOngoing { from: P3, to: P3 },
    DragFinished { from: P3, to: P3 },
    DragAborted,
    HoverStarted { at: P3 },
    HoverOngoing { at: P3 },
    HoverStopped,
    KeyDown(VirtualKeyCode),
    KeyUp(VirtualKeyCode),
}

use monet::Projected3d;

impl Recipient<Projected3d> for UserInterface {
    fn receive(&mut self, msg: &Projected3d) -> Fate {
        match *msg {
            Projected3d { position_3d } => {
                self.cursor_3d = position_3d;
                if let Some(active_interactable) = self.active_interactable {
                    active_interactable <<
                        Event3d::DragOngoing {
                            from: self.drag_start_3d.expect("active interactable but no drag start"),
                            to: position_3d,
                        };
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

impl Recipient<Action> for UserInterface {
    fn receive(&mut self, msg: &Action) -> Fate {
        match *msg {
            Action::Mouse(MouseAction{action_id: id, mouse: Mouse::Moved(position)}) => {
                let delta = self.cursor_2d - position;
                if Some(id) == self.yaw_modifier {
                    Renderer::id() << MoveEye {
                        scene_id: 0,
                        movement: Yaw(-delta.x * Settings::get_rotation_speed() * Settings::get_invert_y() / 300.0),
                    };
                }
                if Some(id) == self.pitch_modifier {
                    Renderer::id() << MoveEye {
                        scene_id: 0,
                        movement: Pitch(-delta.y * Settings::get_rotation_speed() * Settings::get_invert_y() / 300.0),
                    };
                }
                if Some(id) == self.pan_modifier {
                    Renderer::id() <<
                        MoveEye {
                            scene_id: 0,
                            movement: Shift(V3::new(-delta.y * Settings::get_move_speed() * Settings::get_invert_y() / 3.0,
                                                    delta.x * Settings::get_move_speed() * Settings::get_invert_y() / 3.0,
                                                    0.0)),
                        };
                }
                if Some(id) == self.mouse_modifier {
                    self.cursor_2d = position;
                    Renderer::id() <<
                        Project2dTo3d {
                            scene_id: 0,
                            position_2d: position,
                            requester: Self::id(),
                        };
                }
            }
            Action::Mouse(MouseAction{action_id: id, mouse: Mouse::Scrolled(delta)}) => {
                if Some(id) == self.zoom_modifier {
                    Renderer::id() <<
                        MoveEye {
                            scene_id: 0,
                            movement: ::monet::Movement::Zoom(delta.y * Settings::get_zoom_speed()),
                        };
                }
            }
            Action::KeyHeld(KeyAction{action_id: id}) => {
                if Some(id) == self.forward_action_id {
                    Renderer::id() <<
                        MoveEye {
                            scene_id: 0,
                            movement: ::monet::Movement::Shift(V3::new(5.0 * Settings::get_move_speed(),
                                                                       0.0,
                                                                       0.0)),
                        };
                }
                if Some(id) == self.backward_action_id {
                    Renderer::id() <<
                        MoveEye {
                            scene_id: 0,
                            movement: ::monet::Movement::Shift(V3::new(-5.0 * Settings::get_move_speed(),
                                                                       0.0,
                                                                       0.0)),
                        };
                }
                if Some(id) == self.left_action_id {
                    Renderer::id() <<
                        MoveEye {
                            scene_id: 0,
                            movement: ::monet::Movement::Shift(V3::new(0.0,
                                                                       -5.0 * Settings::get_move_speed(),
                                                                       0.0)),
                        };
                }
                if Some(id) == self.right_action_id {
                    Renderer::id() <<
                        MoveEye {
                            scene_id: 0,
                            movement: ::monet::Movement::Shift(V3::new(0.0,
                                                                       5.0 * Settings::get_move_speed(),
                                                                       0.0)),
                        };
                }
            }
            Action::KeyDown(KeyAction{action_id: id}) => {
                if Some(id) == self.mouse_drag {
                    self.drag_start_2d = Some(self.cursor_2d);
                    self.drag_start_3d = Some(self.cursor_3d);
                    let cursor_3d = self.cursor_3d;
                    self.receive(&Projected3d { position_3d: cursor_3d });
                    self.active_interactable = self.hovered_interactable;
                    if let Some(active_interactable) = self.active_interactable {
                        active_interactable << Event3d::DragStarted { at: self.cursor_3d };
                    }
                }
            }
            Action::KeyUp(KeyAction{action_id: id}) => {
                if let Some(active_interactable) = self.active_interactable {
                    active_interactable <<
                        Event3d::DragFinished {
                            from: self.drag_start_3d
                                .expect("active interactable but no drag start"),
                            to: self.cursor_3d,
                        };
                }
                self.drag_start_2d = None;
                self.drag_start_3d = None;
                self.active_interactable = None;
            }
            _ => ()
        }
        Fate::Live
    }
}

#[derive(Copy, Clone)]
pub struct UIUpdate;

impl Recipient<UIUpdate> for UserInterface {
    fn receive(&mut self, _msg: &UIUpdate) -> Fate {
        Fate::Live
    }
}