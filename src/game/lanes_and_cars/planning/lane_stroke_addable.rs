use kay::{ID, Recipient, SubActor, Actor, Swarm, ActorSystem, Fate, CreateWith};
use descartes::{Band, P2};
use ::core::geometry::AnyShape;

use super::{LaneStroke, CurrentPlan};

#[derive(SubActor, Compact, Clone)]
pub struct LaneStrokeAddable {
    _id: Option<ID>,
    stroke: LaneStroke,
}

impl LaneStrokeAddable {
    pub fn new(stroke: LaneStroke) -> Self {
        LaneStrokeAddable {
            _id: None,
            stroke: stroke,
        }
    }
}

use super::AddToUI;
use ::core::ui::Add;

impl Recipient<AddToUI> for LaneStrokeAddable {
    fn receive(&mut self, msg: &AddToUI) -> Fate {
        match *msg {
            AddToUI => {
                ::core::ui::UserInterface::id() <<
                Add::Interactable3d(self.id(),
                                    AnyShape::Band(Band::new(self.stroke.path().clone(), 5.0)),
                                    3);
                Fate::Live
            }
        }
    }
}

use super::ClearDraggables;
use ::core::ui::Remove;

impl Recipient<ClearDraggables> for LaneStrokeAddable {
    fn receive(&mut self, msg: &ClearDraggables) -> Fate {
        match *msg {
            ClearDraggables => {
                ::core::ui::UserInterface::id() << Remove::Interactable3d(self.id());
                Fate::Die
            }
        }
    }
}

use ::core::ui::Event3d;
use super::{AddStroke, Commit};

impl Recipient<Event3d> for LaneStrokeAddable {
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


pub fn setup(system: &mut ActorSystem) {
    system.add_actor(Swarm::<LaneStrokeAddable>::new());
    Swarm::<LaneStrokeAddable>::handle::<CreateWith<LaneStrokeAddable, AddToUI>>();
    Swarm::<LaneStrokeAddable>::handle::<ClearDraggables>();
    Swarm::<LaneStrokeAddable>::handle::<Event3d>();
}
