use descartes::{P2, Norm, Segment, FiniteCurve};
use kay::{Swarm, CVec, Recipient, CreateWith, ActorSystem, Individual, Fate};

mod plan;
mod road_stroke;
mod road_stroke_node_interactable;
mod road_stroke_canvas;
pub mod materialized_reality;
pub mod current_plan_rendering;

pub use self::plan::{Plan, RoadStrokeRef, IntersectionRef, InbetweenStrokeRef, PlanDelta, PlanResult, PlanResultDelta, RemainingOldStrokes};
pub use self::road_stroke::{RoadStroke, RoadStrokeNode, RoadStrokeNodeRef};
pub use self::road_stroke_node_interactable::RoadStrokeNodeInteractable;
pub use self::road_stroke_canvas::RoadStrokeCanvas;
use self::materialized_reality::MaterializedReality;
pub use self::road_stroke::MIN_NODE_DISTANCE;

#[derive(Compact, Clone, Default)]
pub struct CurrentPlan {
    delta: PlanDelta,
    pub current_remaining_old_strokes: RemainingOldStrokes,
    pub current_plan_result: PlanResult,
    pub current_plan_result_delta: PlanResultDelta,
    ui_state: PlanUIState
}
impl Individual for CurrentPlan{}

#[derive(Copy, Clone)]
enum PlanControl{
    AddRoadStrokeNode(P2),
    MoveRoadStrokeNodeTo(RoadStrokeNodeRef, P2),
    ModifyRemainingOld(RoadStrokeRef),
    Materialize
}

const FINISH_STROKE_TOLERANCE : f32 = 5.0;

use self::materialized_reality::Simulate;
use self::materialized_reality::Apply;

impl Recipient<PlanControl> for CurrentPlan {
    fn receive(&mut self, msg: &PlanControl) -> Fate {match *msg{
        PlanControl::AddRoadStrokeNode(position) => {
            self.ui_state.drawing_status = match self.ui_state.drawing_status.clone() {
                DrawingStatus::Nothing(_) => {
                    DrawingStatus::WithStartPoint(position)
                },
                DrawingStatus::WithStartPoint(start) => {
                    if (position - start).norm() < FINISH_STROKE_TOLERANCE {
                        DrawingStatus::Nothing(())
                    } else {
                        self.delta.new_strokes.push(RoadStroke::new(vec![
                            RoadStrokeNode{position: start, direction: (position - start).normalize()},
                            RoadStrokeNode{position: position, direction: (position - start).normalize()}
                        ].into()));
                        DrawingStatus::WithCurrentNodes(
                            vec![RoadStrokeNodeRef(self.delta.new_strokes.len() - 1, 1)].into(),
                            position
                        )
                    }
                },
                DrawingStatus::WithCurrentNodes(current_nodes, previous_add) => {
                    if (position - previous_add).norm() < FINISH_STROKE_TOLERANCE {
                        DrawingStatus::Nothing(())
                    } else {
                        let new_current_nodes = current_nodes.clone().iter().map(|&RoadStrokeNodeRef(stroke_idx, node_idx)| {
                            let stroke = &mut self.delta.new_strokes[stroke_idx];

                            if node_idx == stroke.nodes.len() - 1 {
                                // append
                                let previous_node = stroke.nodes[node_idx];
                                stroke.nodes.push(RoadStrokeNode{
                                    position: position,
                                    direction: Segment::arc_with_direction(previous_node.position, previous_node.direction, position).end_direction()
                                });
                                RoadStrokeNodeRef(stroke_idx, stroke.nodes.len() - 1)
                            } else if node_idx == 0 {
                                // prepend
                                let next_node = stroke.nodes[1];
                                stroke.nodes.insert(0, RoadStrokeNode{
                                    position: position,
                                    direction: -Segment::arc_with_direction(next_node.position, -next_node.direction, position).end_direction()
                                });
                                RoadStrokeNodeRef(stroke_idx, 0)
                            } else {unreachable!()}
                        }).collect();

                        DrawingStatus::WithCurrentNodes(new_current_nodes, position)
                    }
                }
            };
            
            MaterializedReality::id() << Simulate{requester: Self::id(), delta: self.delta.clone(), fresh: false};
            Fate::Live
        },
        PlanControl::MoveRoadStrokeNodeTo(RoadStrokeNodeRef(stroke_idx, node_idx), position) =>  {
            {
                let stroke = &mut self.delta.new_strokes[stroke_idx];
                if node_idx == stroke.nodes.len() - 1 {
                    let previous_node = stroke.nodes[node_idx - 1];
                    if (previous_node.position - position).norm() > MIN_NODE_DISTANCE {
                        stroke.nodes[node_idx].position = position;
                        stroke.nodes[node_idx].direction = Segment::arc_with_direction(previous_node.position, previous_node.direction, position).end_direction();
                    }
                } else if node_idx == 0 {
                    let next_node = stroke.nodes[1];
                    if (next_node.position - position).norm() > MIN_NODE_DISTANCE {
                        stroke.nodes[node_idx].position = position;
                        stroke.nodes[node_idx].direction = -Segment::arc_with_direction(next_node.position, -next_node.direction, position).end_direction();
                    }
                } else {
                    let previous_node = stroke.nodes[node_idx - 1];
                    let next_node = stroke.nodes[node_idx + 1];
                    if (previous_node.position - position).norm() > MIN_NODE_DISTANCE
                    && (next_node.position - position).norm() > MIN_NODE_DISTANCE {
                        stroke.nodes[node_idx].position = position;
                    }
                }
            }
            MaterializedReality::id() << Simulate{requester: Self::id(), delta: self.delta.clone(), fresh: false};
            Fate::Live
        },
        PlanControl::ModifyRemainingOld(old_ref) => {
            let old_stroke = self.current_remaining_old_strokes.mapping.get(old_ref).unwrap();
            self.delta.strokes_to_destroy.insert(old_ref, old_stroke.clone());
            self.delta.new_strokes.push(old_stroke.clone());
            MaterializedReality::id() << Simulate{requester: Self::id(), delta: self.delta.clone(), fresh: false};
            Fate::Live
        }
        PlanControl::Materialize => {
            MaterializedReality::id() << Apply{requester: Self::id(), delta: self.delta.clone()};
            *self = CurrentPlan::default();
            Fate::Live
        }
    }}
}

