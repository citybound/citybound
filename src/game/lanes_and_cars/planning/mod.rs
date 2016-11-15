use descartes::{P2, Path, Band, Intersect, convex_hull};
use kay::{ID, CVec, Swarm, Recipient, CreateWith, ActorSystem, Individual, Fate};
use monet::{Instance, Thing, Norm};
use core::geometry::{CPath, band_to_thing};
use ordered_float::OrderedFloat;

mod road_stroke;
mod road_stroke_node_interactable;
mod road_stroke_canvas;
mod materialized_plan;

pub use self::road_stroke::{RoadStroke, RoadStrokeNode};
pub use self::road_stroke_node_interactable::RoadStrokeNodeInteractable;
pub use self::road_stroke_canvas::RoadStrokeCanvas;
pub use self::materialized_plan::{MaterializedPlan, ReportLaneBuilt};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PlanRef{
    StrokeNode(usize, usize),
    Stroke(usize),
    CutStroke(usize),
    Intersection(usize),
    IntersectionStroke(usize, usize)
}

#[derive(Compact, Clone)]
pub struct Plan {
    strokes: CVec<RoadStroke>,
    intersections: CVec<Intersection>,
    strokes_after_cutting: CVec<RoadStroke>,
}

#[derive(Compact, Clone)]
pub struct CurrentPlan {
    plan: Plan,
    calculation_state: CalculcationState,
    ui_state: PlanUIState
}
impl Individual for CurrentPlan{}

#[derive(Copy, Clone)]
enum PlanControl{
    AddRoadStrokeNode(P2),
    MoveRoadStrokeNodeTo(PlanRef, P2),
    Materialize
}

use self::materialized_plan::Build;
use self::materialized_plan::Unbuild;

