use kay::{Recipient, Actor, Fate};
use core::ui::{KeyOrButton, KeyCombination, VirtualKeyCode};
use core::settings::{Settings, KeyAction};
use super::CurrentPlan;
use core::geometry::AnyShape;

pub struct Interaction {
    materialize_action_id: Option<usize>,
    undo_action_id: Option<usize>,
    redo_action_id: Option<usize>,
    grid_action_id: Option<usize>,
    big_grid_action_id: Option<usize>,
    toggle_single_sided_action_id: Option<usize>,
    spawn_cars_action_id: Option<usize>,
    cancel_selection_action_id: Option<usize>,

    set_lane_width_action_id: [Option<usize>; 10],
}

impl Default for Interaction {
    fn default() -> Interaction {
        let mut interaction = Interaction {
            materialize_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Return)]],
            })),
            undo_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::LControl),
                                KeyOrButton::Key(VirtualKeyCode::RControl),
                                KeyOrButton::Key(VirtualKeyCode::LWin),
                                KeyOrButton::Key(VirtualKeyCode::RWin)],
                           vec![KeyOrButton::Key(VirtualKeyCode::Z)]],
            })),
            redo_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::LControl),
                                KeyOrButton::Key(VirtualKeyCode::RControl),
                                KeyOrButton::Key(VirtualKeyCode::LWin),
                                KeyOrButton::Key(VirtualKeyCode::RWin)],
                           vec![KeyOrButton::Key(VirtualKeyCode::LShift),
                                KeyOrButton::Key(VirtualKeyCode::RShift)],
                           vec![KeyOrButton::Key(VirtualKeyCode::Z)]],
            })),
            grid_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::G)]],
            })),
            big_grid_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::G)],
                           vec![KeyOrButton::Key(VirtualKeyCode::LShift),
                                KeyOrButton::Key(VirtualKeyCode::RShift)]],
            })),
            spawn_cars_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::C)]],
            })),
            cancel_selection_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Escape),
                                KeyOrButton::Key(VirtualKeyCode::Back)]],
            })),

            toggle_single_sided_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key0)]],
            })),

            set_lane_width_action_id: [None; 10],
        };
        interaction.set_lane_width_action_id[1] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key1)]],
        }));
        interaction.set_lane_width_action_id[2] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key2)]],
        }));
        interaction.set_lane_width_action_id[3] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key3)]],
        }));
        interaction.set_lane_width_action_id[4] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key4)]],
        }));
        interaction.set_lane_width_action_id[5] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key5)]],
        }));
        interaction.set_lane_width_action_id[6] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key6)]],
        }));
        interaction.set_lane_width_action_id[7] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key7)]],
        }));
        interaction.set_lane_width_action_id[8] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key8)]],
        }));
        interaction.set_lane_width_action_id[9] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key9)]],
        }));

        Settings::register_exclusiveness(interaction.big_grid_action_id.unwrap(),
                                         interaction.grid_action_id.unwrap());
        Settings::register_exclusiveness(interaction.redo_action_id.unwrap(),
                                         interaction.undo_action_id.unwrap());
        interaction
    }
}

use super::InitInteractable;
use monet::{Renderer, AddEyeListener};
use core::user_interface::{UserInterface, Add, Focus};

impl Recipient<InitInteractable> for CurrentPlan {
    fn receive(&mut self, _msg: &InitInteractable) -> Fate {
        UserInterface::id() << Add::Interactable3d(CurrentPlan::id(), AnyShape::Everywhere, 0);
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

use core::settings::Action;
use core::user_interface::Event3d;
use super::{Intent, ChangeIntent, IntentProgress, Undo, Redo, SetNLanes, ToggleBothSides};

impl Recipient<Action> for CurrentPlan {
    fn receive(&mut self, msg: &Action) -> Fate {
        match *msg {
            Action::KeyDown(KeyAction { action_id: id }) => {
                println!("key down in interaction!");
                if Some(id) == self.interaction.materialize_action_id {
                    //CurrentPlan::id() << Materialize;
                }
                if Some(id) == self.interaction.undo_action_id {
                    println!("Undo");
                    CurrentPlan::id() << Undo;
                }
                if Some(id) == self.interaction.redo_action_id {
                    println!("Redo");
                    CurrentPlan::id() << Redo;
                }
                if Some(id) == self.interaction.spawn_cars_action_id {
                    //  Swarm::<::game::lanes_and_cars::lane::Lane>::all() <<
                    //     ToRandom {
                    //         n_recipients: 5000,
                    //         message: Event3d::DragFinished {
                    //             from: P3::new(0.0, 0.0, 0.0),
                    //             to: P3::new(0.0, 0.0, 0.0),
                    //         },
                    //     };
                }

                if Some(id) == self.interaction.grid_action_id {
                    // println!("Grid");
                    // CurrentPlan::id() << CreateGrid(10);
                }

                if Some(id) == self.interaction.big_grid_action_id {
                    // println!("Grid");
                    // CurrentPlan::id() << CreateGrid(15);
                }

                if Some(id) == self.interaction.cancel_selection_action_id {
                    CurrentPlan::id() <<
                    ChangeIntent(Intent::DeleteSelection, IntentProgress::Finished);
                }

                for (n, n_lane_id) in self.interaction
                    .set_lane_width_action_id
                    .iter()
                    .enumerate() {
                    if Some(id) == *n_lane_id {
                        CurrentPlan::id() << SetNLanes(n)
                    }
                }

                if Some(id) == self.interaction.toggle_single_sided_action_id {
                    CurrentPlan::id() << ToggleBothSides;
                }
            }

            _ => (),
        }
        Fate::Live
    }
}

pub fn setup() {
    CurrentPlan::handle::<InitInteractable>();
    CurrentPlan::handle::<EyeMoved>();
    CurrentPlan::handle::<Action>();
    CurrentPlan::id() << InitInteractable;
}