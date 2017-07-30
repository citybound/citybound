use kay::{ActorSystem, Fate};
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

pub fn setup(system: &mut ActorSystem, env: &'static Environment) {
    let settings = env.load_settings("Camera Control");
    let state = CameraControl::new(settings, env);

    use super::Event3d;

    system.add(state, |mut the_renderer| {
        // TODO: ugly/wrong
        let renderer_id = RendererID::broadcast(&mut the_renderer.world());

        the_renderer.on_critical(move |event, cc, world| {
            match *event {
                Event3d::Combos(combos) => {
                    cc.settings.bindings.do_rebinding(&combos.current);
                    cc.forward = cc.settings.bindings["Move Forward"].is_in(&combos);
                    cc.backward = cc.settings.bindings["Move Backward"].is_in(&combos);
                    cc.left = cc.settings.bindings["Move Left"].is_in(&combos);
                    cc.right = cc.settings.bindings["Move Right"].is_in(&combos);
                    cc.pan_modifier = cc.settings.bindings["Pan"].is_in(&combos);
                    cc.yaw_modifier = cc.settings.bindings["Yaw"].is_in(&combos);
                    cc.pitch_modifier = cc.settings.bindings["Pitch"].is_in(&combos);
                }
                Event3d::MouseMove(cursor_2d) => {
                    let delta = cursor_2d - cc.last_cursor_2d;
                    cc.last_cursor_2d = cursor_2d;

                    if cc.yaw_modifier {
                        renderer_id.move_eye(
                            0,
                            Movement::Yaw(-delta.x * cc.settings.rotation_speed / 300.0),
                            world,
                        );
                    }

                    if cc.pitch_modifier {
                        renderer_id.move_eye(
                            0,
                            Movement::Pitch(
                                -delta.y * cc.settings.rotation_speed *
                                    if cc.settings.invert_y { -1.0 } else { 1.0 } /
                                    300.0,
                            ),
                            world,
                        );
                    }
                }
                Event3d::MouseMove3d(cursor_3d) => {
                    let delta = cursor_3d - cc.last_cursor_3d;
                    cc.last_cursor_3d = cursor_3d;

                    if cc.pan_modifier {
                        renderer_id.move_eye(0, Movement::ShiftAbsolute(-delta), world);
                        // predict next movement to avoid jitter
                        cc.last_cursor_3d -= delta;
                    }
                }
                Event3d::Scroll(delta) => {
                    renderer_id.move_eye(
                        0,
                        Movement::Zoom(delta.y * cc.settings.zoom_speed, cc.last_cursor_3d),
                        world,
                    );
                }
                Event3d::Frame => {
                    if cc.forward {
                        renderer_id.move_eye(
                            0,
                            Movement::Shift(V3::new(5.0 * cc.settings.move_speed, 0.0, 0.0)),
                            world,
                        );

                    }
                    if cc.backward {
                        renderer_id.move_eye(
                            0,
                            Movement::Shift(
                                V3::new(-5.0 * cc.settings.move_speed, 0.0, 0.0),
                            ),
                            world,
                        );
                    }
                    if cc.left {
                        renderer_id.move_eye(
                            0,
                            Movement::Shift(
                                V3::new(0.0, -5.0 * cc.settings.move_speed, 0.0),
                            ),
                            world,
                        );
                    }
                    if cc.right {
                        renderer_id.move_eye(
                            0,
                            Movement::Shift(V3::new(0.0, 5.0 * cc.settings.move_speed, 0.0)),
                            world,
                        );
                    }
                }
                _ => {}
            }
            Fate::Live
        });


        use super::DrawUI2d;
        use super::Ui2dDrawn;
        use imgui::ImGuiSetCond_FirstUseEver;

        the_renderer.on_critical(|&DrawUI2d { ref imgui_ui, return_to }, cc, world| {
            let ui = imgui_ui.steal();

            let mut settings_changed = false;

            ui.window(im_str!("Controls"))
                .size((600.0, 200.0), ImGuiSetCond_FirstUseEver)
                .collapsible(false)
                .build(|| {
                    ui.text(im_str!("Camera Movement"));
                    ui.separator();

                    ui.text(im_str!("Move Speed"));
                    ui.same_line(150.0);
                    settings_changed = settings_changed ||
                        ui.slider_float(im_str!(""), &mut cc.settings.move_speed, 0.1, 10.0)
                            .build();

                    settings_changed = settings_changed ||
                        ui.checkbox(im_str!("Invert Y"), &mut cc.settings.invert_y);

                    settings_changed = settings_changed || cc.settings.bindings.settings_ui(&ui);

                    ui.spacing();

                });

            if settings_changed {
                cc.env.write_settings("Camera Control", &cc.settings);
            }

            world.send(return_to, Ui2dDrawn { imgui_ui: ui });
            Fate::Live
        });

        let ui_id = the_renderer.world().id::<super::UserInterface>();
        let cc_id = the_renderer.world().id::<CameraControl>();

        the_renderer.world().send(
            ui_id,
            super::AddInteractable(cc_id, super::AnyShape::Everywhere, 0),
        );
        the_renderer.world().send(
            ui_id,
            super::AddInteractable2d(cc_id),
        );
        the_renderer.world().send(ui_id, super::Focus(cc_id));
    });
}