impl Recipient<PlanControl> for CurrentPlan {
    fn receive(&mut self, msg: &PlanControl) -> Fate {match *msg{
        PlanControl::AddRoadStrokeNode(at) => {
            let new_node = RoadStrokeNode{position: at, direction: None};
            
            if let Some(PlanRef::StrokeNode(stroke_idx, node_idx)) = self.ui_state.current_node {
                let stroke = &mut self.plan.strokes[stroke_idx];
                let current_node = stroke.nodes[node_idx];

                if (current_node.position - new_node.position).norm() < 5.0 {
                    // finish stroke
                    self.ui_state.current_node = None;
                } else if node_idx == stroke.nodes.len() - 1 {
                    // append
                    stroke.nodes.push(new_node);
                    self.ui_state.current_node = Some(PlanRef::StrokeNode(stroke_idx, stroke.nodes.len() - 1));
                } else if node_idx == 0 {
                    // prepend
                    stroke.nodes.insert(0, new_node);
                } else {unreachable!()}
            } else {
                self.plan.strokes.push(RoadStroke{
                    nodes: vec![new_node].into()
                });
                self.ui_state.current_node = Some(PlanRef::StrokeNode(self.plan.strokes.len() - 1, 0))
            }

            self.recalculate();
            Fate::Live
        },
        PlanControl::MoveRoadStrokeNodeTo(plan_ref, position) => match plan_ref {
            PlanRef::StrokeNode(stroke, node) => {
                self.plan.strokes[stroke].nodes[node].position = position;
                self.recalculate();
                Fate::Live
            },
            _ => unreachable!()
        },
        PlanControl::Materialize => {
            for &(affected_plan_id, replaced_intersection) in self.calculation_state.replaced_intersections.iter() {
                affected_plan_id << Unbuild(replaced_intersection);
            }
            for &(affected_plan_id, stroke_to_unbuild) in self.calculation_state.cut_strokes_to_debuild.iter() {
                affected_plan_id << Unbuild(stroke_to_unbuild);
            }
            Swarm::<MaterializedPlan>::all() << CreateWith(MaterializedPlan::new(self.plan.clone()), Build);
            *self = CurrentPlan::default();
            CurrentPlan::id() << RecreateInteractables;
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
struct RecreateInteractables;
#[derive(Copy, Clone)]
struct ClearAll;

impl Recipient<RecreateInteractables> for CurrentPlan {
    fn receive(&mut self, _msg: &RecreateInteractables) -> Fate {
        Swarm::<RoadStrokeNodeInteractable>::all() << ClearAll;
        Swarm::<RoadStrokeCanvas>::all() << ClearAll;
        self.create_interactables();
        Fate::Live
    }
}

use monet::SetupInScene;

impl Recipient<SetupInScene> for CurrentPlan {
    fn receive(&mut self, _msg: &SetupInScene) -> Fate {
        self.create_interactables();
        Fate::Live
    }
}

use monet::RenderToScene;
use monet::UpdateThing;

impl Recipient<RenderToScene> for CurrentPlan {
    fn receive(&mut self, msg: &RenderToScene) -> Fate {match *msg{
        RenderToScene{renderer_id, scene_id} => {
            if self.ui_state.dirty {
                let thing : Thing = self.plan.strokes_after_cutting.iter()
                    .filter(|stroke| stroke.nodes.len() > 1)
                    .map(RoadStroke::preview_thing)
                    .sum();
                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 3834747834,
                    thing: thing,
                    instance: Instance::with_color([0.3, 0.3, 0.5])
                };
                let intersections_thing : Thing = self.plan.intersections.iter()
                    .filter(|i| i.shape.segments().len() > 0)
                    .map(|i| band_to_thing(&Band::new(i.shape.clone(), 0.4), 0.5))
                    .sum();
                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 3834747835,
                    thing: intersections_thing,
                    instance: Instance::with_color([0.0, 0.0, 1.0])
                };
                let connecting_strokes_thing : Thing = self.plan.intersections.iter()
                    .filter(|i| !i.connecting_strokes.is_empty())
                    .map(|i| -> Thing {i.connecting_strokes.iter().map(RoadStroke::preview_thing).sum()})
                    .sum();
                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 3834747836,
                    thing: connecting_strokes_thing,
                    instance: Instance::with_color([0.5, 0.5, 1.0])
                };
                self.ui_state.dirty = false;
            }
            Fate::Live
        }
    }}
}

use self::materialized_plan::CollectIntersectionPoints;
pub const INTERSECTION_GROUPING_RADIUS : f32 = 20.0;
use kay::{RequestConfirmation};

impl CurrentPlan{
    fn create_interactables(&self) {
        Swarm::<RoadStrokeCanvas>::all() << CreateWith(RoadStrokeCanvas::new(), AddToUI);
        for (i, stroke) in self.plan.strokes.iter().enumerate() {
            stroke.create_interactables(PlanRef::Stroke(i));
        }
    }
}

#[derive(Compact, Clone, Default)]
struct CalculcationState {
    intersection_points: CVec<P2>,
    replaced_intersections: CVec<(ID, PlanRef)>,
    cut_strokes_to_debuild: CVec<(ID, PlanRef)>,
    n_received_intersection_point_messages: usize,
    n_expected_intersection_point_messages: Option<usize>,
    n_received_intersection_messages: usize,
    n_expected_intersection_messages: Option<usize>,
}

impl CurrentPlan {
    fn recalculate(&mut self) {
        self.calculation_state = CalculcationState::default();
        self.ui_state.dirty = true;
        self.collect_intersection_points();
        Swarm::<MaterializedPlan>::all() << RequestConfirmation{
            requester: CurrentPlan::id(),
            message: CollectIntersectionPoints{
                requester: CurrentPlan::id(),
                other_strokes: self.plan.strokes.clone(),
                other_points: self.calculation_state.intersection_points.clone()
            }
        };
    }
}

use self::materialized_plan::ChangesAndIntersectionPoints;
use self::materialized_plan::IntersectWith;

impl Recipient<ChangesAndIntersectionPoints> for CurrentPlan {
    fn receive(&mut self, msg: &ChangesAndIntersectionPoints) -> Fate {match *msg{
        ChangesAndIntersectionPoints{affected_plan_id, ref replaced_intersections, ref points} => {
            self.calculation_state.intersection_points.extend(points.iter().cloned());
            self.calculation_state.replaced_intersections.extend(
                replaced_intersections.iter().map(|index| (affected_plan_id, *index))
            );
            self.calculation_state.n_received_intersection_point_messages += 1;
            if let Some(n_expected_messages) = self.calculation_state.n_expected_intersection_point_messages {
                if n_expected_messages == self.calculation_state.n_received_intersection_point_messages {
                    self.recalculate_step_2();
                }
            }
            Fate::Live
        }
    }}
}

use kay::Confirmation;

impl Recipient<Confirmation<CollectIntersectionPoints>> for CurrentPlan {
    fn receive(&mut self, msg: &Confirmation<CollectIntersectionPoints>) -> Fate {match *msg{
        Confirmation{n_recipients, ..} => {
            self.calculation_state.n_expected_intersection_point_messages = Some(n_recipients);
            if self.calculation_state.n_received_intersection_point_messages == n_recipients {
                self.recalculate_step_2();
            }
            Fate::Live
        }
    }}
}

impl CurrentPlan {
    fn recalculate_step_2(&mut self) {
        self.create_intersections();
        self.cut_strokes_at_intersections();
        Swarm::<MaterializedPlan>::all() << RequestConfirmation{
            requester: CurrentPlan::id(),
            message: IntersectWith{
                requester: CurrentPlan::id(),
                new_intersections: self.plan.intersections.clone(),
                replaced_intersections: self.calculation_state.replaced_intersections.clone()
            }
        };
    }
}

use self::materialized_plan::ChangesAfterIntersecting;

impl Recipient<ChangesAfterIntersecting> for CurrentPlan {
    fn receive(&mut self, msg: &ChangesAfterIntersecting) -> Fate {match *msg{
        ChangesAfterIntersecting{affected_plan_id, ref updated_intersections, ref new_cut_strokes, ref cut_strokes_to_debuild} => {
            for (i, updated_intersection) in updated_intersections.iter().enumerate() {
                self.plan.intersections[i].incoming.extend(updated_intersection.incoming.iter().cloned());
                self.plan.intersections[i].outgoing.extend(updated_intersection.outgoing.iter().cloned());
            }

            self.plan.strokes_after_cutting.extend(new_cut_strokes.iter().cloned());
            self.calculation_state.cut_strokes_to_debuild.extend(
                cut_strokes_to_debuild.iter().map(|plan_ref| (affected_plan_id, *plan_ref))
            );

            self.calculation_state.n_received_intersection_messages += 1;
            if let Some(n_expected_messages) = self.calculation_state.n_expected_intersection_messages {
                if self.calculation_state.n_received_intersection_messages == n_expected_messages {
                    self.recalculate_step_3();
                }
            }
            Fate::Live
        }
    }}
}

impl Recipient<Confirmation<IntersectWith>> for CurrentPlan {
    fn receive(&mut self, msg: &Confirmation<IntersectWith>) -> Fate {match *msg{
        Confirmation{n_recipients, ..} => {
            self.calculation_state.n_expected_intersection_messages = Some(n_recipients);
            if self.calculation_state.n_received_intersection_messages == n_recipients {
                self.recalculate_step_3();
            }
            Fate::Live
        }
    }}
}

impl CurrentPlan {
    fn recalculate_step_3(&mut self) {
        self.create_connecting_strokes_on_intersections();
        self.ui_state.dirty = true;
    }
}

impl CurrentPlan {
    fn collect_intersection_points(&mut self) {
        let mut all_intersection_points = Vec::new();
        
        for i in 0..self.plan.strokes.len() {
            let stroke1 = &self.plan.strokes[i];
            if stroke1.nodes.len() > 1 {
                let band1 = Band::new(stroke1.path(), 8.0).outline();
                for j in (i + 1)..self.plan.strokes.len() {
                    let stroke2 = &self.plan.strokes[j];
                    if stroke2.nodes.len() > 1 {
                        let band2 = Band::new(stroke2.path(), 8.0).outline();

                        let intersections = (&band1, &band2).intersect();
                        all_intersection_points.extend(intersections.iter().map(|i| i.position));
                    }
                }
            }
        }

        self.calculation_state.intersection_points = all_intersection_points.into();
        
    }