use self::materialized_reality::SimulationResult;

impl Recipient<SimulationResult> for CurrentPlan{
    fn receive(&mut self, msg: &SimulationResult) -> Fate {match *msg{
        SimulationResult{ref remaining_old_strokes, ref result, ref result_delta, fresh} => {
            self.current_remaining_old_strokes = remaining_old_strokes.clone();
            self.current_plan_result = result.clone();
            self.current_plan_result_delta = result_delta.clone();
            self.ui_state.dirty = true;
            if fresh {Self::id() << RecreateInteractables}
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

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum InteractableParent{
    New,
    RemainingOldStroke{new_ref_to_become: RoadStrokeRef},
}

impl CurrentPlan{
    fn create_interactables(&self) {
        Swarm::<RoadStrokeCanvas>::all() << CreateWith(RoadStrokeCanvas::new(), AddToUI);
        for (i, stroke) in self.delta.new_strokes.iter().enumerate() {
            stroke.create_interactables(RoadStrokeRef(i), InteractableParent::New);
        }
        for (old_ref, stroke) in self.current_remaining_old_strokes.mapping.pairs() {
            stroke.create_interactables(*old_ref, InteractableParent::RemainingOldStroke{
                new_ref_to_become: RoadStrokeRef(self.delta.new_strokes.len())
            });
        }
    }
}

#[derive(Compact, Clone)]
pub enum DrawingStatus{
    Nothing(()),
    WithStartPoint(P2),
    WithCurrentNodes(CVec<RoadStrokeNodeRef>, P2)
}

#[derive(Compact, Clone)]
struct PlanUIState{
    create_both_sides: bool,
    drawing_status: DrawingStatus,
    dirty: bool
}

impl Default for PlanUIState{
    fn default() -> PlanUIState{
        PlanUIState{
            create_both_sides: true,
            drawing_status: DrawingStatus::Nothing(()),
            dirty: true
        }
    }
}

#[derive(Copy, Clone)]
struct AddToUI;

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(CurrentPlan::default());
    system.add_inbox::<PlanControl, CurrentPlan>();
    system.add_inbox::<SimulationResult, CurrentPlan>();
    system.add_inbox::<RecreateInteractables, CurrentPlan>();
    self::materialized_reality::setup(system);
    self::road_stroke_node_interactable::setup(system);
    self::road_stroke_canvas::setup(system);
}