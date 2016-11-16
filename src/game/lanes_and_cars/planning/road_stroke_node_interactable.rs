use kay::{Actor, Swarm, ID, Recipient, RecipientAsSwarm, CreateWith, ActorSystem, Individual, Fate};
use descartes::{Circle, P2, Into2d};
use core::geometry::AnyShape;
use core::ui::UserInterface;
use monet::{Vertex, Thing, Instance};
use super::{CurrentPlan, RoadStrokeRef, RoadStrokeNodeRef, InteractableParent};

#[derive(Compact, Actor, Clone)]
pub struct RoadStrokeNodeInteractable {
    _id: ID,
    original_position: P2,
    position: P2,
    node_ref: RoadStrokeNodeRef,
    parent: InteractableParent,
    hovered: bool
}

impl RoadStrokeNodeInteractable {
    pub fn new(original_position: P2, node_ref: RoadStrokeNodeRef, parent: InteractableParent) -> Self {
        RoadStrokeNodeInteractable{
            _id: ID::invalid(),
            original_position: original_position,
            position: original_position,
            node_ref: node_ref,
            parent: parent,
            hovered: false
        }
    }
}

use super::AddToUI;
use core::ui::Add;

impl Recipient<AddToUI> for RoadStrokeNodeInteractable {
    fn receive(&mut self, msg: &AddToUI) -> Fate {match *msg{
        AddToUI => {
            UserInterface::id() << Add::Interactable3d(self.id(), AnyShape::Circle(Circle{
                center: self.position,
                radius: 10.0
            }), 1);
            Fate::Live
        }
    }}
}

use super::ClearAll;
use core::ui::Remove;

impl Recipient<ClearAll> for RoadStrokeNodeInteractable {
    fn receive(&mut self, msg: &ClearAll) -> Fate {match *msg{
        ClearAll => {
            ::core::ui::UserInterface::id() << Remove::Interactable3d(self.id());
            Fate::Die
        }
    }}
}

use core::ui::Event3d;
use super::PlanControl::{ModifyRemainingOld, MoveRoadStrokeNodeTo};
use super::RecreateInteractables;

impl Recipient<Event3d> for RoadStrokeNodeInteractable {
    fn receive(&mut self, msg: &Event3d) -> Fate {match *msg{
        Event3d::DragStarted{..} => {
            if let InteractableParent::RemainingOldStroke{new_ref_to_become} = self.parent {
                CurrentPlan::id() << ModifyRemainingOld(RoadStrokeRef(self.node_ref.0));
                self.node_ref = RoadStrokeNodeRef(new_ref_to_become.0, self.node_ref.1);
                self.parent = InteractableParent::New;
            };
            Fate::Live
        },
        Event3d::DragOngoing{from, to} => {
            if let InteractableParent::New = self.parent {
                self.position = self.original_position + (to.into_2d() - from.into_2d());
                CurrentPlan::id() << MoveRoadStrokeNodeTo(self.node_ref, self.position);
            };
            Fate::Live
        },
        Event3d::DragFinished{..} => {
            CurrentPlan::id() << RecreateInteractables;
            Fate::Live
        },
        Event3d::HoverStarted{..} => {
            self.hovered = true;
            Fate::Live
        },
        Event3d::HoverStopped => {
            self.hovered = false;
            Fate::Live
        }
        _ => Fate::Live
    }}
}

use monet::SetupInScene;
use monet::AddBatch;

impl RecipientAsSwarm<SetupInScene> for RoadStrokeNodeInteractable {
    fn receive(_swarm: &mut Swarm<Self>, msg: &SetupInScene) -> Fate {match *msg{
        SetupInScene{renderer_id, scene_id} => {
            renderer_id << AddBatch{scene_id: scene_id, batch_id: 4982939, thing: Thing::new(
                vec![
                    Vertex{position: [-1.0, -1.0, 0.0]},
                    Vertex{position: [1.0, -1.0, 0.0]},
                    Vertex{position: [1.0, 1.0, 0.0]},
                    Vertex{position: [-1.0, 1.0, 0.0]}
                ],
                vec![
                    0, 1, 2,
                    2, 3, 0
                ]
            )};
            Fate::Live
        }
    }}
}

use monet::RenderToScene;
use monet::AddInstance;

impl Recipient<RenderToScene> for RoadStrokeNodeInteractable {
    fn receive(&mut self, msg: &RenderToScene) -> Fate {match *msg {
        RenderToScene{renderer_id, scene_id} => {
            renderer_id << AddInstance{scene_id: scene_id, batch_id: 4982939, position: Instance{
                instance_position: [self.position.x, self.position.y, 0.0],
                instance_direction: [1.0, 0.0],
                instance_color: if self.hovered {[1.0, 0.0, 0.0]} else if self.parent == InteractableParent::New {[0.0, 0.0, 1.0]} else {[0.3, 0.3, 0.3]}
            }};
            Fate::Live
        }
    }}
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(Swarm::<RoadStrokeNodeInteractable>::new());
    system.add_inbox::<ClearAll, Swarm<RoadStrokeNodeInteractable>>();
    system.add_inbox::<Event3d, Swarm<RoadStrokeNodeInteractable>>();
    system.add_inbox::<SetupInScene, Swarm<RoadStrokeNodeInteractable>>();
    system.add_inbox::<RenderToScene, Swarm<RoadStrokeNodeInteractable>>();
    system.add_inbox::<CreateWith<RoadStrokeNodeInteractable, AddToUI>, Swarm<RoadStrokeNodeInteractable>>();
}