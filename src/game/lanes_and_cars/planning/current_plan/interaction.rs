use kay::{Recipient, Actor, Fate};
use super::CurrentPlan;
use stagemaster::geometry::AnyShape;
use descartes::{N, P2};

#[derive(Default)]
pub struct Interaction {
    cmd_pressed: bool,
    shift_pressed: bool,
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

use stagemaster::{Event3d, VirtualKeyCode};
use super::{Intent, ChangeIntent, IntentProgress, Materialize, Undo, Redo, SetNLanes,
            ToggleBothSides};
use super::stroke_canvas::{Stroke, StrokeState};

impl Recipient<Event3d> for CurrentPlan {
    fn receive(&mut self, msg: &Event3d) -> Fate {
        match *msg {
            Event3d::KeyDown(VirtualKeyCode::Return) => {
                CurrentPlan::id() << Materialize;
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::LControl) |
            Event3d::KeyDown(VirtualKeyCode::RControl) |
            Event3d::KeyDown(VirtualKeyCode::LWin) |
            Event3d::KeyDown(VirtualKeyCode::RWin) => {
                self.interaction.cmd_pressed = true;
                Fate::Live
            }
            Event3d::KeyUp(VirtualKeyCode::LControl) |
            Event3d::KeyUp(VirtualKeyCode::RControl) |
            Event3d::KeyUp(VirtualKeyCode::LWin) |
            Event3d::KeyUp(VirtualKeyCode::RWin) => {
                self.interaction.cmd_pressed = false;
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::LShift) |
            Event3d::KeyDown(VirtualKeyCode::RShift) => {
                self.interaction.shift_pressed = true;
                Fate::Live
            }
            Event3d::KeyUp(VirtualKeyCode::LShift) |
            Event3d::KeyUp(VirtualKeyCode::RShift) => {
                self.interaction.shift_pressed = false;
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Z) => {
                if self.interaction.cmd_pressed {
                    if self.interaction.shift_pressed {
                        CurrentPlan::id() << Redo;
                    } else {
                        CurrentPlan::id() << Undo;
                    }
                }
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::C) => {
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
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::G) => {
                const GRID_SPACING: N = 1000.0;
                let grid_size = if self.interaction.shift_pressed {
                    15usize
                } else {
                    10usize
                };
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
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Back) => {
                Self::id() << ChangeIntent(Intent::DeleteSelection, IntentProgress::Immediate);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key1) => {
                Self::id() << SetNLanes(1);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key2) => {
                Self::id() << SetNLanes(2);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key3) => {
                Self::id() << SetNLanes(3);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key4) => {
                Self::id() << SetNLanes(4);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key5) => {
                Self::id() << SetNLanes(5);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key6) => {
                Self::id() << SetNLanes(6);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key7) => {
                Self::id() << SetNLanes(7);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key8) => {
                Self::id() << SetNLanes(8);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key9) => {
                Self::id() << SetNLanes(9);
                Fate::Live
            }
            Event3d::KeyUp(VirtualKeyCode::Key0) => {
                Self::id() << ToggleBothSides;
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
