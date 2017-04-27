use kay::{Actor, Recipient, Fate};
use monet::{Renderer, MoveEye, Movement};
use descartes::{P2, P3, V3};
use combo::Button::*;
use super::combo::Combo2;

#[derive(Serialize, Deserialize)]
pub struct CameraControlSettings {
    pub rotation_speed: f32,
    pub move_speed: f32,
    pub zoom_speed: f32,
    pub invert_y: bool,

    pub forward_combo: Combo2,
    pub backward_combo: Combo2,
    pub left_combo: Combo2,
    pub right_combo: Combo2,
    pub pan_modifier_combo: Combo2,
    pub yaw_modifier_combo: Combo2,
    pub pitch_modifier_combo: Combo2,
}

impl Default for CameraControlSettings {
    fn default() -> Self {
        CameraControlSettings {
            rotation_speed: 1.0f32,
            zoom_speed: 1.0f32,
            move_speed: 1.0f32,
            invert_y: false,

            forward_combo: Combo2::new(&[Up], &[W]),
            backward_combo: Combo2::new(&[Down], &[S]),
            left_combo: Combo2::new(&[Left], &[A]),
            right_combo: Combo2::new(&[Right], &[D]),
            pan_modifier_combo: Combo2::new(&[LShift], &[RShift]),
            yaw_modifier_combo: Combo2::new(&[LAlt], &[RightMouseButton]),
            pitch_modifier_combo: Combo2::new(&[LAlt], &[RightMouseButton]),
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
        match *msg {
            Event3d::Combos(combos) => {
                self.forward = self.settings.forward_combo.is_in(&combos);
                self.backward = self.settings.backward_combo.is_in(&combos);
                self.left = self.settings.left_combo.is_in(&combos);
                self.right = self.settings.right_combo.is_in(&combos);
                self.pan_modifier = self.settings.pan_modifier_combo.is_in(&combos);
                self.yaw_modifier = self.settings.yaw_modifier_combo.is_in(&combos);
                self.pitch_modifier = self.settings.pitch_modifier_combo.is_in(&combos);
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
                    movement: Movement::Zoom(delta.y * self.settings.zoom_speed,
                                             self.last_cursor_3d),
                };
            }
            Event3d::Frame => {
                if self.forward {
                    Renderer::id() <<
                    MoveEye {
                        scene_id: 0,
                        movement: Movement::Shift(V3::new(5.0 * self.settings.move_speed,
                                                          0.0,
                                                          0.0)),
                    };
                }
                if self.backward {
                    Renderer::id() <<
                    MoveEye {
                        scene_id: 0,
                        movement: Movement::Shift(V3::new(-5.0 * self.settings.move_speed,
                                                          0.0,
                                                          0.0)),
                    };
                }
                if self.left {
                    Renderer::id() <<
                    MoveEye {
                        scene_id: 0,
                        movement: Movement::Shift(V3::new(0.0,
                                                          -5.0 * self.settings.move_speed,
                                                          0.0)),
                    };
                }
                if self.right {
                    Renderer::id() <<
                    MoveEye {
                        scene_id: 0,
                        movement: Movement::Shift(V3::new(0.0,
                                                          5.0 * self.settings.move_speed,
                                                          0.0)),
                    };
                }
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