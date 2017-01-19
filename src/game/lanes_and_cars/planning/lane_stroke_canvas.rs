use kay::{Recipient, Actor, Fate};
use kay::swarm::{Swarm, ToRandom};
use descartes::{Into2d, P3};
use core::geometry::AnyShape;
use core::ui::{UserInterface, VirtualKeyCode};
use super::CurrentPlan;

#[derive(Copy, Clone, Default)]
pub struct LaneStrokeCanvas {
    cmd_pressed: bool,
    shift_pressed: bool,
}

impl Actor for LaneStrokeCanvas {}

use core::ui::Event3d;
use super::{Commit, Undo, Redo, WithLatestNode, Materialize, CreateGrid, DeleteSelection,
            SetSelectionMode, SetNLanes, ToggleBothSides};

impl Recipient<Event3d> for LaneStrokeCanvas {
    fn receive(&mut self, msg: &Event3d) -> Fate {
        match *msg {
            Event3d::HoverStarted { at } |
            Event3d::HoverOngoing { at } => {
                CurrentPlan::id() << WithLatestNode(at.into_2d(), true);
                Fate::Live
            }
            Event3d::DragStarted { at } => {
                CurrentPlan::id() << WithLatestNode(at.into_2d(), true);
                CurrentPlan::id() << Commit(true, at.into_2d());
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Return) => {
                CurrentPlan::id() << Materialize;
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::LControl) |
            Event3d::KeyDown(VirtualKeyCode::RControl) |
            Event3d::KeyDown(VirtualKeyCode::LWin) |
            Event3d::KeyDown(VirtualKeyCode::RWin) => {
                self.cmd_pressed = true;
                Fate::Live
            }
            Event3d::KeyUp(VirtualKeyCode::LControl) |
            Event3d::KeyUp(VirtualKeyCode::RControl) |
            Event3d::KeyUp(VirtualKeyCode::LWin) |
            Event3d::KeyUp(VirtualKeyCode::RWin) => {
                self.cmd_pressed = false;
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::LShift) |
            Event3d::KeyDown(VirtualKeyCode::RShift) => {
                self.shift_pressed = true;
                Fate::Live
            }
            Event3d::KeyUp(VirtualKeyCode::LShift) |
            Event3d::KeyUp(VirtualKeyCode::RShift) => {
                self.shift_pressed = false;
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Z) => {
                if self.cmd_pressed {
                    if self.shift_pressed {
                        CurrentPlan::id() << Redo;
                    } else {
                        CurrentPlan::id() << Undo;
                    }
                }
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::C) => {
                Swarm::<::game::lanes_and_cars::Lane>::all() <<
                ToRandom {
                    n_recipients: 5000,
                    message: Event3d::DragFinished {
                        from: P3::new(0.0, 0.0, 0.0),
                        to: P3::new(0.0, 0.0, 0.0),
                    },
                };
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::G) => {
                CurrentPlan::id() << CreateGrid(if self.shift_pressed { 15 } else { 10 });
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Back) => {
                CurrentPlan::id() << DeleteSelection;
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key1) => {
                CurrentPlan::id() << SetNLanes(1);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key2) => {
                CurrentPlan::id() << SetNLanes(2);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key3) => {
                CurrentPlan::id() << SetNLanes(3);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key4) => {
                CurrentPlan::id() << SetNLanes(4);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key5) => {
                CurrentPlan::id() << SetNLanes(5);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key6) => {
                CurrentPlan::id() << SetNLanes(6);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key7) => {
                CurrentPlan::id() << SetNLanes(7);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key8) => {
                CurrentPlan::id() << SetNLanes(8);
                Fate::Live
            }
            Event3d::KeyDown(VirtualKeyCode::Key9) => {
                CurrentPlan::id() << SetNLanes(9);
                Fate::Live
            }
            Event3d::KeyUp(VirtualKeyCode::Key0) => {
                CurrentPlan::id() << ToggleBothSides;
                Fate::Live
            }
            _ => Fate::Live,
        }
    }
}

use ::monet::EyeMoved;

impl Recipient<EyeMoved> for LaneStrokeCanvas {
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

use core::ui::Add;
use core::ui::Focus;

#[derive(Copy, Clone)]
struct AddToUI;

impl Recipient<AddToUI> for LaneStrokeCanvas {
    fn receive(&mut self, _msg: &AddToUI) -> Fate {
        UserInterface::id() << Add::Interactable3d(LaneStrokeCanvas::id(), AnyShape::Everywhere, 0);
        UserInterface::id() << Focus(LaneStrokeCanvas::id());
        ::monet::Renderer::id() <<
        ::monet::AddEyeListener {
            scene_id: 0,
            listener: Self::id(),
        };
        Fate::Live
    }
}

pub fn setup() {
    LaneStrokeCanvas::register_with_state(LaneStrokeCanvas {
        cmd_pressed: false,
        shift_pressed: false,
    });
    LaneStrokeCanvas::handle::<Event3d>();
    LaneStrokeCanvas::handle::<EyeMoved>();
    LaneStrokeCanvas::handle::<AddToUI>();
    LaneStrokeCanvas::id() << AddToUI;
}
