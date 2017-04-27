use kay::{Recipient, Actor, Fate};
use super::CurrentPlan;
use stagemaster::geometry::AnyShape;
use stagemaster::combo::{Combo, Combo2};
use stagemaster::combo::Button::*;
use descartes::{N, P2};

#[derive(Serialize, Deserialize)]
pub struct InteractionSettings {
    materialize_combo: Combo,
    undo_combo: Combo2,
    redo_combo: Combo2,
    spawn_car_combo: Combo,
    create_small_grid_combo: Combo,
    create_large_grid_combo: Combo,
    delete_combo: Combo2,
}

impl Default for InteractionSettings {
    fn default() -> Self {
        InteractionSettings {
            materialize_combo: Combo::new(&[Return]),
            undo_combo: Combo2::new(&[LControl, Z], &[LWin, Z]),
            redo_combo: Combo2::new(&[LControl, LShift, Z], &[LWin, LShift, Z]),
            spawn_car_combo: Combo::new(&[C]),
            create_small_grid_combo: Combo::new(&[G]),
            create_large_grid_combo: Combo::new(&[LShift, G]),
            delete_combo: Combo2::new(&[Back], &[Delete]),
        }
    }
}

use super::InitInteractable;
use monet::{Renderer, AddEyeListener};
use stagemaster::{UserInterface, AddInteractable, Focus};

impl Recipient<InitInteractable> for CurrentPlan {
    fn receive(&mut self, _msg: &InitInteractable) -> Fate {
        UserInterface::id() << AddInteractable(CurrentPlan::id(), AnyShape::Everywhere, 0);
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
                if self.interaction.materialize_combo.is_freshly_in(&combos) {
                    CurrentPlan::id() << Materialize;
                }

                if self.interaction.redo_combo.is_freshly_in(&combos) {
                    CurrentPlan::id() << Redo;
                } else if self.interaction.undo_combo.is_freshly_in(&combos) {
                    CurrentPlan::id() << Undo;
                }

                if self.interaction.spawn_car_combo.is_freshly_in(&combos) {
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

                let maybe_grid_size =
                    if self.interaction.create_large_grid_combo.is_freshly_in(&combos) {
                        Some(15usize)
                    } else if self.interaction
                        .create_small_grid_combo
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

                if self.interaction.delete_combo.is_freshly_in(&combos) {
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

pub fn setup() {
    CurrentPlan::handle::<InitInteractable>();
    CurrentPlan::handle::<EyeMoved>();
    CurrentPlan::handle::<Event3d>();
    CurrentPlan::id() << InitInteractable;
}