    fn create_intersections(&mut self) {
        let mut intersection_point_groups = Vec::<Vec<P2>>::new();

        for point in self.calculation_state.intersection_points.iter() {
            let create_new_group = match intersection_point_groups.iter_mut().find(|group| {
                let max_distance = group.iter().map(|other_point| OrderedFloat((*other_point - *point).norm())).max().unwrap();
                *max_distance < INTERSECTION_GROUPING_RADIUS
            }) {
                Some(exisiting_group) => {exisiting_group.push(*point); false},
                None => true
            };

            if create_new_group {
                intersection_point_groups.push(vec![*point].into());
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

        self.plan.intersections = intersection_point_groups.iter().filter_map(|group|
            if group.len() >= 2 {
                Some(Intersection{
                    shape: convex_hull::<CPath>(group),
                    points: group.clone().into(),
                    incoming: CVec::new(),
                    outgoing: CVec::new(),
                    connecting_strokes: CVec::new()
                })
            } else {None}
        ).collect()
    }

    fn cut_strokes_at_intersections(&mut self) {
        let strokes_after_cutting = cut_strokes_at_intersections(&self.plan.strokes, &mut self.plan.intersections);
        self.plan.strokes_after_cutting = strokes_after_cutting;
    }

    fn create_connecting_strokes_on_intersections(&mut self) {
        for intersection in self.plan.intersections.iter_mut() {
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

pub fn cut_strokes_at_intersections(strokes: &CVec<RoadStroke>, intersections: &mut CVec<Intersection>) -> CVec<RoadStroke> {
    let mut strokes_todo : Vec<_> = strokes.iter().cloned().collect();

    let mut cutting_ongoing = true;
    let mut iters = 0;
    while cutting_ongoing {
        cutting_ongoing = false;
        let mut new_strokes = Vec::new();

        for stroke in &strokes_todo {
            let mut was_cut = false;

            for intersection in intersections.iter_mut() {
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

    strokes_todo.into()
}

impl Default for CurrentPlan {
    fn default() -> CurrentPlan {
        CurrentPlan{
            plan: Plan{
                strokes: CVec::new(),
                strokes_after_cutting: CVec::new(),
                intersections: CVec::new()
            },
            calculation_state: CalculcationState::default(),
            ui_state: PlanUIState{
                current_node: None,
                dirty: true
            }
        }
    }
}

#[derive(Compact, Clone)]
pub struct Intersection{
    points: CVec<P2>,
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

#[derive(Copy, Clone)]
struct AddToUI;

pub fn setup(system: &mut ActorSystem) {
    let current_plan = CurrentPlan::default();

    system.add_individual(current_plan);
    system.add_inbox::<SetupInScene, CurrentPlan>();
    system.add_inbox::<RenderToScene, CurrentPlan>();
    system.add_inbox::<PlanControl, CurrentPlan>();
    system.add_inbox::<RecreateInteractables, CurrentPlan>();
    system.add_inbox::<ChangesAndIntersectionPoints, CurrentPlan>();
    system.add_inbox::<Confirmation<CollectIntersectionPoints>, CurrentPlan>();
    system.add_inbox::<ChangesAfterIntersecting, CurrentPlan>();
    system.add_inbox::<Confirmation<IntersectWith>, CurrentPlan>();

    road_stroke_canvas::setup(system);
    road_stroke_node_interactable::setup(system);
    materialized_plan::setup(system);
}