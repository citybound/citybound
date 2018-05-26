use kay::{ActorSystem, World, External};
use compact::COption;
use monet::{RendererID, Movement};
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
            bindings: Bindings::new(vec![
                ("Move Forward", Combo2::new(&[Up], &[W])),
                ("Move Backward", Combo2::new(&[Down], &[S])),
                ("Move Left", Combo2::new(&[Left], &[A])),
                ("Move Right", Combo2::new(&[Right], &[D])),
                ("Pan", Combo2::new(&[LShift], &[RShift])),
                ("Yaw", Combo2::new(&[LAlt], &[RightMouseButton])),
                ("Pitch", Combo2::new(&[LAlt], &[RightMouseButton])),
            ]),
        }
    }
}

#[derive(Compact, Clone)]
pub struct CameraControl {
    id: CameraControlID,
    renderer_id: RendererID,
    settings: External<CameraControlSettings>,
    env: Environment,
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

use user_interface::{Event3d, Interactable3d, Interactable3dID, UserInterfaceID};

impl CameraControl {
    pub fn spawn(
        id: CameraControlID,
        renderer_id: RendererID,
        ui_id: UserInterfaceID,
        env: Environment,
        world: &mut World,
    ) -> Self {
        ui_id.add(0, id.into(), COption(None), 0, world);
        ui_id.focus(id.into(), world);
        ui_id.add_2d(id.into(), world);

        CameraControl {
            id,
            renderer_id,
            settings: External::new(env.load_settings("Camera Control")),
            env,
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

impl Interactable3d for CameraControl {
    /// Critical
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        match event {
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
                let old_cursor_2d = self.last_cursor_2d;
                let delta = cursor_2d - self.last_cursor_2d;
                self.last_cursor_2d = cursor_2d;

                if self.yaw_modifier {
                    self.renderer_id.move_eye(
                        Movement::Yaw(-delta.x * self.settings.rotation_speed / 300.0),
                        world,
                    );
                }

                if self.pitch_modifier {
                    self.renderer_id.move_eye(
                        Movement::Pitch(
                            -delta.y * self.settings.rotation_speed *
                                if self.settings.invert_y { -1.0 } else { 1.0 } /
                                300.0,
                        ),
                        world,
                    );
                }

                if self.pan_modifier {
                    self.renderer_id.move_eye(
                        Movement::ShiftProjected(
                            old_cursor_2d,
                            cursor_2d,
                        ),
                        world,
                    )
                }
            }
            Event3d::MouseMove3d(cursor_3d) => {
                self.last_cursor_3d = cursor_3d;
            }
            Event3d::Scroll(delta) => {
                self.renderer_id.move_eye(
                    Movement::Zoom(
                        delta.y * self.settings.zoom_speed,
                        self.last_cursor_3d,
                    ),
                    world,
                );
            }
            Event3d::Frame => {
                if self.forward {
                    self.renderer_id.move_eye(
                        Movement::Shift(
                            V3::new(5.0 * self.settings.move_speed, 0.0, 0.0),
                        ),
                        world,
                    );

                }
                if self.backward {
                    self.renderer_id.move_eye(
                        Movement::Shift(
                            V3::new(-5.0 * self.settings.move_speed, 0.0, 0.0),
                        ),
                        world,
                    );
                }
                if self.left {
                    self.renderer_id.move_eye(
                        Movement::Shift(
                            V3::new(0.0, -5.0 * self.settings.move_speed, 0.0),
                        ),
                        world,
                    );
                }
                if self.right {
                    self.renderer_id.move_eye(
                        Movement::Shift(
                            V3::new(0.0, 5.0 * self.settings.move_speed, 0.0),
                        ),
                        world,
                    );
                }
            }
            _ => {}
        }
    }
}

use user_interface::{Interactable2d, Interactable2dID};
use imgui_sys::ImGuiSetCond_FirstUseEver;

impl Interactable2d for CameraControl {
    /// Critical
    fn draw(&mut self, _: &mut World, ui: &::imgui::Ui<'static>) {
        let mut settings_changed = false;

        ui.window(im_str!("Controls"))
            .size((400.0, 200.0), ImGuiSetCond_FirstUseEver)
            .position((250.0, 10.0), ImGuiSetCond_FirstUseEver)
            .build(|| {
                ui.text(im_str!("Camera Movement"));
                ui.separator();

                ui.text(im_str!("Move Speed"));
                ui.same_line(130.0);
                settings_changed = settings_changed ||
                    ui.slider_float(
                        im_str!("##camera-speed"),
                        &mut self.settings.move_speed,
                        0.1,
                        10.0,
                    ).build();

                settings_changed = settings_changed ||
                    ui.checkbox(im_str!("Invert Y"), &mut self.settings.invert_y);

                settings_changed = settings_changed || self.settings.bindings.settings_ui(&ui);

                ui.spacing();

            });

        if settings_changed {
            self.env.write_settings("Camera Control", &*self.settings);
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<CameraControl>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
