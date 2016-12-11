use descartes::{N, P2, V2, Norm, Segment, FiniteCurve, WithUniqueOrthogonal, RelativeToBasis, RoughlyComparable, Dot};
use kay::{CVec, CDict, Recipient, Swarm, ActorSystem, Individual, Fate, CreateWith};
use itertools::Itertools;

//TODO: Clean up this whole mess with more submodules

mod plan;
mod lane_stroke;
mod lane_stroke_canvas;
mod lane_stroke_selectable;
mod lane_stroke_draggable;
pub mod plan_result_steps;
pub mod materialized_reality;
pub mod current_plan_rendering;

pub use self::plan::{Plan, LaneStrokeRef, Intersection, IntersectionRef, TrimmedStrokeRef, TransferStrokeRef, PlanDelta, PlanResult, PlanResultDelta, RemainingOldStrokes};
pub use self::lane_stroke::{LaneStroke, LaneStrokeNode, LaneStrokeNodeRef};
pub use self::lane_stroke_canvas::LaneStrokeCanvas;
use self::lane_stroke_selectable::LaneStrokeSelectable;
use self::lane_stroke_draggable::LaneStrokeDraggable;
use self::materialized_reality::MaterializedReality;
pub use self::lane_stroke::MIN_NODE_DISTANCE;

#[derive(Compact, Clone, Default)]
pub struct CurrentPlan {
    delta: PlanDelta,
    pub current_remaining_old_strokes: RemainingOldStrokes,
    pub current_plan_result_delta: PlanResultDelta,
    ui_state: PlanUIState
}
impl Individual for CurrentPlan{}

