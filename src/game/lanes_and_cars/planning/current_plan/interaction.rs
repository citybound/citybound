use kay::{Recipient, Actor, Fate};
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
            bindings: Bindings::new(vec![("Materialize Plan", Combo2::new(&[Return], &[])),
                                         ("Undo Step", Combo2::new(&[LControl, Z], &[LWin, Z])),
                                         ("Redo Step",
                                          Combo2::new(&[LControl, LShift, Z],
                                                      &[LWin, LShift, Z])),
                                         ("Spawn Cars", Combo2::new(&[C], &[])),
                                         ("Create Small Grid", Combo2::new(&[G], &[])),
                                         ("Create Large Grid", Combo2::new(&[LShift, G], &[])),
                                         ("Delete Selection", Combo2::new(&[Back], &[Delete]))]),
        }
    }
}

use super::InitInteractable;
use monet::{Renderer, AddEyeListener};
use stagemaster::{UserInterface, AddInteractable, AddInteractable2d, Focus};

impl Recipient<InitInteractable> for CurrentPlan {
    fn receive(&mut self, _msg: &InitInteractable) -> Fate {
        self.interaction.settings = ::ENV.load_settings("Plan Editing");
        UserInterface::id() << AddInteractable(CurrentPlan::id(), AnyShape::Everywhere, 0);
        UserInterface::id() << AddInteractable2d(CurrentPlan::id());
        UserInterface::id() << Focus(CurrentPlan::id());
        Renderer::id() <<
        AddEyeListener {
            scene_id: 0,
            listener: CurrentPlan::id(),
        };
        Fate::Live
    }
}

use monet::EyeMoved;

impl Recipient<EyeMoved> for CurrentPlan {
    fn receive(&mut self, msg: &EyeMoved) -> Fate {
        match *msg {
            EyeMoved { eye, .. } => {
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
                Fate::Live
            }
        }
    }
}

use stagemaster::Event3d;
use super::{Intent, ChangeIntent, IntentProgress, Materialize, Undo, Redo, SetNLanes,
            ToggleBothSides};
use super::stroke_canvas::{Stroke, StrokeState};

impl Recipient<Event3d> for CurrentPlan {
    fn receive(&mut self, msg: &Event3d) -> Fate {
        match *msg {
            Event3d::Combos(combos) => {
                self.interaction
                    .settings
                    .bindings
                    .do_rebinding(&combos.current);
                let bindings = &self.interaction.settings.bindings;

                if bindings["Materialize Plan"].is_freshly_in(&combos) {
                    CurrentPlan::id() << Materialize;
                }

                if bindings["Redo Step"].is_freshly_in(&combos) {
                    CurrentPlan::id() << Redo;
                } else if bindings["Undo Step"].is_freshly_in(&combos) {
                    CurrentPlan::id() << Undo;
                }

                if bindings["Spawn Cars"].is_freshly_in(&combos) {
                    // TODO: this is not supposed to be here!
                    //       *but we have only one focusable!*
                    //       WTF?! what's wrong with your UI model?
                    //       *I uh.. I guess I should actually write a good one*
                    //       When will you finally?!
                    //       *Uh.. next week maybe?*
                    use kay::swarm::{Swarm, ToRandom};
                    use descartes::P3;
                    Swarm::<::game::lanes_and_cars::lane::Lane>::all() <<
                    ToRandom {
                        n_recipients: 5000,
                        message: Event3d::DragFinished {
                            from: P3::new(0.0, 0.0, 0.0),
                            from2d: P2::new(0.0, 0.0),
                            to: P3::new(0.0, 0.0, 0.0),
                            to2d: P2::new(0.0, 0.0),
                        },
                    };
                }

                let maybe_grid_size = if bindings["Create Large Grid"].is_freshly_in(&combos) {
                    Some(15usize)
                } else if bindings["Create Small Grid"]
                              .is_freshly_in(&combos) {
                    Some(10usize)
                } else {
                    None
                };

                if let Some(grid_size) = maybe_grid_size {
                    const GRID_SPACING: N = 1000.0;
                    for x in 0..grid_size {
                        Self::id() <<
                        Stroke(vec![P2::new((x as f32 + 0.5) * GRID_SPACING, 0.0),
                                    P2::new((x as f32 + 0.5) * GRID_SPACING,
                                            grid_size as f32 * GRID_SPACING)]
                                       .into(),
                               StrokeState::Finished);
                    }
                    for y in 0..grid_size {
                        Self::id() <<
                        Stroke(vec![P2::new(0.0, (y as f32 + 0.5) * GRID_SPACING),
                                    P2::new(grid_size as f32 * GRID_SPACING,
                                            (y as f32 + 0.5) * GRID_SPACING)]
                                       .into(),
                               StrokeState::Finished);
                    }
                }

                if bindings["Delete Selection"].is_freshly_in(&combos) {
                    Self::id() << ChangeIntent(Intent::DeleteSelection, IntentProgress::Immediate);
                }

                Fate::Live
            }
            Event3d::ButtonDown(NumberKey(num)) => {
                if num == 0 {
                    Self::id() << ToggleBothSides;
                } else {
                    Self::id() << SetNLanes(num as usize);
                }
                Fate::Live
            }
            _ => Fate::Live,
        }
    }
}

use stagemaster::DrawUI2d;
use stagemaster::Ui2dDrawn;

impl Recipient<DrawUI2d> for CurrentPlan {
    fn receive(&mut self, msg: &DrawUI2d) -> Fate {
        match *msg {
            DrawUI2d { ui_ptr, return_to } => {
                let ui = unsafe { Box::from_raw(ui_ptr as *mut ::imgui::Ui) };

                ui.window(im_str!("Controls"))
                    .build(|| {
                        ui.text(im_str!("Plan Editing"));
                        ui.separator();

                        if self.interaction.settings.bindings.settings_ui(&ui) {
                            ::ENV.write_settings("Plan Editing", &self.interaction.settings)
                        }

                        ui.spacing();
                    });

                return_to << Ui2dDrawn { ui_ptr: Box::into_raw(ui) as usize };
                Fate::Live
            }
        }
    }
}

pub fn setup() {
    CurrentPlan::handle::<InitInteractable>();
    CurrentPlan::handle::<EyeMoved>();
    CurrentPlan::handle::<Event3d>();
    CurrentPlan::handle::<DrawUI2d>();
    CurrentPlan::id() << InitInteractable;
}
