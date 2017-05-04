use kay::{Actor, Recipient, Fate};
use monet::{Renderer, MoveEye, Movement};
use descartes::{P2, P3, V3};
use combo::Button::*;
use super::combo::{Bindings, Combo2};
use super::environment::Environment;

#[derive(Serialize, Deserialize, Clone)]
pub struct CameraControlSettings {
    pub rotation_speed: f32,
    pub move_speed: f32,
    pub zoom_speed: f32,
    pub invert_y: bool,

    pub bindings: Bindings,
}

impl Default for CameraControlSettings {
    fn default() -> Self {
        CameraControlSettings {
            rotation_speed: 1.0f32,
            zoom_speed: 1.0f32,
            move_speed: 1.0f32,
            invert_y: false,
            bindings: Bindings::new(vec![("Move Forward", Combo2::new(&[Up], &[W])),
                                         ("Move Backward", Combo2::new(&[Down], &[S])),
                                         ("Move Left", Combo2::new(&[Left], &[A])),
                                         ("Move Right", Combo2::new(&[Right], &[D])),
                                         ("Pan", Combo2::new(&[LShift], &[RShift])),
                                         ("Yaw", Combo2::new(&[LAlt], &[RightMouseButton])),
                                         ("Pitch", Combo2::new(&[LAlt], &[RightMouseButton]))]),
        }
    }
}

pub struct CameraControl {
    settings: CameraControlSettings,
    env: &'static Environment,
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
    fn new(settings: CameraControlSettings, env: &'static Environment) -> Self {
        CameraControl {
            settings: settings,
            env: env,
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
                self.settings.bindings.do_rebinding(&combos.current);
                self.forward = self.settings.bindings["Move Forward"].is_in(&combos);
                self.backward = self.settings.bindings["Move Backward"].is_in(&combos);
                self.left = self.settings.bindings["Move Left"].is_in(&combos);
                self.right = self.settings.bindings["Move Right"].is_in(&combos);
                self.pan_modifier = self.settings.bindings["Pan"].is_in(&combos);
                self.yaw_modifier = self.settings.bindings["Yaw"].is_in(&combos);
                self.pitch_modifier = self.settings.bindings["Pitch"].is_in(&combos);
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
                                                  if self.settings.invert_y { -1.0 } else { 1.0 } /
                                                  300.0),
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

use super::DrawUI2d;
use super::Ui2dDrawn;
use imgui::ImGuiSetCond_FirstUseEver;

impl Recipient<DrawUI2d> for CameraControl {
    fn receive(&mut self, msg: &DrawUI2d) -> Fate {
        match *msg {
            DrawUI2d { ui_ptr, return_to } => {
                let ui = unsafe { Box::from_raw(ui_ptr as *mut ::imgui::Ui) };

                let mut settings_changed = false;

                ui.window(im_str!("Controls"))
                    .size((600.0, 200.0), ImGuiSetCond_FirstUseEver)
                    .collapsible(false)
                    .build(|| {
                        ui.text(im_str!("Camera Movement"));
                        ui.separator();

                        ui.text(im_str!("Move Speed"));
                        ui.same_line(150.0);
                        settings_changed =
                            settings_changed ||
                            ui.slider_float(im_str!(""), &mut self.settings.move_speed, 0.1, 10.0)
                                .build();

                        settings_changed = settings_changed ||
                                           ui.checkbox(im_str!("Invert Y"),
                                                       &mut self.settings.invert_y);

                        settings_changed = settings_changed ||
                                           self.settings.bindings.settings_ui(&ui);

                        ui.spacing();

                    });

                if settings_changed {
                    self.env.write_settings("Camera Control", &self.settings);
                }

                return_to << Ui2dDrawn { ui_ptr: Box::into_raw(ui) as usize };
                Fate::Live
            }
        }
    }
}

pub fn setup(env: &'static Environment) {
    let settings = env.load_settings("Camera Control");
    let state = CameraControl::new(settings, env);
    CameraControl::register_with_state(state);
    CameraControl::handle_critically::<Event3d>();
    CameraControl::handle_critically::<DrawUI2d>();
    super::UserInterface::id() <<
    super::AddInteractable(CameraControl::id(), super::AnyShape::Everywhere, 0);
    super::UserInterface::id() << super::AddInteractable2d(CameraControl::id());
    super::UserInterface::id() << super::Focus(CameraControl::id());
}
