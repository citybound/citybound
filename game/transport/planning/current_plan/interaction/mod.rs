use kay::{ActorSystem, Fate, External, World};
use kay::swarm::Swarm;
use super::{CurrentPlan, CurrentPlanID};
use stagemaster::geometry::AnyShape;
use stagemaster::combo::{Bindings, Combo2};
use stagemaster::combo::Button::*;
use descartes::{N, P2};

#[derive(Compact, Clone)]
pub struct Interaction {
    pub user_interface: UserInterfaceID,
    settings: External<InteractionSettings>,
}

#[derive(Serialize, Deserialize, Clone)]
struct InteractionSettings {
    bindings: Bindings,
}

impl Default for InteractionSettings {
    fn default() -> Self {
        InteractionSettings {
            bindings: Bindings::new(vec![
                ("Materialize Plan", Combo2::new(&[Return], &[])),
                ("Undo Step", Combo2::new(&[LControl, Z], &[LWin, Z])),
                (
                    "Redo Step",
                    Combo2::new(&[LControl, LShift, Z], &[LWin, LShift, Z])
                ),
                ("Spawn Cars", Combo2::new(&[C], &[])),
                ("Create Small Grid", Combo2::new(&[G], &[])),
                ("Create Large Grid", Combo2::new(&[LShift, G], &[])),
                ("Delete Selection", Combo2::new(&[Back], &[Delete])),
            ]),
        }
    }
}

use monet::{RendererID, EyeListener, Eye, Movement, EyeListenerID, MSG_EyeListener_eye_moved};
use stagemaster::UserInterfaceID;
use transport::lane::Lane;

impl Interaction {
    pub fn init(
        world: &mut World,
        user_interface: UserInterfaceID,
        renderer_id: RendererID,
        id: CurrentPlanID,
    ) -> Interaction {
        user_interface.add(id.into(), AnyShape::Everywhere, 0, world);
        user_interface.add_2d(id.into(), world);
        user_interface.focus(id.into(), world);
        renderer_id.add_eye_listener(0, id.into(), world);
        Interaction {
            settings: External::new(::ENV.load_settings("Plan Editing")),
            user_interface,
        }
    }
}

impl EyeListener for CurrentPlan {
    fn eye_moved(&mut self, eye: Eye, _movement: Movement, _: &mut World) {
        if eye.position.z < 100.0 {
            self.settings.select_parallel = false;
            self.settings.select_opposite = false;
        } else if eye.position.z < 130.0 {
            self.settings.select_parallel = true;
            self.settings.select_opposite = false;
        } else {
            self.settings.select_parallel = true;
            self.settings.select_opposite = true;
        }
    }
}

use stagemaster::{Interactable3d, Interactable3dID, Interactable2d, Interactable2dID, Event3d,
                  MSG_Interactable3d_on_event, MSG_Interactable2d_draw_ui_2d};
use super::{StrokeState, Intent, IntentProgress};

impl Interactable3d for CurrentPlan {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        match event {
            Event3d::Combos(combos) => {
                self.interaction.settings.bindings.do_rebinding(
                    &combos.current,
                );
                let bindings = &self.interaction.settings.bindings;

                if bindings["Materialize Plan"].is_freshly_in(&combos) {
                    self.id.materialize(world);
                }

                if bindings["Redo Step"].is_freshly_in(&combos) {
                    self.id.redo(world);
                } else if bindings["Undo Step"].is_freshly_in(&combos) {
                    self.id.undo(world);
                }

                if bindings["Spawn Cars"].is_freshly_in(&combos) {
                    // TODO: this is not supposed to be here!
                    //       *but we have only one focusable!*
                    //       WTF?! what's wrong with your UI model?
                    //       *I uh.. I guess I should actually write a good one*
                    //       When will you finally?!
                    //       *Uh.. next week maybe?*
                    use kay::swarm::ToRandom;
                    use descartes::P3;
                    world.send_to_id_of::<Swarm<Lane>, _>(ToRandom {
                        n_recipients: 5000,
                        message: Event3d::DragFinished {
                            from: P3::new(0.0, 0.0, 0.0),
                            from2d: P2::new(0.0, 0.0),
                            to: P3::new(0.0, 0.0, 0.0),
                            to2d: P2::new(0.0, 0.0),
                        },
                    });
                }

                let maybe_grid_size = if bindings["Create Large Grid"].is_freshly_in(&combos) {
                    Some(15usize)
                } else if bindings["Create Small Grid"].is_freshly_in(&combos) {
                    Some(10usize)
                } else {
                    None
                };

                if let Some(grid_size) = maybe_grid_size {
                    const GRID_SPACING: N = 1000.0;
                    for x in 0..grid_size {
                        self.id.on_stroke(
                            vec![
                                P2::new((x as f32 + 0.5) * GRID_SPACING, 0.0),
                                P2::new(
                                    (x as f32 + 0.5) * GRID_SPACING,
                                    grid_size as f32 * GRID_SPACING
                                ),
                            ].into(),
                            StrokeState::Finished,
                            world,
                        );

                    }
                    for y in 0..grid_size {
                        self.id.on_stroke(
                            vec![
                                P2::new(0.0, (y as f32 + 0.5) * GRID_SPACING),
                                P2::new(
                                    grid_size as f32 * GRID_SPACING,
                                    (y as f32 + 0.5) * GRID_SPACING
                                ),
                            ].into(),
                            StrokeState::Finished,
                            world,
                        );
                    }
                }

                if bindings["Delete Selection"].is_freshly_in(&combos) {
                    self.id.change_intent(
                        Intent::DeleteSelection,
                        IntentProgress::Immediate,
                        world,
                    );
                }
            }
            Event3d::ButtonDown(NumberKey(num)) => {
                if num == 0 {
                    self.id.toggle_both_sides(world);
                } else {
                    self.id.set_n_lanes(num as usize, world);
                }
            }
            _ => {}
        };
    }
}

impl Interactable2d for CurrentPlan {
    fn draw_ui_2d(
        &mut self,
        imgui_ui: &External<::imgui::Ui<'static>>,
        return_to: UserInterfaceID,
        world: &mut World,
    ) {
        let ui = imgui_ui.steal();

        ui.window(im_str!("Controls")).build(|| {
            ui.text(im_str!("Plan Editing"));
            ui.separator();

            if self.interaction.settings.bindings.settings_ui(&ui) {
                ::ENV.write_settings("Plan Editing", &*self.interaction.settings)
            }

            ui.spacing();
        });

        return_to.ui_drawn(ui, world);
    }
}

pub fn setup(system: &mut ActorSystem) {
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;