use descartes::{N, V2, P2, Norm, Segment, FiniteCurve, WithUniqueOrthogonal, RelativeToBasis, RoughlyComparable};
use kay::{Swarm, CVec, Recipient, CreateWith, ActorSystem, Individual, Fate};
use core::merge_groups::MergeGroups;
use itertools::Itertools;

//TODO: Clean up this whole mess with more submodules

mod plan;
mod road_stroke;
mod road_stroke_node_interactable;
mod road_stroke_canvas;
pub mod plan_result_steps;
pub mod materialized_reality;
pub mod current_plan_rendering;

pub use self::plan::{Plan, RoadStrokeRef, Intersection, IntersectionRef, TrimmedStrokeRef, TransferStrokeRef, PlanDelta, PlanResult, PlanResultDelta, RemainingOldStrokes};
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

#[derive(Compact, Clone)]
enum PlanControl{
    AddRoadStrokeNode(P2, bool),
    MoveRoadStrokeNodesTo(CVec<RoadStrokeNodeRef>, P2, P2),
    MaybeMakeCurrent(CVec<RoadStrokeNodeRef>, P2),
    ModifyRemainingOld(CVec<RoadStrokeRef>),
    CreateGrid(()),
    Materialize(())
}

const FINISH_STROKE_TOLERANCE : f32 = 5.0;

use self::materialized_reality::Simulate;
use self::materialized_reality::Apply;

