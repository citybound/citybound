use descartes::{P2, V2, Path, Segment, Band, Into2d, Circle};
use kay::{CVec, Actor, Swarm, ID, Recipient, RecipientAsSwarm, CreateWith, ActorSystem, Individual, Fate};
use monet::{Instance, Thing, Vertex};
use core::geometry::{CPath, band_to_thing, AnyShape};
use core::ui::UserInterface;

type PlanRef = CVec<usize>;

#[derive(Compact, Clone)]
pub struct Plan {
    strokes: CVec<RoadStroke>
}
impl Individual for Plan{}

impl Plan{
    fn create_interactables(&self) {
        for (i, stroke) in self.strokes.iter().enumerate() {
            stroke.create_interactables(vec![i].into());
        }
    }
}

#[derive(Compact, Clone)]
struct MoveRoadStrokeNodeTo{node_ref: PlanRef, position: P2}

impl Recipient<MoveRoadStrokeNodeTo> for Plan {
    fn receive(&mut self, msg: &MoveRoadStrokeNodeTo) -> Fate {match *msg{
        MoveRoadStrokeNodeTo{ref node_ref, position} => {
            self.strokes[node_ref[0]].nodes[node_ref[1]].position = position;
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
struct RecreateInteractables;

impl Recipient<RecreateInteractables> for Plan {
    fn receive(&mut self, _msg: &RecreateInteractables) -> Fate {
        Swarm::<RoadStrokeNodeInteractable>::all() << ClearAll;
        self.create_interactables();
        Fate::Live
    }
}

use monet::SetupInScene;

impl Recipient<SetupInScene> for Plan {
    fn receive(&mut self, _msg: &SetupInScene) -> Fate {
        self.create_interactables();
        Fate::Live
    }
}

use monet::RenderToScene;
use monet::UpdateThing;

impl Recipient<RenderToScene> for Plan {
    fn receive(&mut self, msg: &RenderToScene) -> Fate {match *msg{
        RenderToScene{renderer_id, scene_id} => {
            renderer_id << UpdateThing{
                scene_id: scene_id,
                thing_id: 13,
                thing: self.strokes[0].preview_thing(),
                instance: Instance{
                    instance_position: [0.0, 0.0, 0.0],
                    instance_direction: [1.0, 0.0],
                    instance_color: [0.5, 0.5, 0.5]
                }
            };
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
struct RoadStroke{
    nodes: CVec<RoadStrokeNode>
}

impl RoadStroke {
    fn path(&self) -> CPath {
        CPath::new(vec![
            Segment::line(
                self.nodes[0].position,
                self.nodes[1].position
            )
        ])
    }

    fn preview_thing(&self) -> Thing {
        band_to_thing(&Band::new(self.path(), 3.0), 0.0)
    }
}

impl RoadStroke {
    fn create_interactables(&self, self_ref: PlanRef) {
        for (i, node) in self.nodes.iter().enumerate() {
            let mut child_ref = self_ref.clone();
            child_ref.push(i);
            node.create_interactables(child_ref);
        }
    } 
}

#[derive(Copy, Clone)]
struct RoadStrokeNode {
    position: P2,
    direction: Option<V2>
}
#[derive(Copy, Clone)]
struct AddToUI;

impl RoadStrokeNode {
    fn create_interactables(&self, self_ref: PlanRef) {
        Swarm::<RoadStrokeNodeInteractable>::all() << CreateWith(RoadStrokeNodeInteractable{
            _id: ID::invalid(),
            original_position: self.position,
            position: self.position,
            node_ref: self_ref
        }, AddToUI);
    }
}

#[derive(Compact, Actor, Clone)]
pub struct RoadStrokeNodeInteractable {
    _id: ID,
    original_position: P2,
    position: P2,
    node_ref: PlanRef
}

use core::ui::Add;

impl Recipient<AddToUI> for RoadStrokeNodeInteractable {
    fn receive(&mut self, msg: &AddToUI) -> Fate {match *msg{
        AddToUI => {
            UserInterface::id() << Add::Interactable3d(self.id(), AnyShape::Circle(Circle{
                center: self.position,
                radius: 10.0
            }));
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
struct ClearAll;
use core::ui::Remove;

impl Recipient<ClearAll> for RoadStrokeNodeInteractable {
    fn receive(&mut self, msg: &ClearAll) -> Fate {match *msg{
        ClearAll => {
            ::core::ui::UserInterface::id() << Remove::Interactable3d(self.id());
            Fate::Die
        }
    }}
}

use core::ui::Dragging3d;

impl Recipient<Dragging3d> for RoadStrokeNodeInteractable {
    fn receive(&mut self, msg: &Dragging3d) -> Fate {match *msg{
        Dragging3d::Ongoing{from, to} => {
            self.position = self.original_position + (to.into_2d() - from.into_2d());
            Plan::id() << MoveRoadStrokeNodeTo{
                node_ref: self.node_ref.clone(),
                position: self.position
            };
            Fate::Live
        },
        Dragging3d::Finished => {
            Plan::id() << RecreateInteractables;
            Fate::Live
        }
        _ => Fate::Live
    }}
}

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

use monet::AddInstance;

impl Recipient<RenderToScene> for RoadStrokeNodeInteractable {
    fn receive(&mut self, msg: &RenderToScene) -> Fate {match *msg {
        RenderToScene{renderer_id, scene_id} => {
            renderer_id << AddInstance{scene_id: scene_id, batch_id: 4982939, position: Instance{
                instance_position: [self.position.x, self.position.y, 0.0],
                instance_direction: [1.0, 0.0],
                instance_color: [1.0, 0.0, 0.0]
            }};
            Fate::Live
        }
    }}
}

pub fn setup(system: &mut ActorSystem) {
    let plan = Plan{
        strokes: vec![RoadStroke{
            nodes: vec![
                RoadStrokeNode{
                    position: P2::new(0.0, 0.0),
                    direction: None
                },
                RoadStrokeNode{
                    position: P2::new(100.0, 0.0),
                    direction: None
                }
            ].into()
        }].into()
    };

    system.add_individual(plan);
    system.add_inbox::<SetupInScene, Plan>();
    system.add_inbox::<RenderToScene, Plan>();
    system.add_inbox::<MoveRoadStrokeNodeTo, Plan>();
    system.add_inbox::<RecreateInteractables, Plan>();
    system.add_individual(Swarm::<RoadStrokeNodeInteractable>::new());
    system.add_inbox::<ClearAll, Swarm<RoadStrokeNodeInteractable>>();
    system.add_inbox::<Dragging3d, Swarm<RoadStrokeNodeInteractable>>();
    system.add_inbox::<SetupInScene, Swarm<RoadStrokeNodeInteractable>>();
    system.add_inbox::<RenderToScene, Swarm<RoadStrokeNodeInteractable>>();
    system.add_inbox::<CreateWith<RoadStrokeNodeInteractable, AddToUI>, Swarm<RoadStrokeNodeInteractable>>();
}