#[derive(Compact, Clone)]
enum PlanControl{
    AddLaneStrokeNode(P2, bool),
    Select(SelectableStrokeRef, N, N),
    MoveSelection(V2),
    DeleteSelection(()),
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
                    self.clear_selectables();
                    DrawingStatus::WithStartPoint(position)
                },
                DrawingStatus::WithStartPoint(start) => {
                    if (position - start).norm() < FINISH_STROKE_TOLERANCE {
                        self.create_selectables();
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
                        DrawingStatus::ContinuingFrom(new_node_refs.collect(), position)
                    }
                },
                DrawingStatus::ContinuingFrom(current_nodes, previous_add) => {
                    if (position - previous_add).norm() < FINISH_STROKE_TOLERANCE {
                        self.create_selectables();
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

                        DrawingStatus::ContinuingFrom(new_current_nodes, position)
                    }
                },
                DrawingStatus::WithSelections(_) => {
                    self.create_selectables();
                    DrawingStatus::Nothing(())
                }
            };
            if update_preview {
                MaterializedReality::id() << Simulate{requester: Self::id(), delta: self.delta.clone()};
            }
            Fate::Live
        },
        PlanControl::Select(selection_ref, start, end) => {
            if let DrawingStatus::WithSelections(ref mut selections) = self.ui_state.drawing_status {
                selections.insert(selection_ref, (start, end));
            } else {
                self.ui_state.drawing_status = DrawingStatus::WithSelections(
                    vec![(selection_ref, (start, end))].into_iter().collect()
                );
            }
            self.create_draggables();
            Fate::Live
        },
        PlanControl::MoveSelection(delta) => {
            if let DrawingStatus::WithSelections(ref selections) = self.ui_state.drawing_status {
                let mut with_subsections_moved = selections.pairs().map(|(&selection_ref, &(start, end))| {
                    let stroke = match selection_ref {
                        SelectableStrokeRef::New(node_idx) => &self.delta.new_strokes[node_idx],
                        SelectableStrokeRef::RemainingOld(old_ref) =>
                            self.current_remaining_old_strokes.mapping.get(old_ref).unwrap()
                    };
                    (selection_ref, stroke.with_subsection_moved(start, end, delta))
                }).collect::<::fnv::FnvHashMap<_, _>>();

                #[derive(PartialEq, Eq)]
                enum C{Before, After};

                let mut connector_alignments = Vec::<((SelectableStrokeRef, C), (SelectableStrokeRef, C))>::new();

                fn a_close_and_right_of_b(maybe_node_a: Option<&LaneStrokeNode>, maybe_node_b: Option<&LaneStrokeNode>) -> bool {
                    if let (Some(node_a), Some(node_b)) = (maybe_node_a, maybe_node_b) {
                        node_a.position.is_roughly_within(node_b.position, 7.0)
                        && (node_a.position - node_b.position).dot(&node_a.direction.orthogonal()) > 0.0
                    } else {false}
                }

                for (
                    (&selection_ref_a, &(_, ref maybe_before_connector_a, ref new_subsection_a, ref maybe_after_connector_a, _)),
                    (&selection_ref_b, &(_, ref maybe_before_connector_b, ref new_subsection_b, ref maybe_after_connector_b, _))
                ) in with_subsections_moved.iter().cartesian_product(with_subsections_moved.iter()).filter(|&((a, _), (b, _))| a != b) {
                    if a_close_and_right_of_b(new_subsection_a.get(0), new_subsection_b.get(0))
                    && maybe_before_connector_a.is_some() && maybe_before_connector_b.is_some() {
                        connector_alignments.push(((selection_ref_a, C::Before), (selection_ref_b, C::Before)));
                    }
                    if a_close_and_right_of_b(new_subsection_a.get(0), new_subsection_b.last())
                    && maybe_before_connector_a.is_some() && maybe_after_connector_b.is_some()
                    && !connector_alignments.iter().any(|other|
                        other == &((selection_ref_b, C::After), (selection_ref_a, C::Before))
                    ) {
                        connector_alignments.push(((selection_ref_a, C::Before), (selection_ref_b, C::After)));
                    }
                    if a_close_and_right_of_b(new_subsection_a.last(), new_subsection_b.last())
                    && maybe_after_connector_a.is_some() && maybe_after_connector_b.is_some() {
                        connector_alignments.push(((selection_ref_a, C::After), (selection_ref_b, C::After)));
                    }
                    if a_close_and_right_of_b(new_subsection_a.last(), new_subsection_b.get(0))
                    && maybe_after_connector_a.is_some() && maybe_before_connector_b.is_some()
                    && !connector_alignments.iter().any(|other|
                        other == &((selection_ref_b, C::Before), (selection_ref_a, C::After))
                    ) {
                        connector_alignments.push(((selection_ref_a, C::After), (selection_ref_b, C::Before)));
                    }
                }

                if connector_alignments.len() > 1 {
                    // figure out which alignments need to happen first
                    // yes, this is not optimal at all, but correct
                    while {
                        let mut something_happened = false;
                        #[allow(needless_range_loop)]
                        for i in 0..connector_alignments.len() {
                            let swap = {
                                let &(_, ref align_a_to) = &connector_alignments[i];
                                connector_alignments.iter().position(|&(ref b, _)| align_a_to == b)
                                    .and_then(|b_idx| if b_idx > i {Some(b_idx)} else {None})
                            };
                            if let Some(swap_with) = swap {
                                connector_alignments.swap(i, swap_with);
                                something_happened = true;
                                break;
                            }
                        }
                        something_happened
                    } {}
                } 

                for ((align_ref, align_connector), (align_to_ref, align_to_connector)) in connector_alignments {
                    let align_to = match align_to_connector {
                        C::Before => with_subsections_moved[&align_to_ref].1.unwrap(),
                        C::After => with_subsections_moved[&align_to_ref].3.unwrap()
                    };
                    let align = match align_connector {
                        C::Before => with_subsections_moved.get_mut(&align_ref).unwrap().1.as_mut().unwrap(),
                        C::After => with_subsections_moved.get_mut(&align_ref).unwrap().3.as_mut().unwrap()
                    };

                    let direction_sign = align.direction.dot(&align_to.direction).signum();
                    align.direction = direction_sign * align_to.direction;
                    let distance = if direction_sign < 0.0 {6.0} else {5.0};
                    align.position = align_to.position + distance * align.direction.orthogonal();
                }

                for (selection_ref, (b, bc, s, ac, a)) in with_subsections_moved {
                    let new_stroke = LaneStroke::new(
                        b.into_iter().chain(bc).chain(s).chain(ac).chain(a).collect()
                    );
                    match selection_ref {
                        SelectableStrokeRef::New(stroke_idx) => {
                            self.delta.new_strokes[stroke_idx] = new_stroke
                        },
                        SelectableStrokeRef::RemainingOld(old_ref) => {
                            let old_stroke = self.current_remaining_old_strokes.mapping.get(old_ref).unwrap();
                            self.delta.strokes_to_destroy.insert(old_ref, old_stroke.clone());
                            self.delta.new_strokes.push(new_stroke);
                        }
                    }
                }

            } else {unreachable!()}
            MaterializedReality::id() << Simulate{requester: Self::id(), delta: self.delta.clone()};
            self.ui_state.drawing_status = DrawingStatus::Nothing(());
            self.ui_state.dirty = true;
            self.ui_state.recreate_selectables = true;
            self.clear_selectables();
            self.clear_draggables();
            Fate::Live
        },
        PlanControl::DeleteSelection(()) => {
            if let DrawingStatus::WithSelections(ref selections) = self.ui_state.drawing_status {
                let mut new_stroke_indices_to_remove = Vec::new();
                let mut new_strokes = Vec::new();

                for (&selection_ref, &(start, end)) in selections.pairs() {
                    let stroke = match selection_ref {
                        SelectableStrokeRef::New(node_idx) => {
                            new_stroke_indices_to_remove.push(node_idx);
                            &self.delta.new_strokes[node_idx]
                        },
                        SelectableStrokeRef::RemainingOld(old_ref) => {
                            let old_stroke = self.current_remaining_old_strokes.mapping.get(old_ref).unwrap();
                            self.delta.strokes_to_destroy.insert(old_ref, old_stroke.clone());
                            old_stroke
                        }
                    };
                    if let Some(before) = stroke.subsection(0.0, start) {
                        new_strokes.push(before);
                    }
                    if let Some(after) = stroke.subsection(end, stroke.path().length()) {
                        new_strokes.push(after);
                    }
                }

                for index_to_remove in new_stroke_indices_to_remove {
                    self.delta.new_strokes.remove(index_to_remove);
                }

                for new_stroke in new_strokes {
                    self.delta.new_strokes.push(new_stroke);
                }
            }
            MaterializedReality::id() << Simulate{requester: Self::id(), delta: self.delta.clone()};
            self.ui_state.drawing_status = DrawingStatus::Nothing(());
            self.ui_state.dirty = true;
            self.ui_state.recreate_selectables = true;
            self.clear_selectables();
            self.clear_draggables();
            Fate::Live
        }
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
            self.clear_selectables();
            self.ui_state.recreate_selectables = true;
            Fate::Live
        }
    }}
}

