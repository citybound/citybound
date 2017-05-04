use kay::{ID, Recipient, Actor, Fate};
use kay::swarm::{Swarm, SubActor, CreateWith};
use descartes::{N, Band, Into2d, Norm};
use stagemaster::geometry::{CPath, AnyShape};

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
use stagemaster::{UserInterface, AddInteractable};

impl Recipient<InitInteractable> for Draggable {
    fn receive(&mut self, _msg: &InitInteractable) -> Fate {
        UserInterface::id() <<
        AddInteractable(self.id(),
                        AnyShape::Band(Band::new(self.path.clone(), 5.0)),
                        4);
        Fate::Live
    }
}

use super::ClearInteractable;
use stagemaster::RemoveInteractable;

impl Recipient<ClearInteractable> for Draggable {
    fn receive(&mut self, _msg: &ClearInteractable) -> Fate {
        UserInterface::id() << RemoveInteractable(self.id());
        Fate::Die
    }
}

use stagemaster::Event3d;
use super::{ChangeIntent, Intent, IntentProgress};

const MAXIMIZE_DISTANCE: N = 0.5;

impl Recipient<Event3d> for Draggable {
    fn receive(&mut self, msg: &Event3d) -> Fate {
        match *msg {
            Event3d::DragOngoing { from, to, .. } => {
                CurrentPlan::id() <<
                ChangeIntent(Intent::MoveSelection(to.into_2d() - from.into_2d()),
                             IntentProgress::Preview);
                Fate::Live
            }
            Event3d::DragFinished { from, to, .. } => {
                let delta = to.into_2d() - from.into_2d();
                if delta.norm() < MAXIMIZE_DISTANCE {
                    CurrentPlan::id() <<
                    ChangeIntent(Intent::MaximizeSelection, IntentProgress::Immediate);
                } else {
                    CurrentPlan::id() <<
                    ChangeIntent(Intent::MoveSelection(delta), IntentProgress::Immediate);
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
