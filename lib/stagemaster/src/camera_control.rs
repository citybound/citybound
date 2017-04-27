use kay::{Actor, Recipient, Fate};
use monet::{Renderer, MoveEye, Movement};
use descartes::{P2, P3};
use monet::glium::glutin::VirtualKeyCode;

#[derive(Serialize, Deserialize)]
pub struct CameraControlSettings {
    pub rotation_speed: f32,
    pub move_speed: f32,
    pub zoom_speed: f32,
    pub invert_y: bool,

    #[serde(with = "VirtualKeyCodeDef")]
    pub forward_key: VirtualKeyCode,
    #[serde(with = "VirtualKeyCodeDef")]
    pub backward_key: VirtualKeyCode,
    #[serde(with = "VirtualKeyCodeDef")]
    pub left_key: VirtualKeyCode,
    #[serde(with = "VirtualKeyCodeDef")]
    pub right_key: VirtualKeyCode,
    #[serde(with = "VirtualKeyCodeDef")]
    pub pan_modifier_key: VirtualKeyCode,
    #[serde(with = "VirtualKeyCodeDef")]
    pub yaw_modifier_key: VirtualKeyCode,
    #[serde(with = "VirtualKeyCodeDef")]
    pub pitch_modifier_key: VirtualKeyCode,
}

impl Default for CameraControlSettings {
    fn default() -> Self {
        CameraControlSettings {
            rotation_speed: 1.0f32,
            zoom_speed: 1.0f32,
            move_speed: 1.0f32,
            invert_y: false,

            forward_key: VirtualKeyCode::Up,
            backward_key: VirtualKeyCode::Down,
            left_key: VirtualKeyCode::Left,
            right_key: VirtualKeyCode::Right,
            pan_modifier_key: VirtualKeyCode::LShift,
            yaw_modifier_key: VirtualKeyCode::LAlt,
            pitch_modifier_key: VirtualKeyCode::LAlt,
        }
    }
}

pub struct CameraControl {
    settings: CameraControlSettings,
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    pan_modifier: bool,
    yaw_modifier: bool,
    pitch_modifier: bool,
    last_cursor_2d: P2,
    last_cursor_3d: P3,
}

impl CameraControl {
    fn new(settings: CameraControlSettings) -> Self {
        CameraControl {
            settings: settings,
            forward: false,
            backward: false,
            left: false,
            right: false,
            pan_modifier: false,
            yaw_modifier: false,
            pitch_modifier: false,
            last_cursor_2d: P2::new(0.0, 0.0),
            last_cursor_3d: P3::new(0.0, 0.0, 0.0),
        }
    }
}

impl Actor for CameraControl {}

use super::Event3d;

impl Recipient<Event3d> for CameraControl {
    fn receive(&mut self, msg: &Event3d) -> Fate {
        super::UserInterface::id() <<
        super::AddDebugText {
            key: vec!['c'].into(),
            text: vec!['i'].into(),
            color: [1.0, 0.0, 0.0, 1.0],
            persistent: false,
        };
        match *msg {
            Event3d::KeyDown(k) => {
                if k == self.settings.forward_key {
                    self.forward = true
                }
                if k == self.settings.backward_key {
                    self.backward = true
                }
                if k == self.settings.left_key {
                    self.left = true
                }
                if k == self.settings.right_key {
                    self.right = true
                }
                if k == self.settings.pan_modifier_key {
                    self.pan_modifier = true
                }
                if k == self.settings.yaw_modifier_key {
                    self.yaw_modifier = true
                }
                if k == self.settings.pitch_modifier_key {
                    self.pitch_modifier = true
                }
            }
            Event3d::KeyUp(k) => {
                if k == self.settings.forward_key {
                    self.forward = false
                }
                if k == self.settings.backward_key {
                    self.backward = false
                }
                if k == self.settings.left_key {
                    self.left = false
                }
                if k == self.settings.right_key {
                    self.right = false
                }
                if k == self.settings.pan_modifier_key {
                    self.pan_modifier = false
                }
                if k == self.settings.yaw_modifier_key {
                    self.yaw_modifier = false
                }
                if k == self.settings.pitch_modifier_key {
                    self.pitch_modifier = false
                }
            }
            Event3d::MouseMove(cursor_2d) => {
                let delta = cursor_2d - self.last_cursor_2d;
                self.last_cursor_2d = cursor_2d;

                if self.yaw_modifier {
                    Renderer::id() <<
                    MoveEye {
                        scene_id: 0,
                        movement: Movement::Yaw(-delta.x * self.settings.rotation_speed / 300.0),
                    };
                }

                if self.pitch_modifier {
                    Renderer::id() <<
                    MoveEye {
                        scene_id: 0,
                        movement: Movement::Pitch(-delta.y * self.settings.rotation_speed *
                                                  if self.settings.invert_y {
                            -1.0
                        } else {
                            1.0
                        } / 300.0),
                    };
                }
            }
            Event3d::MouseMove3d(cursor_3d) => {
                let delta = cursor_3d - self.last_cursor_3d;
                self.last_cursor_3d = cursor_3d;

                if self.pan_modifier {
                    Renderer::id() <<
                    MoveEye {
                        scene_id: 0,
                        movement: Movement::ShiftAbsolute(-delta),
                    };
                    // predict next movement to avoid jitter
                    self.last_cursor_3d -= delta;
                }
            }
            Event3d::Scroll(delta) => {
                Renderer::id() <<
                MoveEye {
                    scene_id: 0,
                    movement: ::monet::Movement::Zoom(delta.y * self.settings.zoom_speed,
                                                      self.last_cursor_3d),
                };
            }
            _ => {}
        }
        Fate::Live
    }
}

pub fn setup(env: &super::environment::Environment) {
    let settings = env.load_settings("Camera Control");
    let state = CameraControl::new(settings);
    CameraControl::register_with_state(state);
    CameraControl::handle::<Event3d>();
    super::UserInterface::id() <<
    super::AddInteractable(CameraControl::id(), super::AnyShape::Everywhere, 0);
    super::UserInterface::id() << super::Focus(CameraControl::id());
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(rustfmt, rustfmt_skip)]
#[serde(remote = "::monet::glium::glutin::VirtualKeyCode")]
#[allow(dead_code)]
enum VirtualKeyCodeDef {
    Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9, Key0, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, Escape, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, Snapshot, Scroll, Pause, Insert, Home, Delete, End, PageDown, PageUp, Left, Up, Right, Down, Back, Return, Space, Compose, Numlock, Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9, AbntC1, AbntC2, Add, Apostrophe, Apps, At, Ax, Backslash, Calculator, Capital, Colon, Comma, Convert, Decimal, Divide, Equals, Grave, Kana, Kanji, LAlt, LBracket, LControl, LMenu, LShift, LWin, Mail, MediaSelect, MediaStop, Minus, Multiply, Mute, MyComputer, NavigateForward, NavigateBackward, NextTrack, NoConvert, NumpadComma, NumpadEnter, NumpadEquals, OEM102, Period, PlayPause, Power, PrevTrack, RAlt, RBracket, RControl, RMenu, RShift, RWin, Semicolon, Slash, Sleep, Stop, Subtract, Sysrq, Tab, Underline, Unlabeled, VolumeDown, VolumeUp, Wake, WebBack, WebFavorites, WebForward, WebHome, WebRefresh, WebSearch, WebStop, Yen,
}