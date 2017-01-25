use kay::{ID, Recipient, Actor, Fate};
use kay::swarm::{Swarm, SubActor, CreateWith};
use descartes::{Band, Into2d, RoughlyComparable};
use core::geometry::{CPath, AnyShape};

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

#[derive(Copy, Clone)]
pub struct Become(SelectableStrokeRef);

impl Recipient<Become> for Draggable {
    fn receive(&mut self, msg: &Become) -> Fate {
        match *msg {
            Become(stroke_ref) => {
                self.stroke_ref = stroke_ref;
                Fate::Live
            }
        }
    }
}

use super::AddToUI;
use core::user_interface::Add;

impl Recipient<AddToUI> for Draggable {
    fn receive(&mut self, msg: &AddToUI) -> Fate {
        match *msg {
            AddToUI => {
                ::core::user_interface::UserInterface::id() <<
                Add::Interactable3d(self.id(),
                                    AnyShape::Band(Band::new(self.path.clone(), 5.0)),
                                    2);
                Fate::Live
            }
        }
    }
}

use super::ClearDraggables;
use core::user_interface::Remove;

impl Recipient<ClearDraggables> for Draggable {
    fn receive(&mut self, msg: &ClearDraggables) -> Fate {
        match *msg {
            ClearDraggables => {
                ::core::user_interface::UserInterface::id() << Remove::Interactable3d(self.id());
                Fate::Die
            }
        }
    }
}

use core::settings::Action;
use core::user_interface::Event3d;
use super::{MoveSelection, MaximizeSelection, Commit};

impl Recipient<Action> for Draggable {
    fn receive(&mut self, msg: &Action) -> Fate {
        match *msg {
            Action::Event3d(event_3d) => {
                match event_3d {
                    Event3d::DragOngoing { from, to } => {
                        CurrentPlan::id() << MoveSelection(to.into_2d() - from.into_2d());
                    }
                    Event3d::DragFinished { from, to } => {
                        if from.into_2d().is_roughly_within(to.into_2d(), 3.0) {
                            CurrentPlan::id() << MaximizeSelection;
                        }
                        CurrentPlan::id() << Commit(true, to.into_2d());
                    }
                _ => (),
                }
            }
            _ => (),
        }
        Fate::Live
    }
}

pub fn setup() {
    Swarm::<Draggable>::register_default();
    Swarm::<Draggable>::handle::<CreateWith<Draggable, AddToUI>>();
    Swarm::<Draggable>::handle::<Become>();
    Swarm::<Draggable>::handle::<ClearDraggables>();
    Swarm::<Draggable>::handle::<Action>();
}
