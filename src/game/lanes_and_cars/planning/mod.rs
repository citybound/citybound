use descartes::{P2, V2, Path, Segment, Band, Intersect, convex_hull};
use kay::{CVec, Swarm, Recipient, CreateWith, ActorSystem, Individual, Fate};
use monet::{Instance, Thing, Norm};
use core::geometry::{CPath, band_to_thing};

mod road_stroke_node_interactable;
mod road_stroke_canvas;

pub use self::road_stroke_node_interactable::RoadStrokeNodeInteractable;
pub use self::road_stroke_canvas::RoadStrokeCanvas;

#[derive(Copy, Clone)]
pub struct PlanRef(usize, usize);

#[derive(Compact, Clone)]
pub struct Plan {
    strokes: CVec<RoadStroke>,
    strokes_after_cutting: CVec<RoadStroke>,
    intersections: CVec<Intersection>,
    ui_state: PlanUIState
}
impl Individual for Plan{}

impl Plan{
    fn create_interactables(&self) {
        Swarm::<RoadStrokeCanvas>::all() << CreateWith(RoadStrokeCanvas::new(), AddToUI);
        for (i, stroke) in self.strokes.iter().enumerate() {
            stroke.create_interactables(PlanRef(i, 0));
        }
    }

    fn find_intersections(&self) -> CVec<Intersection> {
        let mut intersections = Vec::new();
        
        for i in 0..self.strokes.len() {
            let stroke1 = &self.strokes[i];
            if stroke1.nodes.len() > 1 {
                let band1 = Band::new(stroke1.path(), 3.0).outline();
                for j in (i + 1)..self.strokes.len() {
                    let stroke2 = &self.strokes[j];
                    if stroke2.nodes.len() > 2 {
                        let band2 = Band::new(stroke2.path(), 3.0).outline();

                        let intersection_points = (&band1, &band2).intersect();

                        if intersection_points.len() >= 2 {
                            intersections.push(Intersection{
                                shape: convex_hull(&*intersection_points.iter().map(|i| i.position).collect::<Vec<_>>()),
                                connecting_strokes: CVec::new()
                            });
                        } 
                    }
                }
            }
        }

        intersections.into()
    }
}

#[derive(Copy, Clone)]
enum PlanControl{
    AddRoadStrokeNode(P2),
    MoveRoadStrokeNodeTo(PlanRef, P2)
}

impl Recipient<PlanControl> for Plan {
    fn receive(&mut self, msg: &PlanControl) -> Fate {match *msg{
        PlanControl::AddRoadStrokeNode(at) => {
            let new_node = RoadStrokeNode{position: at, direction: None};
            
            if let Some(PlanRef(stroke_idx, node_idx)) = self.ui_state.current_node {
                let stroke = &mut self.strokes[stroke_idx];
                let current_node = stroke.nodes[node_idx];

                if (current_node.position - new_node.position).norm() < 5.0 {
                    // finish stroke
                    self.ui_state.current_node = None;
                } else if node_idx == stroke.nodes.len() - 1 {
                    // append
                    stroke.nodes.push(new_node);
                    self.ui_state.current_node = Some(PlanRef(stroke_idx, stroke.nodes.len() - 1));
                } else if node_idx == 0 {
                    // prepend
                    stroke.nodes.insert(0, new_node);
                } else {unreachable!()}
            } else {
                self.strokes.push(RoadStroke{
                    nodes: vec![new_node].into()
                });
                self.ui_state.current_node = Some(PlanRef(self.strokes.len() - 1, 0))
            }

            self.ui_state.dirty = true;
            self.intersections = self.find_intersections();
            Fate::Live
        },
        PlanControl::MoveRoadStrokeNodeTo(PlanRef(stroke, node), position) => {
            self.strokes[stroke].nodes[node].position = position;
            self.ui_state.dirty = true;
            self.intersections = self.find_intersections();
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
struct RecreateInteractables;
#[derive(Copy, Clone)]
struct ClearAll;

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
            if self.ui_state.dirty {
                let thing : Thing = self.strokes.iter()
                    .filter(|stroke| stroke.nodes.len() > 1)
                    .map(RoadStroke::preview_thing)
                    .sum();
                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 13,
                    thing: thing,
                    instance: Instance{
                        instance_position: [0.0, 0.0, 0.0],
                        instance_direction: [1.0, 0.0],
                        instance_color: [0.5, 0.5, 0.5]
                    }
                };
                let intersections_thing : Thing = self.intersections.iter()
                    .filter(|i| i.shape.segments().len() > 0)
                    .map(|i| band_to_thing(&Band::new(i.shape.clone(), 0.2), 0.0))
                    .sum();
                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 14,
                    thing: intersections_thing,
                    instance: Instance{
                        instance_position: [0.0, 0.0, 0.0],
                        instance_direction: [1.0, 0.0],
                        instance_color: [0.0, 0.0, 1.0]
                    }
                };
                self.ui_state.dirty = false;
            }
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
        CPath::new(self.nodes.windows(2).map(|window|
            Segment::line(window[0].position, window[1].position)
        ).collect::<Vec<_>>())
    }

    fn preview_thing(&self) -> Thing {
        band_to_thing(&Band::new(Band::new(self.path(), 3.0).outline(), 0.2), 0.0)
    }

    fn create_interactables(&self, self_ref: PlanRef) {
        for (i, node) in self.nodes.iter().enumerate() {
            let child_ref = match self_ref {
                PlanRef(stroke_idx, _) => PlanRef(stroke_idx, i)
            };
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
        Swarm::<RoadStrokeNodeInteractable>::all() << CreateWith(
            RoadStrokeNodeInteractable::new(self.position, self_ref),
            AddToUI
        );
    }
}

#[derive(Compact, Clone)]
struct Intersection{
    shape: CPath,
    connecting_strokes: CVec<RoadStroke>
}

#[derive(Compact, Clone)]
struct PlanUIState{
    current_node: Option<PlanRef>,
    dirty: bool
}

pub fn setup(system: &mut ActorSystem) {
    let plan = Plan{
        strokes: vec![
            RoadStroke{
                nodes: vec![
                    RoadStrokeNode{position: P2::new(0.0, 0.0), direction: None},
                    RoadStrokeNode{position: P2::new(100.0, 0.0), direction: None},
                    RoadStrokeNode{position: P2::new(150.0, 50.0), direction: None}
                ].into()
            },
            RoadStroke{
                nodes: vec![
                    RoadStrokeNode{position: P2::new(0.0, 100.0), direction: None},
                    RoadStrokeNode{position: P2::new(100.0, 100.0), direction: None},
                    RoadStrokeNode{position: P2::new(150.0, 150.0), direction: None}
                ].into()
            },
        ].into(),
        strokes_after_cutting: CVec::new(),
        intersections: CVec::new(),
        ui_state: PlanUIState{
            current_node: None,
            dirty: true
        }
    };

    system.add_individual(plan);
    system.add_inbox::<SetupInScene, Plan>();
    system.add_inbox::<RenderToScene, Plan>();
    system.add_inbox::<PlanControl, Plan>();
    system.add_inbox::<RecreateInteractables, Plan>();

    road_stroke_canvas::setup(system);
    road_stroke_node_interactable::setup(system);
}