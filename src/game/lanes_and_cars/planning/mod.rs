use descartes::{P2, V2, Path, Segment, Band, Intersect, convex_hull, FiniteCurve, N, RoughlyComparable};
use kay::{CVec, Swarm, Recipient, CreateWith, ActorSystem, Individual, Fate};
use monet::{Instance, Thing, Norm};
use core::geometry::{CPath, band_to_thing};
use ordered_float::OrderedFloat;

mod road_stroke_node_interactable;
mod road_stroke_canvas;

pub use self::road_stroke_node_interactable::RoadStrokeNodeInteractable;
pub use self::road_stroke_canvas::RoadStrokeCanvas;

#[derive(Copy, Clone)]
pub struct PlanRef(usize, usize);

#[derive(Compact, Clone)]
pub struct Plan {
    strokes: CVec<RoadStroke>,
    intersections: CVec<Intersection>,
    strokes_after_cutting: CVec<RoadStroke>,
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

    fn create_intersections(&mut self) {
        let mut all_intersection_points = Vec::new();
        
        for i in 0..self.strokes.len() {
            let stroke1 = &self.strokes[i];
            if stroke1.nodes.len() > 1 {
                let band1 = Band::new(stroke1.path(), 8.0).outline();
                for j in (i + 1)..self.strokes.len() {
                    let stroke2 = &self.strokes[j];
                    if stroke2.nodes.len() > 1 {
                        let band2 = Band::new(stroke2.path(), 8.0).outline();

                        let intersections = (&band1, &band2).intersect();
                        all_intersection_points.extend(intersections.iter().map(|i| i.position));
                    }
                }
            }
        }

        let mut intersection_point_groups = Vec::<Vec<P2>>::new();

        const INTERSECTION_GROUPING_RADIUS : f32 = 20.0;

        for point in all_intersection_points.drain(..) {
            let create_new_group = match intersection_point_groups.iter_mut().find(|group| {
                let max_distance = group.iter().map(|other_point| OrderedFloat((*other_point - point).norm())).max().unwrap();
                *max_distance < INTERSECTION_GROUPING_RADIUS
            }) {
                Some(exisiting_group) => {exisiting_group.push(point); false},
                None => true
            };

            if create_new_group {
                intersection_point_groups.push(vec![point]);
            }
        }

        let mut merging_ongoing = true;

        while merging_ongoing {
            merging_ongoing = false;
            #[allow(needless_range_loop)]
            for i in 0..intersection_point_groups.len() {
                for j in ((i + 1)..intersection_point_groups.len()).rev() {
                    let merge_groups = {
                        let group_i = &intersection_point_groups[i];
                        let group_j = &intersection_point_groups[j];
                        group_i.iter().any(|point_i|
                            group_j.iter().any(|point_j| (*point_i - *point_j).norm() < INTERSECTION_GROUPING_RADIUS)
                        )
                    };
                    if merge_groups {
                        let group_to_be_merged = intersection_point_groups[j].clone();
                        intersection_point_groups[i].extend_from_slice(&group_to_be_merged);
                        intersection_point_groups[j].clear();
                        merging_ongoing = true;
                    }
                }
            }
        }

        self.intersections = intersection_point_groups.iter().filter_map(|group|
            if group.len() >= 2 {
                Some(Intersection{
                    shape: convex_hull::<CPath>(group),
                    incoming: CVec::new(),
                    outgoing: CVec::new(),
                    connecting_strokes: CVec::new()
                })
            } else {None}
        ).collect()
    }

    fn cut_strokes_at_intersections(&mut self) {
        let mut strokes_todo : Vec<_> = self.strokes.iter().cloned().collect();

        let mut cutting_ongoing = true;
        let mut iters = 0;
        while cutting_ongoing {
            cutting_ongoing = false;
            let mut new_strokes = Vec::new();

            for stroke in &strokes_todo {
                let mut was_cut = false;

                for intersection in self.intersections.iter_mut() {
                    let intersection_points = (&stroke.path(), &intersection.shape).intersect();
                    if intersection_points.len() >= 2 {
                        let entry_distance = intersection_points.iter().map(|p| OrderedFloat(p.along_a)).min().unwrap();
                        let exit_distance = intersection_points.iter().map(|p| OrderedFloat(p.along_a)).max().unwrap();
                        let before_intersection = stroke.cut_before(*entry_distance - 1.0);
                        let after_intersection = stroke.cut_after(*exit_distance + 1.0);  

                        intersection.incoming.push(*before_intersection.nodes.last().unwrap());
                        intersection.outgoing.push(after_intersection.nodes[0]);
                        new_strokes.push(before_intersection);
                        new_strokes.push(after_intersection);

                        cutting_ongoing = true;
                        was_cut = true;
                        break;
                    }
                }

                if !was_cut {new_strokes.push(stroke.clone())};
            }

            strokes_todo = new_strokes;
            iters += 1;
            if iters > 30 {
                panic!("STuck!!!")
            }
        }

        self.strokes_after_cutting = strokes_todo.into();
    }

