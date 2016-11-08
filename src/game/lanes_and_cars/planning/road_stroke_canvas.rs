use kay::{Actor, Swarm, ID, Recipient, CreateWith, ActorSystem, Individual, Fate};
use descartes::{Into2d};
use core::geometry::AnyShape;
use core::ui::UserInterface;
use super::{Plan};

#[derive(Copy, Clone, Actor, Default)]
pub struct RoadStrokeCanvas {_id: ID}

impl RoadStrokeCanvas{
    pub fn new() -> Self {Self::default()}
}

use super::AddToUI;
use core::ui::Add;

impl Recipient<AddToUI> for RoadStrokeCanvas {
    fn receive(&mut self, _msg: &AddToUI) -> Fate {
        UserInterface::id() << Add::Interactable3d(self.id(), AnyShape::Everywhere, 0);
        Fate::Live
    }
}

use super::ClearAll;
use core::ui::Remove;

impl Recipient<ClearAll> for RoadStrokeCanvas {
    fn receive(&mut self, _msg: &ClearAll) -> Fate {
        UserInterface::id() << Remove::Interactable3d(self.id());
        Fate::Die
    }
}

use core::ui::Event3d;
use super::PlanControl::AddRoadStrokeNode;
use super::RecreateInteractables;

impl Recipient<Event3d> for RoadStrokeCanvas {
    fn receive(&mut self, msg: &Event3d) -> Fate {match *msg {
        Event3d::DragStarted{at} => {
            Plan::id() << AddRoadStrokeNode(at.into_2d());
            Fate::Live
        },
        Event3d::DragFinished{..} => {
            Plan::id() << RecreateInteractables;
            Fate::Live
        },
        _ => Fate::Live
    }}
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(Swarm::<RoadStrokeCanvas>::new());
    system.add_inbox::<ClearAll, Swarm<RoadStrokeCanvas>>();
    system.add_inbox::<Event3d, Swarm<RoadStrokeCanvas>>();
    system.add_inbox::<CreateWith<RoadStrokeCanvas, AddToUI>, Swarm<RoadStrokeCanvas>>();
}