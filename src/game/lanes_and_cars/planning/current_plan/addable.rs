use kay::{ID, Recipient, Actor, Fate};
use kay::swarm::{Swarm, SubActor, CreateWith};
use descartes::{Band, P2};
use core::geometry::AnyShape;

use super::CurrentPlan;
use super::super::lane_stroke::LaneStroke;

#[derive(SubActor, Compact, Clone)]
pub struct Addable {
    _id: Option<ID>,
    stroke: LaneStroke,
}

impl Addable {
    pub fn new(stroke: LaneStroke) -> Self {
        Addable {
            _id: None,
            stroke: stroke,
        }
    }
}

use super::AddToUI;
use core::user_interface::Add;

impl Recipient<AddToUI> for Addable {
    fn receive(&mut self, msg: &AddToUI) -> Fate {
        match *msg {
            AddToUI => {
                ::core::user_interface::UserInterface::id() <<
                Add::Interactable3d(self.id(),
                                    AnyShape::Band(Band::new(self.stroke.path().clone(), 5.0)),
                                    3);
                Fate::Live
            }
        }
    }
}

use super::ClearDraggables;
use core::user_interface::Remove;

impl Recipient<ClearDraggables> for Addable {
    fn receive(&mut self, msg: &ClearDraggables) -> Fate {
        match *msg {
            ClearDraggables => {
                ::core::user_interface::UserInterface::id() << Remove::Interactable3d(self.id());
                Fate::Die
            }
        }
    }
}

use core::user_interface::Event3d;
use super::{AddStroke, Commit};

impl Recipient<Event3d> for Addable {
    fn receive(&mut self, msg: &Event3d) -> Fate {
        match *msg {
            Event3d::HoverStarted { .. } |
            Event3d::HoverOngoing { .. } => {
                CurrentPlan::id() << AddStroke { stroke: self.stroke.clone() };
                Fate::Live
            }
            Event3d::DragFinished { .. } => {
                CurrentPlan::id() << Commit(true, P2::new(0.0, 0.0));
                Fate::Live
            }
            _ => Fate::Live,
        }
    }
}


pub fn setup() {
    Swarm::<Addable>::register_default();
    Swarm::<Addable>::handle::<CreateWith<Addable, AddToUI>>();
    Swarm::<Addable>::handle::<ClearDraggables>();
    Swarm::<Addable>::handle::<Event3d>();
}
