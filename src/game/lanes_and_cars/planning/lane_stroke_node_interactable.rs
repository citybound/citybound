use kay::{Actor, Swarm, ID, Recipient, RecipientAsSwarm, CreateWith, ActorSystem, Individual, Fate};
use descartes::{Circle, P2, V2, Into2d};
use core::geometry::AnyShape;
use core::ui::UserInterface;
use monet::{Vertex, Thing, Instance};
use kay::CVec;
use super::{CurrentPlan, LaneStrokeRef, LaneStrokeNodeRef, InteractableParent};

#[derive(Compact, Actor, Clone)]
pub struct LaneStrokeNodeInteractable {
    _id: ID,
    original_position: P2,
    pub position: P2,
    original_direction: V2,
    pub direction: V2,
    pub node_refs: CVec<LaneStrokeNodeRef>,
    parent: InteractableParent,
    hovered: bool
}

impl LaneStrokeNodeInteractable {
    pub fn new(original_position: P2, original_direction: V2, node_refs: Vec<LaneStrokeNodeRef>, parent: InteractableParent) -> Self {
        LaneStrokeNodeInteractable{
            _id: ID::invalid(),
            original_position: original_position,
            position: original_position,
            original_direction: original_direction,
            direction: original_direction,
            node_refs: node_refs.into(),
            parent: parent,
            hovered: false
        }
    }
}

use super::AddToUI;
use core::ui::Add;

impl Recipient<AddToUI> for LaneStrokeNodeInteractable {
    fn receive(&mut self, msg: &AddToUI) -> Fate {match *msg{
        AddToUI => {
            UserInterface::id() << Add::Interactable3d(self.id(), AnyShape::Circle(Circle{
                center: self.position,
                radius: 10.0
            }), if self.node_refs.len() > 1 {2} else {1});
            Fate::Live
        }
    }}
}

use super::ClearAll;
use core::ui::Remove;

impl Recipient<ClearAll> for LaneStrokeNodeInteractable {
    fn receive(&mut self, msg: &ClearAll) -> Fate {match *msg{
        ClearAll => {
            ::core::ui::UserInterface::id() << Remove::Interactable3d(self.id());
            Fate::Die
        }
    }}
}

use core::ui::Event3d;
use super::PlanControl::{ModifyRemainingOld, MoveLaneStrokeNodesTo, MaybeMakeCurrent};
use super::RecreateInteractables;

impl Recipient<Event3d> for LaneStrokeNodeInteractable {
    fn receive(&mut self, msg: &Event3d) -> Fate {match *msg{
        Event3d::DragStarted{..} => {
            let maybe_new_parent = if let InteractableParent::WillBecomeNew(ref new_refs_to_become) = self.parent {
                CurrentPlan::id() << ModifyRemainingOld(self.node_refs.iter().map(|old_ref| LaneStrokeRef(old_ref.0)).collect());
                self.node_refs = self.node_refs.iter().zip(new_refs_to_become).map(
                    |(node_ref, new_ref)| LaneStrokeNodeRef(new_ref.0, node_ref.1)
                ).collect();
                Some(InteractableParent::New(()))
            } else {None};
            if let Some(new_parent) = maybe_new_parent {
                self.parent = new_parent;
            }
            Fate::Live
        },
        Event3d::DragOngoing{from, to} => {
            if let InteractableParent::New(()) = self.parent {
                let old_position = self.position;
                self.position = self.original_position + (to.into_2d() - from.into_2d());
                CurrentPlan::id() << MoveLaneStrokeNodesTo(self.node_refs.clone(), old_position, self.position);
            };
            Fate::Live
        },
        Event3d::DragFinished{..} => {
            CurrentPlan::id() << MaybeMakeCurrent(self.node_refs.clone(), self.position);
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

impl RecipientAsSwarm<SetupInScene> for LaneStrokeNodeInteractable {
    fn receive(_swarm: &mut Swarm<Self>, msg: &SetupInScene) -> Fate {match *msg{
        SetupInScene{renderer_id, scene_id} => {
            renderer_id << AddBatch{scene_id: scene_id, batch_id: 2400, thing: Thing::new(
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

impl Recipient<RenderToScene> for LaneStrokeNodeInteractable {
    fn receive(&mut self, msg: &RenderToScene) -> Fate {match *msg {
        RenderToScene{renderer_id, scene_id} => {
            renderer_id << AddInstance{scene_id: scene_id, batch_id: 2400, instance: Instance{
                instance_position: [self.position.x, self.position.y, 0.0],
                instance_direction: [1.0, 0.0],
                instance_color: if self.hovered {[1.0, 0.0, 0.0]} else {match self.parent {
                    InteractableParent::New(()) => [0.0, 0.0, 1.0],
                    _ => [0.3, 0.3, 0.3]}}
            }};
            Fate::Live
        }
    }}
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(Swarm::<LaneStrokeNodeInteractable>::new());
    system.add_inbox::<ClearAll, Swarm<LaneStrokeNodeInteractable>>();
    system.add_inbox::<Event3d, Swarm<LaneStrokeNodeInteractable>>();
    system.add_inbox::<SetupInScene, Swarm<LaneStrokeNodeInteractable>>();
    system.add_inbox::<RenderToScene, Swarm<LaneStrokeNodeInteractable>>();
    system.add_inbox::<CreateWith<LaneStrokeNodeInteractable, AddToUI>, Swarm<LaneStrokeNodeInteractable>>();
}