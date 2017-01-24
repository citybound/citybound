use kay::{Recipient, Actor, Fate};
use kay::swarm::{Swarm, ToRandom};
use descartes::{Into2d, P3};
use core::geometry::AnyShape;
use core::settings::{Action, Settings, KeyAction};
pub use monet::glium::glutin::VirtualKeyCode;

use core::user_interface::UserInterface;
use core::ui::{KeyOrButton, KeyCombination};
use super::CurrentPlan;

#[derive(Copy, Clone, Default)]
pub struct Canvas {
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

impl Canvas {
    fn new() -> Canvas {
        let mut canvas = Canvas {
            materialize_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Return)]],
            })),
            undo_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::LControl),
                                KeyOrButton::Key(VirtualKeyCode::RControl),
                                KeyOrButton::Key(VirtualKeyCode::LWin),
                                KeyOrButton::Key(VirtualKeyCode::RWin)],
                           vec![KeyOrButton::Key(VirtualKeyCode::Z)],
                ],
            })),
            redo_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::LControl),
                                KeyOrButton::Key(VirtualKeyCode::RControl),
                                KeyOrButton::Key(VirtualKeyCode::LWin),
                                KeyOrButton::Key(VirtualKeyCode::RWin)],
                           vec![KeyOrButton::Key(VirtualKeyCode::LShift),
                                KeyOrButton::Key(VirtualKeyCode::RShift), ],
                           vec![KeyOrButton::Key(VirtualKeyCode::Z)],
                ],
            })),
            grid_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::G)]],
            })),
            big_grid_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::G)],
                           vec![KeyOrButton::Key(VirtualKeyCode::LShift),
                                KeyOrButton::Key(VirtualKeyCode::RShift)]]
            })),
            spawn_cars_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::C)]],
            })),
            cancel_selection_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Escape),
                                KeyOrButton::Key(VirtualKeyCode::Back)]],
            })),

            toggle_single_sided_action_id: Some(Settings::register_key(KeyCombination {
                keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key0)]]
            })),

            set_lane_width_action_id: [None; 10],
        };
        canvas.set_lane_width_action_id[1] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key1)]],
        }));
        canvas.set_lane_width_action_id[2] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key2)]],
        }));
        canvas.set_lane_width_action_id[3] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key3)]],
        }));
        canvas.set_lane_width_action_id[4] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key4)]],
        }));
        canvas.set_lane_width_action_id[5] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key5)]],
        }));
        canvas.set_lane_width_action_id[6] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key6)]],
        }));
        canvas.set_lane_width_action_id[7] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key7)]],
        }));
        canvas.set_lane_width_action_id[8] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key8)]],
        }));
        canvas.set_lane_width_action_id[9] = Some(Settings::register_key(KeyCombination {
            keys: vec![vec![KeyOrButton::Key(VirtualKeyCode::Key9)]],
        }));
        canvas
    }
}

impl Actor for Canvas {}

use core::user_interface::Event3d;
use super::{Commit, Undo, Redo, WithLatestNode, Materialize, CreateGrid, DeleteSelection,
            SetSelectionMode, SetNLanes, ToggleBothSides};

impl Recipient<Action> for Canvas {
    fn receive(&mut self, msg: &Action) -> Fate {
        match *msg {
            Action::Event3d(event_3d) => {
                match event_3d {
                    Event3d::HoverStarted { at } |
                    Event3d::HoverOngoing { at } => {
                        CurrentPlan::id() << WithLatestNode(at.into_2d(), true);
                    }
                    Event3d::DragStarted { at } => {
                        CurrentPlan::id() << WithLatestNode(at.into_2d(), true);
                        CurrentPlan::id() << Commit(true, at.into_2d());
                    }
                    _ => ()
                }
            }

            Action::KeyDown(KeyAction{action_id: id}) => {
                if Some(id) == self.materialize_action_id {
                    CurrentPlan::id() << Materialize;
                }
                if Some(id) == self.undo_action_id {
                    CurrentPlan::id() << Undo;
                }
                if Some(id) == self.redo_action_id {
                    CurrentPlan::id() << Redo;
                }
                if Some(id) == self.spawn_cars_action_id {
                     Swarm::<::game::lanes_and_cars::lane::Lane>::all() <<
                        ToRandom {
                            n_recipients: 5000,
                            message: Event3d::DragFinished {
                                from: P3::new(0.0, 0.0, 0.0),
                                to: P3::new(0.0, 0.0, 0.0),
                            },
                        };
                }

                if Some(id) == self.grid_action_id {
                    CurrentPlan::id() << CreateGrid(10);
                }

                if Some(id) == self.big_grid_action_id {
                    CurrentPlan::id() << CreateGrid(15);
                }

                if Some(id) == self.cancel_selection_action_id {
                    CurrentPlan::id() << DeleteSelection;
                }

                for i in self.set_lane_width_action_id.into_iter() {
                    if Some(id) == *i {
                        CurrentPlan::id() << SetNLanes(i.unwrap())
                    }
                }

                if Some(id) == self.toggle_single_sided_action_id {
                    CurrentPlan::id() << ToggleBothSides;
                }
            }

            _ => (),
        }
        Fate::Live
    }
}

use monet::EyeMoved;

impl Recipient<EyeMoved> for Canvas {
    fn receive(&mut self, msg: &EyeMoved) -> Fate {
        match *msg {
            EyeMoved { eye, .. } => {
                if eye.position.z < 100.0 {
                    CurrentPlan::id() << SetSelectionMode(false, false);
                } else if eye.position.z < 130.0 {
                    CurrentPlan::id() << SetSelectionMode(true, false);
                } else {
                    CurrentPlan::id() << SetSelectionMode(true, true);
                }
                Fate::Live
            }
        }
    }
}

use core::user_interface::Add;
use core::user_interface::Focus;

#[derive(Copy, Clone)]
struct AddToUI;

impl Recipient<AddToUI> for Canvas {
    fn receive(&mut self, _msg: &AddToUI) -> Fate {
        UserInterface::id() << Add::Interactable3d(Canvas::id(), AnyShape::Everywhere, 0);
        UserInterface::id() << Focus(Canvas::id());
        ::monet::Renderer::id() <<
        ::monet::AddEyeListener {
            scene_id: 0,
            listener: Self::id(),
        };
        Fate::Live
    }
}

pub fn setup() {
    Canvas::register_with_state(Canvas::new());
    Canvas::handle::<Action>();
    Canvas::handle::<EyeMoved>();
    Canvas::handle::<AddToUI>();
    Canvas::id() << AddToUI;
}