    fn create_connecting_strokes_on_intersections(&mut self) {
        for intersection in self.intersections.iter_mut() {
            let mut connecting_strokes = CVec::new();
            for incoming in intersection.incoming.iter() {
                for outgoing in intersection.outgoing.iter() {
                    connecting_strokes.push(RoadStroke{
                        nodes: vec![*incoming, *outgoing].into()
                    });
                }
            }
            intersection.connecting_strokes = connecting_strokes;
        }

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
            self.create_intersections();
            self.cut_strokes_at_intersections();
            self.create_connecting_strokes_on_intersections();
            Fate::Live
        },
        PlanControl::MoveRoadStrokeNodeTo(PlanRef(stroke, node), position) => {
            self.strokes[stroke].nodes[node].position = position;
            self.ui_state.dirty = true;
            self.create_intersections();
            self.cut_strokes_at_intersections();
            self.create_connecting_strokes_on_intersections();
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
                let thing : Thing = self.strokes_after_cutting.iter()
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
                        instance_color: [0.3, 0.3, 0.5]
                    }
                };
                let intersections_thing : Thing = self.intersections.iter()
                    .filter(|i| i.shape.segments().len() > 0)
                    .map(|i| band_to_thing(&Band::new(i.shape.clone(), 0.4), 0.5))
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
                let connecting_strokes_thing : Thing = self.intersections.iter()
                    .filter(|i| !i.connecting_strokes.is_empty())
                    .map(|i| i.connecting_strokes.iter().map(RoadStroke::preview_thing).sum())
                    .sum();
                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 15,
                    thing: connecting_strokes_thing,
                    instance: Instance{
                        instance_position: [0.0, 0.0, 0.0],
                        instance_direction: [1.0, 0.0],
                        instance_color: [0.5, 0.5, 1.0]
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
        band_to_thing(&Band::new(Band::new(self.path(), 3.0).outline(), 0.3), 0.0)
    }

    fn create_interactables(&self, self_ref: PlanRef) {
        for (i, node) in self.nodes.iter().enumerate() {
            let child_ref = match self_ref {
                PlanRef(stroke_idx, _) => PlanRef(stroke_idx, i)
            };
            node.create_interactables(child_ref);
        }
    } 

    // TODO: this is really ugly
    fn cut_before(&self, offset: N) -> Self {
        let path = self.path();
        let cut_path = path.subsection(0.0, offset);
        RoadStroke{nodes: self.nodes.iter().filter(|node|
            cut_path.segments().iter().any(|segment|
                segment.start().is_roughly_within(node.position, 0.1) || segment.end().is_roughly_within(node.position, 0.1)
            )
        ).chain(&[RoadStrokeNode{
            position: path.along(offset), direction: None
        }]).cloned().collect()}
    }

    fn cut_after(&self, offset: N) -> Self {
        let path = self.path();
        let cut_path = path.subsection(offset, path.length());
        RoadStroke{nodes: (&[RoadStrokeNode{
            position: path.along(offset), direction: None
        }]).iter().chain(self.nodes.iter().filter(|node|
            cut_path.segments().iter().any(|segment|
                segment.start().is_roughly_within(node.position, 0.1) || segment.end().is_roughly_within(node.position, 0.1)
            )
        )).cloned().collect()}
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
    incoming: CVec<RoadStrokeNode>,
    outgoing: CVec<RoadStrokeNode>,
    connecting_strokes: CVec<RoadStroke>
}

#[derive(Compact, Clone)]
struct PlanUIState{
    current_node: Option<PlanRef>,
    dirty: bool
}

pub fn setup(system: &mut ActorSystem) {
    let plan = Plan{
        strokes: CVec::new(),
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