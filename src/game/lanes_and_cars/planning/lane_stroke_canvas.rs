use kay::{Swarm, ToRandom, Recipient, ActorSystem, Individual, Fate};
use descartes::{Into2d, P3};
use core::geometry::AnyShape;
use core::ui::{UserInterface, VirtualKeyCode, KeyOrButton, intersection};
use core::settings::Settings;
use super::{CurrentPlan};

#[derive(Clone, Default)]
pub struct LaneStrokeCanvas{
    keys_down: Vec<KeyOrButton>,
    settings: Settings
}


impl LaneStrokeCanvas {
    fn new() -> LaneStrokeCanvas {
        LaneStrokeCanvas {
            keys_down: Vec::new(),
            settings: Settings::load(),
        }
    }
}

impl Individual for LaneStrokeCanvas {}

use core::ui::Event3d;
use super::{Commit, Undo, Redo, WithLatestNode, Materialize, CreateGrid, DeleteSelection,
            SetSelectionMode, SetNLanes, ToggleBothSides};

impl Recipient<Event3d> for LaneStrokeCanvas {
    fn receive(&mut self, msg: &Event3d) -> Fate {
        match *msg {
            Event3d::HoverStarted{at} | Event3d::HoverOngoing{at} => {
                CurrentPlan::id() << WithLatestNode(at.into_2d(), true);
            },
            Event3d::DragStarted{at} => {
                CurrentPlan::id() << WithLatestNode(at.into_2d(), true);
                CurrentPlan::id() << Commit(true, at.into_2d());
            },
            Event3d::DragFinished{..} => {
            },
            _ => ()
        }
        Fate::Live
    }

}

use core::ui::UIUpdate;

impl Recipient<UIUpdate> for LaneStrokeCanvas {
    fn receive(&mut self, _msg: &UIUpdate) -> Fate {
        if intersection(&self.keys_down, &self.settings.undo_key) && intersection(&self.keys_down, &self.settings.undo_modifier_key) {
            if intersection(&self.keys_down, &self.settings.redo_modifier_key) {
                CurrentPlan::id() << Redo;
            } else {
                CurrentPlan::id() << Undo;
            }
        }
        if intersection(&self.keys_down, &self.settings.car_spawning_key) {
            Swarm::<::game::lanes_and_cars::Lane>::all() << ToRandom{
            n_recipients: 5000,
            message: Event3d::DragFinished{from: P3::new(0.0, 0.0, 0.0), to: P3::new(0.0, 0.0, 0.0)}
            };
        }
        if intersection(&self.keys_down, &self.settings.finalize_key) {
            CurrentPlan::id() << Materialize;
        }
        if intersection(&self.keys_down, &self.settings.grid_key) {
            CurrentPlan::id() << CreateGrid(if intersection(&self.keys_down, &self.settings.grid_modifier_key) {15} else {10});
        }
        if intersection(&self.keys_down, &self.settings.delete_selection_key) {
            CurrentPlan::id() << DeleteSelection;
        }
        if self.keys_down.contains(&KeyOrButton::Key(VirtualKeyCode::Key1)) {CurrentPlan::id() << SetNLanes(1)}
        if self.keys_down.contains(&KeyOrButton::Key(VirtualKeyCode::Key2)) {CurrentPlan::id() << SetNLanes(2)}
        if self.keys_down.contains(&KeyOrButton::Key(VirtualKeyCode::Key3)) {CurrentPlan::id() << SetNLanes(3)}
        if self.keys_down.contains(&KeyOrButton::Key(VirtualKeyCode::Key4)) {CurrentPlan::id() << SetNLanes(4)}
        if self.keys_down.contains(&KeyOrButton::Key(VirtualKeyCode::Key5)) {CurrentPlan::id() << SetNLanes(5)}
        if self.keys_down.contains(&KeyOrButton::Key(VirtualKeyCode::Key6)) {CurrentPlan::id() << SetNLanes(6)}
        if self.keys_down.contains(&KeyOrButton::Key(VirtualKeyCode::Key7)) {CurrentPlan::id() << SetNLanes(7)}
        if self.keys_down.contains(&KeyOrButton::Key(VirtualKeyCode::Key8)) {CurrentPlan::id() << SetNLanes(8)}
        if self.keys_down.contains(&KeyOrButton::Key(VirtualKeyCode::Key9)) {CurrentPlan::id() << SetNLanes(9)}
        if self.keys_down.contains(&KeyOrButton::Key(VirtualKeyCode::Key0)) {
            CurrentPlan::id() << ToggleBothSides;
        }
        Fate::Live
    }
}

use core::ui::KeysHeld;

impl Recipient<KeysHeld> for LaneStrokeCanvas {
    fn receive(&mut self, msg: &KeysHeld) -> Fate {
        self.keys_down.clear();
        for i in msg.keys.iter() {
            self.keys_down.push(*i);
        }
        Fate::Live
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
        ::monet::Renderer::id() << ::monet::AddEyeListener{scene_id: 0, listener: Self::id()};
        Fate::Live
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(LaneStrokeCanvas::new());
    system.add_inbox::<Event3d, LaneStrokeCanvas>();
    system.add_inbox::<EyeMoved, LaneStrokeCanvas>();
    system.add_inbox::<AddToUI, LaneStrokeCanvas>();
    system.add_inbox::<UIUpdate, LaneStrokeCanvas>();
    system.add_inbox::<KeysHeld, LaneStrokeCanvas>();
    LaneStrokeCanvas::id() << AddToUI;
}
