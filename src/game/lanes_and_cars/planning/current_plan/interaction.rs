use kay::{ActorSystem, Fate};
use kay::swarm::Swarm;
use super::CurrentPlan;
use stagemaster::geometry::AnyShape;
use stagemaster::combo::{Bindings, Combo2};
use stagemaster::combo::Button::*;
use descartes::{N, P2};

#[derive(Default, Clone)]
pub struct Interaction {
    settings: InteractionSettings,
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

use super::InitInteractable;
use monet::{RendererID, EyeListenerID, MSG_EyeListener_eye_moved};
use stagemaster::{UserInterface, AddInteractable, AddInteractable2d, Focus};
use game::lanes_and_cars::lane::Lane;

pub fn setup(system: &mut ActorSystem) {
    system.extend::<CurrentPlan, _>(|mut the_cp| {
        let ui_id = the_cp.world().id::<UserInterface>();
        let cp_id = the_cp.world().id::<CurrentPlan>();
        let lanes_swarm_id = the_cp.world().id::<Swarm<Lane>>();

        the_cp.on(move |_: &InitInteractable, plan, world| {
            plan.interaction.settings = ::ENV.load_settings("Plan Editing");
            world.send(ui_id, AddInteractable(cp_id, AnyShape::Everywhere, 0));
            world.send(ui_id, AddInteractable2d(cp_id));
            world.send(ui_id, Focus(cp_id));
            // TODO: ugly/wrong
            RendererID::broadcast(world).add_eye_listener(
                0,
                EyeListenerID { _raw_id: cp_id },
                world,
            );
            Fate::Live
        });

        the_cp.on(|&MSG_EyeListener_eye_moved(eye, ..), plan, _| {
            if eye.position.z < 100.0 {
                plan.settings.select_parallel = false;
                plan.settings.select_opposite = false;
            } else if eye.position.z < 130.0 {
                plan.settings.select_parallel = true;
                plan.settings.select_opposite = false;
            } else {
                plan.settings.select_parallel = true;
                plan.settings.select_opposite = true;
            }
            Fate::Live
        });

        the_cp.on(move |event, plan, world| {
            match *event {
                Event3d::Combos(combos) => {
                    plan.interaction.settings.bindings.do_rebinding(
                        &combos.current,
                    );
                    let bindings = &plan.interaction.settings.bindings;

                    if bindings["Materialize Plan"].is_freshly_in(&combos) {
                        world.send(cp_id, Materialize);
                    }

                    if bindings["Redo Step"].is_freshly_in(&combos) {
                        world.send(cp_id, Redo);
                    } else if bindings["Undo Step"].is_freshly_in(&combos) {
                        world.send(cp_id, Undo);
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
                        world.send(
                            lanes_swarm_id,
                            ToRandom {
                                n_recipients: 5000,
                                message: Event3d::DragFinished {
                                    from: P3::new(0.0, 0.0, 0.0),
                                    from2d: P2::new(0.0, 0.0),
                                    to: P3::new(0.0, 0.0, 0.0),
                                    to2d: P2::new(0.0, 0.0),
                                },
                            },
                        );
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
                            world.send(
                                cp_id,
                                Stroke(
                                    vec![
                                        P2::new((x as f32 + 0.5) * GRID_SPACING, 0.0),
                                        P2::new(
                                            (x as f32 + 0.5) * GRID_SPACING,
                                            grid_size as f32 * GRID_SPACING
                                        ),
                                    ].into(),
                                    StrokeState::Finished,
                                ),
                            );
                        }
                        for y in 0..grid_size {
                            world.send(
                                cp_id,
                                Stroke(
                                    vec![
                                        P2::new(0.0, (y as f32 + 0.5) * GRID_SPACING),
                                        P2::new(
                                            grid_size as f32 * GRID_SPACING,
                                            (y as f32 + 0.5) * GRID_SPACING
                                        ),
                                    ].into(),
                                    StrokeState::Finished,
                                ),
                            );
                        }
                    }

                    if bindings["Delete Selection"].is_freshly_in(&combos) {
                        world.send(
                            cp_id,
                            ChangeIntent(Intent::DeleteSelection, IntentProgress::Immediate),
                        );
                    }
                }
                Event3d::ButtonDown(NumberKey(num)) => {
                    if num == 0 {
                        world.send(cp_id, ToggleBothSides);
                    } else {
                        world.send(cp_id, SetNLanes(num as usize));
                    }
                }
                _ => {}
            };
            Fate::Live
        });

        the_cp.on(|&DrawUI2d { ui_ptr, return_to }, plan, world| {
            let ui = unsafe { Box::from_raw(ui_ptr as *mut ::imgui::Ui) };

            ui.window(im_str!("Controls")).build(|| {
                ui.text(im_str!("Plan Editing"));
                ui.separator();

                if plan.interaction.settings.bindings.settings_ui(&ui) {
                    ::ENV.write_settings("Plan Editing", &plan.interaction.settings)
                }

                ui.spacing();
            });

            world.send(return_to, Ui2dDrawn { ui_ptr: Box::into_raw(ui) as usize });
            Fate::Live
        });

        the_cp.world().send(cp_id, InitInteractable);
    });
}

use stagemaster::Event3d;
use super::{Intent, ChangeIntent, IntentProgress, Materialize, Undo, Redo, SetNLanes,
            ToggleBothSides};
use super::stroke_canvas::{Stroke, StrokeState};

use stagemaster::DrawUI2d;
use stagemaster::Ui2dDrawn;