use self::materialized_reality::SimulationResult;

impl Recipient<SimulationResult> for CurrentPlan{
    fn receive(&mut self, msg: &SimulationResult) -> Fate {match *msg{
        SimulationResult{ref remaining_old_strokes, ref result_delta} => {
            self.current_remaining_old_strokes = remaining_old_strokes.clone();
            self.current_plan_result_delta = result_delta.clone();
            self.ui_state.dirty = true;
            if self.ui_state.recreate_selectables {
                self.ui_state.recreate_selectables = false;
                self.create_selectables();
            }
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum SelectableStrokeRef{
    New(usize),
    RemainingOld(LaneStrokeRef)
}

impl CurrentPlan {
    fn create_selectables(&mut self) {
        for (stroke_idx, stroke) in self.delta.new_strokes.iter().enumerate() {
            Swarm::<LaneStrokeSelectable>::all() << CreateWith(
                LaneStrokeSelectable::new(SelectableStrokeRef::New(stroke_idx), stroke.path().clone()
            ), AddToUI);
        }
        for (old_ref, stroke) in self.current_remaining_old_strokes.mapping.pairs() {
            Swarm::<LaneStrokeSelectable>::all() << CreateWith(
                LaneStrokeSelectable::new(SelectableStrokeRef::RemainingOld(*old_ref), stroke.path().clone()
            ), AddToUI);
        }
    }

    fn create_draggables(&mut self) {
        if let DrawingStatus::WithSelections(ref selections) = self.ui_state.drawing_status {
            for (&selection_ref, &(start, end)) in selections.pairs() {
                let stroke = match selection_ref {
                    SelectableStrokeRef::New(stroke_idx) => &self.delta.new_strokes[stroke_idx],
                    SelectableStrokeRef::RemainingOld(old_stroke_ref) => self.current_remaining_old_strokes.mapping.get(old_stroke_ref).unwrap()
                };
                if let Some(subsection) = stroke.path().subsection(start, end) {
                    Swarm::<LaneStrokeDraggable>::all() << CreateWith(
                        LaneStrokeDraggable::new(selection_ref, subsection),
                        AddToUI
                    );
                }
            }
        } else {unreachable!()}
    }

    fn clear_selectables(&mut self) {
        Swarm::<LaneStrokeSelectable>::all() << ClearSelectables;
    }

    fn clear_draggables(&mut self) {
        Swarm::<LaneStrokeDraggable>::all() << ClearDraggables;
    }
}

#[derive(Compact, Clone)]
pub enum DrawingStatus{
    Nothing(()),
    WithStartPoint(P2),
    ContinuingFrom(CVec<LaneStrokeNodeRef>, P2),
    WithSelections(CDict<SelectableStrokeRef, (N, N)>)
}

#[derive(Compact, Clone)]
struct PlanUIState{
    create_both_sides: bool,
    n_lanes_per_side: usize,
    drawing_status: DrawingStatus,
    dirty: bool,
    recreate_selectables: bool
}

impl Default for PlanUIState{
    fn default() -> PlanUIState{
        PlanUIState{
            create_both_sides: true,
            n_lanes_per_side: 5,
            drawing_status: DrawingStatus::Nothing(()),
            dirty: true,
            recreate_selectables: true
        }
    }
}

#[derive(Copy, Clone)]
struct AddToUI;

#[derive(Copy, Clone)]
struct ClearSelectables;

#[derive(Copy, Clone)]
struct ClearDraggables;

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(CurrentPlan::default());
    system.add_inbox::<PlanControl, CurrentPlan>();
    system.add_inbox::<SimulationResult, CurrentPlan>();
    self::materialized_reality::setup(system);
    self::lane_stroke_canvas::setup(system);
    self::lane_stroke_selectable::setup(system);
    self::lane_stroke_draggable::setup(system);
}