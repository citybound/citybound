use kay::{Actor, Swarm, ToRandom, ID, Recipient, CreateWith, ActorSystem, Individual, Fate};
use descartes::{Into2d, P2, P3};
use core::geometry::AnyShape;
use core::ui::{UserInterface, VirtualKeyCode};
use super::{CurrentPlan};

#[derive(Copy, Clone, Actor, Default)]
pub struct RoadStrokeCanvas {_id: ID, last_click: Option<P2>}

impl RoadStrokeCanvas{
    pub fn new() -> Self {Self::default()}
}

use super::AddToUI;
use core::ui::Add;
use core::ui::Focus;

impl Recipient<AddToUI> for RoadStrokeCanvas {
    fn receive(&mut self, _msg: &AddToUI) -> Fate {
        UserInterface::id() << Add::Interactable3d(self.id(), AnyShape::Everywhere, 0);
        UserInterface::id() << Focus(self.id());
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
use super::PlanControl::{AddRoadStrokeNode, Materialize, CreateGrid};
use super::RecreateInteractables;

impl Recipient<Event3d> for RoadStrokeCanvas {
    fn receive(&mut self, msg: &Event3d) -> Fate {match *msg {
        Event3d::DragStarted{at} => {
            CurrentPlan::id() << AddRoadStrokeNode(at.into_2d(), true);
            Fate::Live
        },
        Event3d::DragFinished{..} => {
            CurrentPlan::id() << RecreateInteractables;
            Fate::Live
        },
        Event3d::KeyDown(VirtualKeyCode::Return) => {
            CurrentPlan::id() << Materialize(());
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
        }
        _ => Fate::Live
    }}
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(Swarm::<RoadStrokeCanvas>::new());
    system.add_inbox::<ClearAll, Swarm<RoadStrokeCanvas>>();
    system.add_inbox::<Event3d, Swarm<RoadStrokeCanvas>>();
    system.add_inbox::<CreateWith<RoadStrokeCanvas, AddToUI>, Swarm<RoadStrokeCanvas>>();
}