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
use core::user_interface::Add;

impl Recipient<InitInteractable> for Draggable {
    fn receive(&mut self, _msg: &InitInteractable) -> Fate {
        ::core::user_interface::UserInterface::id() <<
        Add::Interactable3d(self.id(),
                            AnyShape::Band(Band::new(self.path.clone(), 5.0)),
                            4);
        Fate::Live
    }
}

use super::ClearInteractable;
use core::user_interface::Remove;

impl Recipient<ClearInteractable> for Draggable {
    fn receive(&mut self, _msg: &ClearInteractable) -> Fate {
        ::core::user_interface::UserInterface::id() << Remove::Interactable3d(self.id());
        Fate::Die
    }
}

const MAXIMIZE_DISTANCE: N = 0.5;

use core::settings::Action;
use core::user_interface::Event3d;
use super::{ChangeIntent, Intent, IntentProgress};

impl Recipient<Action> for Draggable {
    fn receive(&mut self, msg: &Action) -> Fate {
        match *msg {
            Action::Event3d(event_3d) => {
                match event_3d {
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
            _ => Fate::Live,
        }
    }
}

pub fn setup() {
    Swarm::<Draggable>::register_default();
    Swarm::<Draggable>::handle::<CreateWith<Draggable, InitInteractable>>();
    Swarm::<Draggable>::handle::<ClearInteractable>();
    Swarm::<Draggable>::handle::<Action>();
}