use kay::{Swarm, ToRandom, Recipient, ActorSystem, Individual, Fate};
use descartes::{Into2d, P3};
use core::geometry::AnyShape;
use core::ui::{UserInterface, VirtualKeyCode};
use super::{CurrentPlan};

#[derive(Copy, Clone, Default)]
pub struct LaneStrokeCanvas{
    cmd_pressed: bool,
    shift_pressed: bool
}

impl Individual for LaneStrokeCanvas{}

use core::ui::Event3d;
use super::PlanControl::{Commit, Undo, Redo, WithLatestNode, Materialize, CreateGrid, DeleteSelection};

impl Recipient<Event3d> for LaneStrokeCanvas {
    fn receive(&mut self, msg: &Event3d) -> Fate {match *msg {
        Event3d::HoverStarted{at} | Event3d::HoverOngoing{at} => {
            CurrentPlan::id() << WithLatestNode(at.into_2d(), true);
            Fate::Live
        },
        Event3d::DragStarted{at} => {
            CurrentPlan::id() << WithLatestNode(at.into_2d(), true);
            CurrentPlan::id() << Commit(true, at.into_2d());
            Fate::Live
        },
        Event3d::DragFinished{..} => {
            Fate::Live
        },
        Event3d::KeyDown(VirtualKeyCode::Return) => {
            CurrentPlan::id() << Materialize(());
            Fate::Live
        },
        Event3d::KeyDown(VirtualKeyCode::LControl) | Event3d::KeyDown(VirtualKeyCode::RControl)
            | Event3d::KeyDown(VirtualKeyCode::LWin) | Event3d::KeyDown(VirtualKeyCode::RWin) => {
            self.cmd_pressed = true;
            Fate::Live
        },
        Event3d::KeyUp(VirtualKeyCode::LControl) | Event3d::KeyUp(VirtualKeyCode::RControl)
            | Event3d::KeyUp(VirtualKeyCode::LWin) | Event3d::KeyUp(VirtualKeyCode::RWin) => {
            self.cmd_pressed = false;
            Fate::Live
        },
        Event3d::KeyDown(VirtualKeyCode::LShift) | Event3d::KeyDown(VirtualKeyCode::RShift) => {
            self.shift_pressed = true;
            Fate::Live
        },
        Event3d::KeyUp(VirtualKeyCode::LShift) | Event3d::KeyUp(VirtualKeyCode::RShift) => {
            self.shift_pressed = false;
            Fate::Live
        },
        Event3d::KeyDown(VirtualKeyCode::Z) => {
            if self.cmd_pressed {
                if self.shift_pressed {
                    CurrentPlan::id() << Redo(());
                } else {
                    CurrentPlan::id() << Undo(());
                }
            }
            Fate::Live
        },
        Event3d::KeyDown(VirtualKeyCode::C) => {
            Swarm::<::game::lanes_and_cars::Lane>::all() << ToRandom{
                n_recipients: 5000,
                message: Event3d::DragFinished{from: P3::new(0.0, 0.0, 0.0), to: P3::new(0.0, 0.0, 0.0)
            }};
            Fate::Live
        },
        Event3d::KeyDown(VirtualKeyCode::G) => {
            CurrentPlan::id() << CreateGrid(());
            Fate::Live
        },
        Event3d::KeyDown(VirtualKeyCode::Back) => {
            CurrentPlan::id() << DeleteSelection(());
            Fate::Live
        }
        _ => Fate::Live
    }}
}

use core::ui::Add;
use core::ui::Focus;

#[derive(Copy, Clone)]
struct AddToUI;

impl Recipient<AddToUI> for LaneStrokeCanvas {
    fn receive(&mut self, _msg: &AddToUI) -> Fate {
        UserInterface::id() << Add::Interactable3d(LaneStrokeCanvas::id(), AnyShape::Everywhere, 0);
        UserInterface::id() << Focus(LaneStrokeCanvas::id());
        Fate::Live
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(LaneStrokeCanvas{cmd_pressed: false, shift_pressed: false});
    system.add_inbox::<Event3d, LaneStrokeCanvas>();
    system.add_inbox::<AddToUI, LaneStrokeCanvas>();
    LaneStrokeCanvas::id() << AddToUI;    
}