impl Recipient<PlanControl> for CurrentPlan {
    fn receive(&mut self, msg: &PlanControl) -> Fate {match *msg{
        PlanControl::AddRoadStrokeNode(position, update_preview) => {
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

                            self.delta.new_strokes.push(RoadStroke::new(vec![
                                RoadStrokeNode{position: start + offset, direction: (position - start).normalize()},
                                RoadStrokeNode{position: position + offset, direction: (position - start).normalize()}
                            ].into()));
                            let right_lane_node_ref = RoadStrokeNodeRef(self.delta.new_strokes.len() - 1, 1);
                            
                            if self.ui_state.create_both_sides {
                                self.delta.new_strokes.push(RoadStroke::new(vec![
                                    RoadStrokeNode{position: position - offset, direction: (start - position).normalize()},
                                    RoadStrokeNode{position: start - offset, direction: (start - position).normalize()},
                                ].into()));
                                let left_lane_node_ref = RoadStrokeNodeRef(self.delta.new_strokes.len() - 1, 0);
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
                        let new_current_nodes = current_nodes.clone().iter().map(|&RoadStrokeNodeRef(stroke_idx, node_idx)| {
                            let stroke = &mut self.delta.new_strokes[stroke_idx];
                            let node = stroke.nodes[node_idx];
                            let relative_position_in_basis = (node.position - previous_add).to_basis(node.direction);

                            if node_idx == stroke.nodes.len() - 1 {
                                // append
                                let new_direction = Segment::arc_with_direction(previous_add, node.direction, position).end_direction();
                                stroke.nodes.push(RoadStrokeNode{
                                    position: position + relative_position_in_basis.from_basis(new_direction),
                                    direction: new_direction
                                });
                                RoadStrokeNodeRef(stroke_idx, stroke.nodes.len() - 1)
                            } else if node_idx == 0 {
                                // prepend
                                let new_direction = -Segment::arc_with_direction(previous_add, -node.direction, position).end_direction();
                                stroke.nodes.insert(0, RoadStrokeNode{
                                    position: position + relative_position_in_basis.from_basis(new_direction),
                                    direction: new_direction
                                });
                                RoadStrokeNodeRef(stroke_idx, 0)
                            } else {unreachable!()}
                        }).collect();

                        DrawingStatus::WithCurrentNodes(new_current_nodes, position)
                    }
                }
            };
            if update_preview {
                MaterializedReality::id() << Simulate{requester: Self::id(), delta: self.delta.clone(), fresh: false};
            }
            Fate::Live
        },
        PlanControl::MoveRoadStrokeNodesTo(ref node_refs, handle_from, handle_to) =>  {
            for &RoadStrokeNodeRef(stroke_idx, node_idx) in node_refs {
                let stroke = &mut self.delta.new_strokes[stroke_idx];
                let node = stroke.nodes[node_idx];
                if node_idx == stroke.nodes.len() - 1 {
                    let previous_node = stroke.nodes[node_idx - 1];
                    let relative_position_in_basis = (node.position - handle_from).to_basis(node.direction);
                    let previous_node_relative_position = relative_position_in_basis.from_basis(previous_node.direction);
                    let imaginary_previous_add = previous_node.position - previous_node_relative_position;
                    let new_direction = Segment::arc_with_direction(imaginary_previous_add, previous_node.direction, handle_to).end_direction();
                    let new_position = handle_to + relative_position_in_basis.from_basis(new_direction);
                    if (previous_node.position - new_position).norm() > MIN_NODE_DISTANCE {
                        stroke.nodes[node_idx].position = new_position;
                        stroke.nodes[node_idx].direction = new_direction
                    }
                } else if node_idx == 0 {
                    let next_node = stroke.nodes[node_idx + 1];
                    let relative_position_in_basis = (node.position - handle_from).to_basis(node.direction);
                    let next_node_relative_position = relative_position_in_basis.from_basis(next_node.direction);
                    let imaginary_next_add = next_node.position - next_node_relative_position;
                    let new_direction = -Segment::arc_with_direction(imaginary_next_add, -next_node.direction, handle_to).end_direction();
                    let new_position = handle_to + relative_position_in_basis.from_basis(new_direction);
                    if (next_node.position - new_position).norm() > MIN_NODE_DISTANCE {
                        stroke.nodes[node_idx].position = new_position;
                        stroke.nodes[node_idx].direction = new_direction
                    }
                } else {
                    let previous_node = stroke.nodes[node_idx - 1];
                    let next_node = stroke.nodes[node_idx + 1];
                    let new_position = node.position + (handle_to - handle_from);
                    if (previous_node.position - new_position).norm() > MIN_NODE_DISTANCE
                    && (next_node.position - new_position).norm() > MIN_NODE_DISTANCE {
                        stroke.nodes[node_idx].position = new_position;
                    }
                }
            }

            MaterializedReality::id() << Simulate{requester: Self::id(), delta: self.delta.clone(), fresh: false};
            Fate::Live
        },
        PlanControl::MaybeMakeCurrent(ref node_refs, handle_position) => {
            let strokes = &mut self.delta.new_strokes;
            let is_first_or_last = |&RoadStrokeNodeRef(stroke_idx, node_idx)| {
                node_idx == 0 || node_idx == strokes[stroke_idx].nodes.len() - 1
            };

            if node_refs.iter().all(is_first_or_last) {
                self.ui_state.drawing_status = DrawingStatus::WithCurrentNodes(node_refs.clone(), handle_position);
            }
            Fate::Live
        }
        PlanControl::ModifyRemainingOld(ref old_refs) => {
            for old_ref in old_refs {
                let old_stroke = self.current_remaining_old_strokes.mapping.get(*old_ref).unwrap();
                self.delta.strokes_to_destroy.insert(*old_ref, old_stroke.clone());
                self.delta.new_strokes.push(old_stroke.clone());
            }
            MaterializedReality::id() << Simulate{requester: Self::id(), delta: self.delta.clone(), fresh: false};
            Fate::Live
        },
        PlanControl::CreateGrid(()) => {
            let grid_size = 12u32;
            let grid_spacing = 300.0;

            for x in 0..grid_size {
                self.receive(&PlanControl::AddRoadStrokeNode(P2::new((x as f32 + 0.5) * grid_spacing, 0.0), false));
                self.receive(&PlanControl::AddRoadStrokeNode(P2::new((x as f32 + 0.5) * grid_spacing, grid_size as f32 * grid_spacing), false));
                self.receive(&PlanControl::AddRoadStrokeNode(P2::new((x as f32 + 0.5) * grid_spacing, grid_size as f32 * grid_spacing), false));
            }
            for y in 0..grid_size {
                self.receive(&PlanControl::AddRoadStrokeNode(P2::new(0.0, (y as f32 + 0.5) * grid_spacing), false));
                self.receive(&PlanControl::AddRoadStrokeNode(P2::new(grid_size as f32 * grid_spacing, (y as f32 + 0.5) * grid_spacing), false));
                self.receive(&PlanControl::AddRoadStrokeNode(P2::new(grid_size as f32 * grid_spacing, (y as f32 + 0.5) * grid_spacing), false));
            }
            MaterializedReality::id() << Simulate{requester: Self::id(), delta: self.delta.clone(), fresh: true};
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

#[derive(Compact, Clone)]
pub enum InteractableParent{
    New(()),
    WillBecomeNew(CVec<RoadStrokeRef>),
}

impl CurrentPlan{
    fn create_interactables(&self) {
        Swarm::<RoadStrokeCanvas>::all() << CreateWith(RoadStrokeCanvas::new(), AddToUI);
        let new_interactables = self.delta.new_strokes.iter().enumerate().flat_map(|(i, stroke)| {
            stroke.create_interactables(RoadStrokeRef(i), InteractableParent::New(()))
        }).collect::<Vec<_>>();
        let old_interactables = self.current_remaining_old_strokes.mapping.pairs().flat_map(|(old_ref, stroke)| {
            stroke.create_interactables(*old_ref, InteractableParent::WillBecomeNew(
                vec![RoadStrokeRef(self.delta.new_strokes.len())].into()
            ))
        });

        let mut interactable_groups = new_interactables.clone().into_iter().map(
            |interactable| vec![interactable]).collect::<Vec<_>>();
        interactable_groups.merge_groups(|group_1, group_2|
                group_1.iter().cartesian_product(group_2.iter()).any(|(interactable_1, interactable_2)|
                    (interactable_1.position - interactable_2.position).norm() < 20.0
                    && (interactable_1.direction.is_roughly_within(interactable_2.direction, 0.1)
                        || interactable_1.direction.is_roughly_within(-interactable_2.direction, 0.1))
                )
        );

        for interactable in new_interactables.into_iter().chain(old_interactables) {
            Swarm::<RoadStrokeNodeInteractable>::all() << CreateWith(interactable, AddToUI);
        }

        for interactable_group in interactable_groups {
            if interactable_group.len() > 1 {
                let position = (interactable_group.iter().fold(V2::new(0.0, 0.0),
                    |sum, interactable| sum + interactable.position.to_vector()
                ) / (interactable_group.len() as N)).to_point();

                let direction = interactable_group[0].direction;
                let node_refs = interactable_group.iter().flat_map(
                    |interactable| interactable.node_refs.clone()).collect();
                let group_interactable = RoadStrokeNodeInteractable::new(
                    position,
                    direction,
                    node_refs,
                    InteractableParent::New(())
                );
                Swarm::<RoadStrokeNodeInteractable>::all() << CreateWith(group_interactable, AddToUI);
            }
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
    system.add_inbox::<RecreateInteractables, CurrentPlan>();
    self::materialized_reality::setup(system);
    self::road_stroke_node_interactable::setup(system);
    self::road_stroke_canvas::setup(system);
}