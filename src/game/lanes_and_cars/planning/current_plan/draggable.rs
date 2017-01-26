use kay::{ID, Recipient, Actor, Fate};
use kay::swarm::{Swarm, SubActor, CreateWith};
use descartes::{N, Band, Into2d, Norm};
use ::core::geometry::{CPath, AnyShape};

use super::{SelectableStrokeRef, CurrentPlan};

#[derive(SubActor, Compact, Clone)]
pub struct Draggable {
    _id: Option<ID>,
    stroke_ref: SelectableStrokeRef,
    path: CPath,
}

impl Draggable {
    pub fn new(stroke_ref: SelectableStrokeRef, path: CPath) -> Self {
        Draggable {
            _id: None,
            stroke_ref: stroke_ref,
            path: path,
        }
    }
}

use super::InitInteractable;
use core::ui::Add;

impl Recipient<InitInteractable> for Draggable {
    fn receive(&mut self, _msg: &InitInteractable) -> Fate {
        println!("draggable created");
        ::core::ui::UserInterface::id() <<
        Add::Interactable3d(self.id(),
                            AnyShape::Band(Band::new(self.path.clone(), 5.0)),
                            3);
        Fate::Live
    }
}

use super::ClearInteractable;
use core::ui::Remove;

impl Recipient<ClearInteractable> for Draggable {
    fn receive(&mut self, _msg: &ClearInteractable) -> Fate {
        ::core::ui::UserInterface::id() << Remove::Interactable3d(self.id());
        Fate::Die
    }
}

use core::ui::Event3d;
use super::{ChangeIntent, Intent, IntentProgress};

const MAXIMIZE_DISTANCE: N = 0.5;

impl Recipient<Event3d> for Draggable {
    fn receive(&mut self, msg: &Event3d) -> Fate {
        match *msg {
            Event3d::DragOngoing { from, to } => {
                CurrentPlan::id() <<
                ChangeIntent(Intent::MoveSelection(to.into_2d() - from.into_2d()),
                             IntentProgress::Preview);
                Fate::Live
            }
            Event3d::DragFinished { from, to } => {
                let delta = to.into_2d() - from.into_2d();
                if delta.norm() < MAXIMIZE_DISTANCE {
                    CurrentPlan::id() <<
                    ChangeIntent(Intent::MaximizeSelection, IntentProgress::Finished);
                } else {
                    CurrentPlan::id() <<
                    ChangeIntent(Intent::MoveSelection(delta), IntentProgress::Finished);
                }
                Fate::Live
            }
            _ => Fate::Live,
        }
    }
}

pub fn setup() {
    Swarm::<Draggable>::register_default();
    Swarm::<Draggable>::handle::<CreateWith<Draggable, InitInteractable>>();
    Swarm::<Draggable>::handle::<ClearInteractable>();
    Swarm::<Draggable>::handle::<Event3d>();
}