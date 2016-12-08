use descartes::{N, P2, Norm, Segment, FiniteCurve, WithUniqueOrthogonal, RelativeToBasis};
use kay::{CVec, Recipient, ActorSystem, Individual, Fate};

//TODO: Clean up this whole mess with more submodules

mod plan;
mod lane_stroke;
mod lane_stroke_canvas;
pub mod plan_result_steps;
pub mod materialized_reality;
pub mod current_plan_rendering;

pub use self::plan::{Plan, LaneStrokeRef, Intersection, IntersectionRef, TrimmedStrokeRef, TransferStrokeRef, PlanDelta, PlanResult, PlanResultDelta, RemainingOldStrokes};
pub use self::lane_stroke::{LaneStroke, LaneStrokeNode, LaneStrokeNodeRef};
pub use self::lane_stroke_canvas::LaneStrokeCanvas;
use self::materialized_reality::MaterializedReality;
pub use self::lane_stroke::MIN_NODE_DISTANCE;

#[derive(Compact, Clone, Default)]
pub struct CurrentPlan {
    delta: PlanDelta,
    pub current_remaining_old_strokes: RemainingOldStrokes,
    pub current_plan_result: PlanResult,
    pub current_plan_result_delta: PlanResultDelta,
    ui_state: PlanUIState
}
impl Individual for CurrentPlan{}

#[derive(Compact, Clone)]
enum PlanControl{
    AddLaneStrokeNode(P2, bool),
    ModifyRemainingOld(CVec<LaneStrokeRef>),
    CreateGrid(()),
    Materialize(())
}

const FINISH_STROKE_TOLERANCE : f32 = 5.0;

use self::materialized_reality::Simulate;
use self::materialized_reality::Apply;

impl Recipient<PlanControl> for CurrentPlan {
    fn receive(&mut self, msg: &PlanControl) -> Fate {match *msg{
        PlanControl::AddLaneStrokeNode(position, update_preview) => {
            self.ui_state.drawing_status = match self.ui_state.drawing_status.clone() {
                DrawingStatus::Nothing(_) => {
                    DrawingStatus::WithStartPoint(position)
                },
                DrawingStatus::WithStartPoint(start) => {
                    if (position - start).norm() < FINISH_STROKE_TOLERANCE {
                        DrawingStatus::Nothing(())
                    } else {
                        let new_node_refs = (0..self.ui_state.n_lanes_per_side).into_iter().flat_map(|lane_idx| {
                            let offset = (position - start).normalize().orthogonal() * (3.0 + 5.0 * lane_idx as N);

                            self.delta.new_strokes.push(LaneStroke::new(vec![
                                LaneStrokeNode{position: start + offset, direction: (position - start).normalize()},
                                LaneStrokeNode{position: position + offset, direction: (position - start).normalize()}
                            ].into()));
                            let right_lane_node_ref = LaneStrokeNodeRef(self.delta.new_strokes.len() - 1, 1);
                            
                            if self.ui_state.create_both_sides {
                                self.delta.new_strokes.push(LaneStroke::new(vec![
                                    LaneStrokeNode{position: position - offset, direction: (start - position).normalize()},
                                    LaneStrokeNode{position: start - offset, direction: (start - position).normalize()},
                                ].into()));
                                let left_lane_node_ref = LaneStrokeNodeRef(self.delta.new_strokes.len() - 1, 0);
                                vec![right_lane_node_ref, left_lane_node_ref]
                            } else {
                                vec![right_lane_node_ref]
                            }
                        });
                        DrawingStatus::WithCurrentNodes(new_node_refs.collect(), position)
                    }
                },
                DrawingStatus::WithCurrentNodes(current_nodes, previous_add) => {
                    if (position - previous_add).norm() < FINISH_STROKE_TOLERANCE {
                        DrawingStatus::Nothing(())
                    } else {
                        let new_current_nodes = current_nodes.clone().iter().map(|&LaneStrokeNodeRef(stroke_idx, node_idx)| {
                            let stroke = &mut self.delta.new_strokes[stroke_idx];
                            let node = stroke.nodes()[node_idx];
                            let relative_position_in_basis = (node.position - previous_add).to_basis(node.direction);

                            if node_idx == stroke.nodes().len() - 1 {
                                // append
                                let new_direction = Segment::arc_with_direction(previous_add, node.direction, position).end_direction();
                                stroke.nodes_mut().push(LaneStrokeNode{
                                    position: position + relative_position_in_basis.from_basis(new_direction),
                                    direction: new_direction
                                });
                                LaneStrokeNodeRef(stroke_idx, stroke.nodes().len() - 1)
                            } else if node_idx == 0 {
                                // prepend
                                let new_direction = -Segment::arc_with_direction(previous_add, -node.direction, position).end_direction();
                                stroke.nodes_mut().insert(0, LaneStrokeNode{
                                    position: position + relative_position_in_basis.from_basis(new_direction),
                                    direction: new_direction
                                });
                                LaneStrokeNodeRef(stroke_idx, 0)
                            } else {unreachable!()}
                        }).collect();

                        DrawingStatus::WithCurrentNodes(new_current_nodes, position)
                    }
                }
            };
            if update_preview {
                MaterializedReality::id() << Simulate{requester: Self::id(), delta: self.delta.clone()};
            }
            Fate::Live
        },
        PlanControl::ModifyRemainingOld(ref old_refs) => {
            for old_ref in old_refs {
                let old_stroke = self.current_remaining_old_strokes.mapping.get(*old_ref).unwrap();
                self.delta.strokes_to_destroy.insert(*old_ref, old_stroke.clone());
                self.delta.new_strokes.push(old_stroke.clone());
            }
            MaterializedReality::id() << Simulate{requester: Self::id(), delta: self.delta.clone()};
            Fate::Live
        },
        PlanControl::CreateGrid(()) => {
            let grid_size = 18u32;
            let grid_spacing = 400.0;

            for x in 0..grid_size {
                self.receive(&PlanControl::AddLaneStrokeNode(P2::new((x as f32 + 0.5) * grid_spacing, 0.0), false));
                self.receive(&PlanControl::AddLaneStrokeNode(P2::new((x as f32 + 0.5) * grid_spacing, grid_size as f32 * grid_spacing), false));
                self.receive(&PlanControl::AddLaneStrokeNode(P2::new((x as f32 + 0.5) * grid_spacing, grid_size as f32 * grid_spacing), false));
            }
            for y in 0..grid_size {
                self.receive(&PlanControl::AddLaneStrokeNode(P2::new(0.0, (y as f32 + 0.5) * grid_spacing), false));
                self.receive(&PlanControl::AddLaneStrokeNode(P2::new(grid_size as f32 * grid_spacing, (y as f32 + 0.5) * grid_spacing), false));
                self.receive(&PlanControl::AddLaneStrokeNode(P2::new(grid_size as f32 * grid_spacing, (y as f32 + 0.5) * grid_spacing), false));
            }
            MaterializedReality::id() << Simulate{requester: Self::id(), delta: self.delta.clone()};
            self.ui_state.dirty = true;
            Fate::Live
        },
        PlanControl::Materialize(()) => {
            MaterializedReality::id() << Apply{requester: Self::id(), delta: self.delta.clone()};
            *self = CurrentPlan::default();
            Fate::Live
        }
    }}
}

use self::materialized_reality::SimulationResult;

impl Recipient<SimulationResult> for CurrentPlan{
    fn receive(&mut self, msg: &SimulationResult) -> Fate {match *msg{
        SimulationResult{ref remaining_old_strokes, ref result, ref result_delta} => {
            self.current_remaining_old_strokes = remaining_old_strokes.clone();
            self.current_plan_result = result.clone();
            self.current_plan_result_delta = result_delta.clone();
            self.ui_state.dirty = true;
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub enum InteractableParent{
    New(()),
    WillBecomeNew(CVec<LaneStrokeRef>),
}
    
#[derive(Compact, Clone)]
pub enum DrawingStatus{
    Nothing(()),
    WithStartPoint(P2),
    WithCurrentNodes(CVec<LaneStrokeNodeRef>, P2)
}

#[derive(Compact, Clone)]
struct PlanUIState{
    create_both_sides: bool,
    n_lanes_per_side: usize,
    drawing_status: DrawingStatus,
    dirty: bool
}

impl Default for PlanUIState{
    fn default() -> PlanUIState{
        PlanUIState{
            create_both_sides: true,
            n_lanes_per_side: 3,
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
    self::materialized_reality::setup(system);
    self::lane_stroke_canvas::setup(system);
}