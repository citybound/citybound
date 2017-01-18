use descartes::{N, P2, V2, Norm, Segment, FiniteCurve, WithUniqueOrthogonal, Curve,
                RelativeToBasis, RoughlyComparable, Dot};
use compact::{CVec, CDict};
use kay::{Recipient, ActorSystem, Actor, Fate};
use kay::swarm::{Swarm, CreateWith};
use itertools::Itertools;
use ordered_float::OrderedFloat;

// TODO: Clean up this whole mess with more submodules

mod plan;
mod lane_stroke;
mod lane_stroke_canvas;
mod lane_stroke_selectable;
mod lane_stroke_draggable;
mod lane_stroke_addable;
pub mod plan_result_steps;
pub mod materialized_reality;
pub mod current_plan_rendering;

pub use self::plan::{Plan, LaneStrokeRef, Intersection, IntersectionRef, TrimmedStrokeRef,
                     TransferStrokeRef, PlanDelta, PlanResult, PlanResultDelta,
                     RemainingOldStrokes};
pub use self::lane_stroke::{LaneStroke, LaneStrokeNode, LaneStrokeNodeRef};
pub use self::lane_stroke_canvas::LaneStrokeCanvas;
use self::lane_stroke_selectable::LaneStrokeSelectable;
use self::lane_stroke_draggable::LaneStrokeDraggable;
use self::lane_stroke_addable::LaneStrokeAddable;
use self::materialized_reality::MaterializedReality;
pub use self::lane_stroke::MIN_NODE_DISTANCE;

#[derive(Compact, Clone, Default)]
pub struct PlanState {
    delta: PlanDelta,
    pub current_remaining_old_strokes: RemainingOldStrokes,
    pub current_plan_result_delta: PlanResultDelta,
    ui_state: PlanUIState,
}

#[derive(Compact, Clone, Default)]
pub struct CurrentPlan {
    undo_history: CVec<PlanState>,
    redo_history: CVec<PlanState>,
    current: PlanState,
    preview: PlanState,
}
impl Actor for CurrentPlan {}

const FINISH_STROKE_TOLERANCE: f32 = 5.0;

use self::materialized_reality::Simulate;
use self::materialized_reality::Apply;

#[derive(Copy, Clone)]
struct Commit(bool, P2);

impl Recipient<Commit> for CurrentPlan {
    fn receive(&mut self, msg: &Commit) -> Fate {
        match *msg {
            Commit(update_preview_and_history, at) => {
                if update_preview_and_history {
                    self.undo_history.push(self.current.clone());
                    self.redo_history.clear();
                }
                self.current = self.preview.clone();
                if update_preview_and_history {
                    match self.preview.ui_state.drawing_status {
                        DrawingStatus::WithStartPoint(..) => {
                            self.clear_selectables();
                        }
                        DrawingStatus::Nothing(()) => {
                        self.preview.ui_state.recreate_selectables = true;
                        MaterializedReality::id() <<
                        Simulate{requester: Self::id(), delta: self.preview.delta.clone()};
                    }
                        DrawingStatus::WithSelections(_, clear_next_commit) => {
                            if clear_next_commit {
                                // ugly hack to clear selections on certain commits
                                self.clear_selectables();
                                self.clear_draggables();
                                self.current.ui_state.drawing_status = DrawingStatus::Nothing(());
                                self.preview.ui_state.drawing_status = DrawingStatus::Nothing(());
                                self.preview.ui_state.recreate_selectables = true;
                                MaterializedReality::id() <<
                                Simulate {
                                    requester: Self::id(),
                                    delta: self.preview.delta.clone(),
                                };
                            } else {
                                // to prevent borrow of self
                                let selections = match self.preview.ui_state.drawing_status {
                                    DrawingStatus::WithSelections(ref selections, _) => {
                                        selections.clone()
                                    }
                                    _ => unreachable!(),
                                };
                                #[derive(PartialEq, Eq)]
                                enum SelectionMeaning {
                                    Start,
                                    SubSection,
                                    End,
                                };
                                let meanings = selections.pairs()
                                    .map(|(selection_ref, &(start, end))| {
                                        let stroke = match *selection_ref {
                                            SelectableStrokeRef::New(node_idx) => {
                                                &self.preview.delta.new_strokes[node_idx]
                                            }
                                            SelectableStrokeRef::RemainingOld(old_ref) => {
                                                self.preview
                                                    .current_remaining_old_strokes
                                                    .mapping
                                                    .get(old_ref)
                                                    .unwrap()
                                            }
                                        };
                                        if start.is_roughly_within(0.0, 6.0) &&
                                           end.is_roughly_within(0.0, 6.0) {
                                            SelectionMeaning::Start
                                        } else if start.is_roughly_within(stroke.path()
                                                                              .length(),
                                                                          6.0) &&
                                                  end.is_roughly_within(stroke.path()
                                                                            .length(),
                                                                        6.0) {
                                            SelectionMeaning::End
                                        } else {
                                            SelectionMeaning::SubSection
                                        }
                                    })
                                    .collect::<Vec<_>>();
                                if meanings.iter()
                                    .all(|meaning| *meaning == SelectionMeaning::SubSection) {
                                    self.preview.ui_state.recreate_selectables = true;
                                    self.preview.ui_state.recreate_draggables = true;
                                    MaterializedReality::id() <<
                                    Simulate {
                                        requester: Self::id(),
                                        delta: self.preview.delta.clone(),
                                    };
                                } else {
                                    let current_nodes =
                                        meanings.iter()
                                            .filter(|meaning| {
                                                **meaning != SelectionMeaning::SubSection
                                            })
                                            .zip(selections.keys())
                                            .map(|(meaning, selection_ref)| {
                                                let stroke_idx = match *selection_ref {
                                                    SelectableStrokeRef::New(usize) => usize,
                                                    SelectableStrokeRef::RemainingOld(old_ref) => {
                                                        let old_stroke = self.preview
                                                            .current_remaining_old_strokes
                                                            .mapping
                                                            .get(old_ref)
                                                            .unwrap();
                                                        self.preview
                                                            .delta
                                                            .strokes_to_destroy
                                                            .insert(old_ref, old_stroke.clone());
                                                        self.preview
                                                            .delta
                                                            .new_strokes
                                                            .push(old_stroke.clone());
                                                        self.preview.delta.new_strokes.len() - 1
                                                    }
                                                };

                                                let node_idx = match *meaning {
                                                    SelectionMeaning::Start => 0,
                                                    SelectionMeaning::End => {
                                                        self.preview.delta.new_strokes[stroke_idx]
                                                            .nodes()
                                                            .len() -
                                                        1
                                                    }
                                                    _ => unreachable!(),
                                                };
                                                LaneStrokeNodeRef(stroke_idx, node_idx)
                                            })
                                            .collect();

                                    let previous_add = at;
                                    self.preview.ui_state.drawing_status =
                                        DrawingStatus::ContinuingFrom(current_nodes, previous_add);

                                    self.current = self.preview.clone();
                                    self.clear_selectables();
                                    self.clear_draggables();
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
struct Undo;

impl Recipient<Undo> for CurrentPlan {
    fn receive(&mut self, _msg: &Undo) -> Fate {
        if let Some(previous_state) = self.undo_history.pop() {
            self.redo_history.push(self.current.clone());
            self.preview = previous_state.clone();
            self.current = previous_state;
            self.clear_selectables();
            self.clear_draggables();

            match self.preview.ui_state.drawing_status {
                DrawingStatus::Nothing(()) => {
                    self.preview.ui_state.recreate_selectables = true;
                }
                DrawingStatus::WithSelections(_, _) => {
                    self.preview.ui_state.recreate_selectables = true;
                    self.preview.ui_state.recreate_draggables = true;
                }
                _ => {}
            }

            MaterializedReality::id() <<
            Simulate {
                requester: Self::id(),
                delta: self.preview.delta.clone(),
            };
        }
        Fate::Live
    }
}

#[derive(Copy, Clone)]
struct Redo;

impl Recipient<Redo> for CurrentPlan {
    fn receive(&mut self, _msg: &Redo) -> Fate {
        if let Some(next_state) = self.redo_history.pop() {
            self.undo_history.push(self.current.clone());
            self.preview = next_state.clone();
            self.current = next_state;
            self.clear_selectables();
            self.clear_draggables();

            match self.preview.ui_state.drawing_status {
                DrawingStatus::Nothing(()) => {
                    self.preview.ui_state.recreate_selectables = true;
                }
                DrawingStatus::WithSelections(_, _) => {
                    self.preview.ui_state.recreate_selectables = true;
                    self.preview.ui_state.recreate_draggables = true;
                }
                _ => {}
            }

            MaterializedReality::id() <<
            Simulate {
                requester: Self::id(),
                delta: self.preview.delta.clone(),
            };
        }
        Fate::Live
    }
}

#[derive(Copy, Clone)]
struct WithLatestNode(P2, bool);

impl Recipient<WithLatestNode> for CurrentPlan {
    fn receive(&mut self, msg: &WithLatestNode) -> Fate {
        match *msg {
            WithLatestNode(position, update_preview) => {
                let mut update_preview = update_preview;
                self.preview = self.current.clone();
                self.preview.ui_state.drawing_status = match self.current
                    .ui_state
                    .drawing_status
                    .clone() {
                    DrawingStatus::Nothing(_) => {
                        update_preview = false;
                        DrawingStatus::WithStartPoint(position)
                    }
                    DrawingStatus::WithStartPoint(start) => {
                        if (position - start).norm() < FINISH_STROKE_TOLERANCE {
                            DrawingStatus::Nothing(())
                        } else {
                            let new_node_refs = (0..self.preview.ui_state.n_lanes_per_side)
                                .into_iter()
                                .flat_map(|lane_idx| {
                                    let offset = (position - start).normalize().orthogonal() *
                                                 (3.0 + 5.0 * lane_idx as N);

                                    let maybe_right_stroke = LaneStroke::new(vec![LaneStrokeNode {
                                                                 position: start + offset,
                                                                 direction: (position - start)
                                                                     .normalize(),
                                                             },
                                                             LaneStrokeNode {
                                                                 position: position + offset,
                                                                 direction: (position - start)
                                                                     .normalize(),
                                                             }]
                                        .into());

                                    let maybe_right_lane_node_ref = maybe_right_stroke.ok()
                                        .map(|right_stroke| {
                                            self.preview.delta.new_strokes.push(right_stroke);
                                            LaneStrokeNodeRef(self.preview.delta.new_strokes.len() -
                                                              1,
                                                              1)
                                        });

                                    if self.preview.ui_state.create_both_sides {
                                        let maybe_left_stroke =
                                            LaneStroke::new(vec![LaneStrokeNode {
                                                                     position: position - offset,
                                                                     direction: (start - position)
                                                                         .normalize(),
                                                                 },
                                                                 LaneStrokeNode {
                                                                     position: start - offset,
                                                                     direction: (start - position)
                                                                         .normalize(),
                                                                 }]
                                                .into());

                                        let maybe_left_lane_node_ref = maybe_left_stroke.ok()
                                            .map(|left_stroke| {
                                                self.preview.delta.new_strokes.push(left_stroke);
                                                LaneStrokeNodeRef(self.preview
                                                                      .delta
                                                                      .new_strokes
                                                                      .len() -
                                                                  1,
                                                                  0)
                                            });

                                        maybe_right_lane_node_ref.into_iter()
                                            .chain(maybe_left_lane_node_ref)
                                            .collect::<Vec<_>>()
                                    } else {
                                        maybe_right_lane_node_ref.into_iter().collect::<Vec<_>>()
                                    }
                                });
                            DrawingStatus::ContinuingFrom(new_node_refs.collect(), position)
                        }
                    }
                    DrawingStatus::ContinuingFrom(current_nodes, previous_add) => {
                        if (position - previous_add).norm() < FINISH_STROKE_TOLERANCE {
                            DrawingStatus::Nothing(())
                        } else {
                            let new_current_nodes = current_nodes.clone()
                                .iter()
                                .map(|&LaneStrokeNodeRef(stroke_idx, node_idx)| {
                                    let stroke = &mut self.preview.delta.new_strokes[stroke_idx];
                                    let node = stroke.nodes()[node_idx];
                                    let relative_position_in_basis = (node.position - previous_add)
                                        .to_basis(node.direction);

                                    if node_idx == stroke.nodes().len() - 1 {
                                        // append
                                        let new_direction =
                                            Segment::arc_with_direction(previous_add,
                                                                        node.direction,
                                                                        position)
                                                .end_direction();
                                        stroke.nodes_mut().push(LaneStrokeNode{
                                            position: position
                                                      + relative_position_in_basis
                                                            .from_basis(new_direction),
                                            direction: new_direction
                                        });
                                        LaneStrokeNodeRef(stroke_idx, stroke.nodes().len() - 1)
                                    } else if node_idx == 0 {
                                        // prepend
                                        let new_direction =
                                            -Segment::arc_with_direction(previous_add,
                                                                         -node.direction,
                                                                         position)
                                                .end_direction();
                                        stroke.nodes_mut().insert(0, LaneStrokeNode{
                                            position: position
                                                      + relative_position_in_basis
                                                            .from_basis(new_direction),
                                            direction: new_direction
                                        });
                                        LaneStrokeNodeRef(stroke_idx, 0)
                                    } else {
                                        unreachable!()
                                    }
                                })
                                .collect();

                            let mut joined_some = false;
                            let mut new_strokes_to_remove = Vec::new();

                            for &LaneStrokeNodeRef(stroke_idx, node_idx) in &new_current_nodes {

                                let (maybe_join_with, is_end) = {
                                    let stroke = &self.preview.delta.new_strokes[stroke_idx];
                                    let is_end = stroke.nodes().len() - 1 == node_idx;
                                    let node = &stroke.nodes()[node_idx];

                                    let all_strokes = self.preview
                                        .delta
                                        .new_strokes
                                        .iter()
                                        .enumerate()
                                        .map(|(new_idx, new_stroke)| {
                                            (SelectableStrokeRef::New(new_idx), new_stroke)
                                        })
                                        .chain(self.preview
                                            .current_remaining_old_strokes
                                            .mapping
                                            .pairs()
                                            .map(|(old_ref, old_stroke)| {
                                                (SelectableStrokeRef::RemainingOld(*old_ref),
                                                 old_stroke)
                                            }));

                                    let maybe_join_with = all_strokes.map(|(stroke_ref, stroke)| {
                                            if is_end {
                                                let mut distance = (stroke.nodes()[0].position -
                                                                    node.position)
                                                    .norm();
                                                if !stroke.nodes()[0]
                                                    .direction
                                                    .is_roughly_within(node.direction, 0.5) {
                                                    // prevent unaligned connects
                                                    distance = ::std::f32::INFINITY
                                                }
                                                (stroke_ref, distance)
                                            } else {
                                                let mut distance =
                                                    (stroke.nodes().last().unwrap().position -
                                                     node.position)
                                                        .norm();
                                                if !stroke.nodes()
                                                    .last()
                                                    .unwrap()
                                                    .direction
                                                    .is_roughly_within(node.direction, 0.5) {
                                                    // prevent unaligned connects
                                                    distance = ::std::f32::INFINITY
                                                }
                                                (stroke_ref, distance)
                                            }
                                        })
                                        .min_by_key(|&(_, distance)| OrderedFloat(distance))
                                        .and_then(|(stroke_ref, distance)| if distance < 6.0 {
                                            Some(stroke_ref)
                                        } else {
                                            None
                                        });

                                    (maybe_join_with, is_end)
                                };

                                if let Some(join_with_ref) = maybe_join_with {
                                    joined_some = true;
                                    let mut self_join = false;

                                    let stroke = self.preview.delta.new_strokes[stroke_idx].clone();

                                    {
                                        let other_stroke = match join_with_ref {
                                            SelectableStrokeRef::New(other_stroke_idx) => {
                                                if stroke_idx == other_stroke_idx {
                                                    self_join = true;
                                                }
                                                &mut self.preview
                                                    .delta
                                                    .new_strokes
                                                         [other_stroke_idx]
                                            }
                                            SelectableStrokeRef::RemainingOld(old_ref) => {
                                                let old_stroke = self.preview
                                                    .current_remaining_old_strokes
                                                    .mapping
                                                    .get(old_ref)
                                                    .unwrap();
                                                self.preview
                                                    .delta
                                                    .strokes_to_destroy
                                                    .insert(old_ref, old_stroke.clone());
                                                self.preview
                                                    .delta
                                                    .new_strokes
                                                    .push(old_stroke.clone());
                                                self.preview.delta.new_strokes.last_mut().unwrap()
                                            }
                                        };

                                        let other_nodes = other_stroke.nodes().clone();

                                        *other_stroke.nodes_mut() = if self_join {
                                            let mut new_nodes = stroke.nodes().clone();
                                            if is_end {
                                                new_nodes.pop();
                                                new_nodes.push(other_nodes[0]);
                                            } else {
                                                new_nodes.remove(0);
                                                new_nodes.insert(0, *other_nodes.last().unwrap());
                                            }
                                            new_nodes
                                        } else if is_end {
                                            let mut new_nodes = stroke.nodes().clone();
                                            new_nodes.pop();
                                            new_nodes.extend(other_nodes.into_iter());
                                            new_nodes
                                        } else {
                                            let mut new_nodes = other_nodes;
                                            new_nodes.extend(
                                                stroke.nodes().clone().into_iter().skip(1));
                                            new_nodes
                                        }
                                    }

                                    if !self_join {
                                        new_strokes_to_remove.push(stroke_idx);
                                    }
                                }
                            }

                            if joined_some {
                                new_strokes_to_remove.sort();
                                for idx_to_remove in new_strokes_to_remove.into_iter().rev() {
                                    self.preview.delta.new_strokes.remove(idx_to_remove);
                                }
                                DrawingStatus::Nothing(())
                            } else {
                                DrawingStatus::ContinuingFrom(new_current_nodes, position)
                            }
                        }
                    }
                    DrawingStatus::WithSelections(selections, _) => {
                        DrawingStatus::WithSelections(selections, true)
                    }
                };
                if update_preview {
                    MaterializedReality::id() <<
                    Simulate {
                        requester: Self::id(),
                        delta: self.preview.delta.clone(),
                    };
                }
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
struct Select(SelectableStrokeRef, N, N);

impl Recipient<Select> for CurrentPlan {
    fn receive(&mut self, msg: &Select) -> Fate {
        match *msg {
            Select(selection_ref, start, end) => {
                if let DrawingStatus::WithSelections(ref mut selections,
                                                     ref mut remove_next_commit) =
                    self.preview.ui_state.drawing_status {
                    selections.insert(selection_ref, (start, end));
                    *remove_next_commit = false;
                } else {
                    self.preview.ui_state.drawing_status =
                        DrawingStatus::WithSelections(vec![(selection_ref, (start, end))]
                                                          .into_iter()
                                                          .collect(),
                                                      false);
                }

                if self.preview.ui_state.select_parallel {
                    let stroke = match selection_ref {
                        SelectableStrokeRef::New(node_idx) => {
                            &self.preview.delta.new_strokes[node_idx]
                        }
                        SelectableStrokeRef::RemainingOld(old_ref) => {
                            self.preview
                                .current_remaining_old_strokes
                                .mapping
                                .get(old_ref)
                                .expect(format!("old_ref {:?} should exist", old_ref).as_str())
                        }
                    };

                    let start_position = stroke.path().along(start);
                    let start_direction = stroke.path().direction_along(start);
                    let end_position = stroke.path().along(end);
                    let end_direction = stroke.path().direction_along(end);

                    let mut additional_selections = Vec::new();

                    let all_strokes =
                        self.preview
                            .delta
                            .new_strokes
                            .iter()
                            .enumerate()
                            .map(|(new_idx, new_stroke)| {
                                (SelectableStrokeRef::New(new_idx), new_stroke)
                            })
                            .chain(self.preview
                                .current_remaining_old_strokes
                                .mapping
                                .pairs()
                                .map(|(old_ref, old_stroke)| {
                                    (SelectableStrokeRef::RemainingOld(*old_ref), old_stroke)
                                }));

                    for (other_ref, other_stroke) in all_strokes {
                        if other_ref != selection_ref {
                            if let (Some(start_on_other_distance), Some(end_on_other_distance)) =
                                (other_stroke.path().project(start_position),
                                 other_stroke.path().project(end_position)) {
                                let start_on_other = other_stroke.path()
                                    .along(start_on_other_distance);
                                let start_direction_on_other = other_stroke.path()
                                    .direction_along(start_on_other_distance);
                                let end_on_other = other_stroke.path().along(end_on_other_distance);
                                let end_direction_on_other = other_stroke.path()
                                    .direction_along(end_on_other_distance);

                                let add_selection =
                                    start_on_other.is_roughly_within(start_position, 60.0) &&
                                    end_on_other.is_roughly_within(end_position, 60.0) &&
                                    if start_on_other_distance < end_on_other_distance {
                                        start_direction_on_other
                                            .is_roughly_within(start_direction, 0.1)
                                    && end_direction_on_other.is_roughly_within(end_direction, 0.1)
                                    } else if self.preview.ui_state.select_opposite {
                                        start_direction_on_other
                                            .is_roughly_within(-start_direction, 0.1)
                                    && end_direction_on_other.is_roughly_within(-end_direction, 0.1)
                                    } else {
                                        false
                                    };
                                if add_selection {
                                    additional_selections.push((other_ref, (
                                    start_on_other_distance.min(end_on_other_distance),
                                    end_on_other_distance.max(start_on_other_distance)
                                )));
                                }
                            }
                        }
                    }

                    if let DrawingStatus::WithSelections(ref mut selections, _) =
                        self.preview.ui_state.drawing_status {
                        for (other_ref, (start, end)) in additional_selections {
                            selections.insert(other_ref, (start, end));
                        }
                    } else {
                        unreachable!()
                    }
                }

                self.create_draggables();
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
struct MaximizeSelection;

impl Recipient<MaximizeSelection> for CurrentPlan {
    fn receive(&mut self, _msg: &MaximizeSelection) -> Fate {
        let new_selections = if let DrawingStatus::WithSelections(ref selections, _) =
            self.preview.ui_state.drawing_status {
            selections.pairs()
                .map(|(selection_ref, _)| {
                    let stroke = match *selection_ref {
                        SelectableStrokeRef::New(node_idx) => {
                            &self.preview.delta.new_strokes[node_idx]
                        }
                        SelectableStrokeRef::RemainingOld(old_ref) => {
                            self.preview.current_remaining_old_strokes.mapping.get(old_ref).unwrap()
                        }
                    };
                    (*selection_ref, (0.0, stroke.path().length()))
                })
                .collect()
        } else {
            unreachable!()
        };
        self.preview.ui_state.drawing_status = DrawingStatus::WithSelections(new_selections, false);
        Fate::Live
    }
}

#[derive(Copy, Clone)]
struct MoveSelection(V2);

impl Recipient<MoveSelection> for CurrentPlan {
    fn receive(&mut self, msg: &MoveSelection) -> Fate {
        match *msg {
            MoveSelection(delta) => {
                self.preview = self.current.clone();
                let new_selections = if let DrawingStatus::WithSelections(ref selections, _) =
                    self.preview.ui_state.drawing_status {
                    let mut with_subsections_moved = selections.pairs()
                        .map(|(&selection_ref, &(start, end))| {
                            let stroke = match selection_ref {
                                SelectableStrokeRef::New(node_idx) => {
                                    &self.preview.delta.new_strokes[node_idx]
                                }
                                SelectableStrokeRef::RemainingOld(old_ref) => {
                                    self.preview
                                        .current_remaining_old_strokes
                                        .mapping
                                        .get(old_ref)
                                        .unwrap()
                                }
                            };
                            (selection_ref, stroke.with_subsection_moved(start, end, delta))
                        })
                        .collect::<::fnv::FnvHashMap<_, _>>();

                    #[derive(PartialEq, Eq)]
                    enum C {
                        Before,
                        After,
                    };

                    let mut connector_alignments = Vec::<((SelectableStrokeRef, C),
                                                          (SelectableStrokeRef, C))>::new();

                    fn a_close_and_right_of_b(maybe_node_a: Option<&LaneStrokeNode>,
                                              maybe_node_b: Option<&LaneStrokeNode>)
                                              -> bool {
                        if let (Some(node_a), Some(node_b)) = (maybe_node_a, maybe_node_b) {
                            node_a.position.is_roughly_within(node_b.position, 7.0) &&
                            (node_a.position - node_b.position)
                                .dot(&node_a.direction.orthogonal()) >
                            0.0
                        } else {
                            false
                        }
                    }

                    for ((&ref_a,
                          &(_,
                            ref maybe_before_connector_a,
                            ref new_subsection_a,
                            ref maybe_after_connector_a,
                            _)),
                         (&ref_b,
                          &(_,
                            ref maybe_before_connector_b,
                            ref new_subsection_b,
                            ref maybe_after_connector_b,
                            _))) in
                        with_subsections_moved.iter()
                            .cartesian_product(with_subsections_moved.iter())
                            .filter(|&((a, _), (b, _))| a != b) {
                        if a_close_and_right_of_b(new_subsection_a.get(0),
                                                  new_subsection_b.get(0)) &&
                           maybe_before_connector_a.is_some() &&
                           maybe_before_connector_b.is_some() {
                            connector_alignments.push(((ref_a, C::Before), (ref_b, C::Before)));
                        }
                        if a_close_and_right_of_b(new_subsection_a.get(0),
                                                  new_subsection_b.last()) &&
                           maybe_before_connector_a.is_some() &&
                           maybe_after_connector_b.is_some() &&
                           !connector_alignments.iter()
                            .any(|other| other == &((ref_b, C::After), (ref_a, C::Before))) {
                            connector_alignments.push(((ref_a, C::Before), (ref_b, C::After)));
                        }
                        if a_close_and_right_of_b(new_subsection_a.last(),
                                                  new_subsection_b.last()) &&
                           maybe_after_connector_a.is_some() &&
                           maybe_after_connector_b.is_some() {
                            connector_alignments.push(((ref_a, C::After), (ref_b, C::After)));
                        }
                        if a_close_and_right_of_b(new_subsection_a.last(),
                                                  new_subsection_b.get(0)) &&
                           maybe_after_connector_a.is_some() &&
                           maybe_before_connector_b.is_some() &&
                           !connector_alignments.iter()
                            .any(|other| other == &((ref_b, C::Before), (ref_a, C::After))) {
                            connector_alignments.push(((ref_a, C::After), (ref_b, C::Before)));
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
                                    connector_alignments.iter()
                                        .position(|&(ref b, _)| align_a_to == b)
                                        .and_then(|b_idx| if b_idx > i {
                                            Some(b_idx)
                                        } else {
                                            None
                                        })
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

                    for ((align_ref, align_connector), (align_to_ref, align_to_connector)) in
                        connector_alignments {
                        let align_to = match align_to_connector {
                            C::Before => with_subsections_moved[&align_to_ref].1.unwrap(),
                            C::After => with_subsections_moved[&align_to_ref].3.unwrap(),
                        };
                        let align = match align_connector {
                            C::Before => {
                                with_subsections_moved.get_mut(&align_ref)
                                    .unwrap()
                                    .1
                                    .as_mut()
                                    .unwrap()
                            }
                            C::After => {
                                with_subsections_moved.get_mut(&align_ref)
                                    .unwrap()
                                    .3
                                    .as_mut()
                                    .unwrap()
                            }
                        };

                        let direction_sign = align.direction.dot(&align_to.direction).signum();
                        align.direction = direction_sign * align_to.direction;
                        let distance = if direction_sign < 0.0 { 6.0 } else { 5.0 };
                        align.position = align_to.position +
                                         distance * align.direction.orthogonal();
                    }

                    let mut new_selections = CDict::new();

                    for (selection_ref, (b, bc, s, ac, a)) in with_subsections_moved {
                        if let Ok(new_stroke) = LaneStroke::new(b.into_iter()
                                .chain(bc)
                                .chain(s.clone())
                                .chain(ac)
                                .chain(a)
                                .collect())
                            .map_err(|e| println!("{:?}", e)) {
                            let new_selection_start =
                                new_stroke.path().project(s[0].position).unwrap();
                            let new_selection_end =
                                new_stroke.path().project(s.last().unwrap().position).unwrap();

                            let new_selection_ref = match selection_ref {
                                SelectableStrokeRef::New(stroke_idx) => {
                                    self.preview.delta.new_strokes[stroke_idx] = new_stroke;
                                    SelectableStrokeRef::New(stroke_idx)
                                }
                                SelectableStrokeRef::RemainingOld(old_ref) => {
                                    let old_stroke = self.preview
                                        .current_remaining_old_strokes
                                        .mapping
                                        .get(old_ref)
                                        .unwrap();
                                    self.preview
                                        .delta
                                        .strokes_to_destroy
                                        .insert(old_ref, old_stroke.clone());
                                    self.preview.delta.new_strokes.push(new_stroke);
                                    SelectableStrokeRef::New(self.preview.delta.new_strokes.len() -
                                                             1)
                                }
                            };

                            new_selections.insert(new_selection_ref,
                                                  (new_selection_start, new_selection_end));
                        }
                    }

                    new_selections
                } else {
                    unreachable!()
                };
                self.preview.ui_state.drawing_status =
                    DrawingStatus::WithSelections(new_selections, false);
                self.preview.ui_state.dirty = true;
                MaterializedReality::id() <<
                Simulate {
                    requester: Self::id(),
                    delta: self.preview.delta.clone(),
                };
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
struct DeleteSelection;

impl Recipient<DeleteSelection> for CurrentPlan {
    fn receive(&mut self, _msg: &DeleteSelection) -> Fate {
        self.preview = self.current.clone();
        if let DrawingStatus::WithSelections(ref selections, _) =
            self.preview.ui_state.drawing_status {
            let mut new_stroke_indices_to_remove = Vec::new();
            let mut new_strokes = Vec::new();

            for (&selection_ref, &(start, end)) in selections.pairs() {
                let stroke = match selection_ref {
                    SelectableStrokeRef::New(node_idx) => {
                        new_stroke_indices_to_remove.push(node_idx);
                        &self.preview.delta.new_strokes[node_idx]
                    }
                    SelectableStrokeRef::RemainingOld(old_ref) => {
                        let old_stroke = self.preview
                            .current_remaining_old_strokes
                            .mapping
                            .get(old_ref)
                            .unwrap();
                        self.preview.delta.strokes_to_destroy.insert(old_ref, old_stroke.clone());
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

            new_stroke_indices_to_remove.sort();

            for index_to_remove in new_stroke_indices_to_remove.into_iter().rev() {
                self.preview.delta.new_strokes.remove(index_to_remove);
            }

            for new_stroke in new_strokes {
                self.preview.delta.new_strokes.push(new_stroke);
            }
        }
        self.preview.ui_state.drawing_status = DrawingStatus::Nothing(());
        self.clear_selectables();
        self.clear_draggables();
        Self::id() << Commit(true, P2::new(0.0, 0.0));
        Fate::Live
    }
}

#[derive(Compact, Clone)]
struct AddStroke {
    stroke: LaneStroke,
}

impl Recipient<AddStroke> for CurrentPlan {
    fn receive(&mut self, msg: &AddStroke) -> Fate {
        match *msg {
            AddStroke { ref stroke } => {
                self.preview = self.current.clone();
                self.preview.delta.new_strokes.push(stroke.clone());
                MaterializedReality::id() <<
                Simulate {
                    requester: Self::id(),
                    delta: self.preview.delta.clone(),
                };
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
struct CreateGrid(usize);

impl Recipient<CreateGrid> for CurrentPlan {
    fn receive(&mut self, msg: &CreateGrid) -> Fate {
        match *msg {
            CreateGrid(gridsize) => {
                let grid_size = gridsize;
                let grid_spacing = 1000.0;

                for x in 0..grid_size {
                    self.receive(&WithLatestNode(P2::new((x as f32 + 0.5) * grid_spacing, 0.0),
                                                 false));
                    self.receive(&Commit(false, P2::new(0.0, 0.0)));
                    self.receive(&WithLatestNode(P2::new((x as f32 + 0.5) * grid_spacing,
                                                         grid_size as f32 * grid_spacing),
                                                 false));
                    self.receive(&Commit(false, P2::new(0.0, 0.0)));
                    self.receive(&WithLatestNode(P2::new((x as f32 + 0.5) * grid_spacing,
                                                         grid_size as f32 * grid_spacing),
                                                 false));
                    self.receive(&Commit(false, P2::new(0.0, 0.0)));
                }
                for y in 0..grid_size {
                    self.receive(&WithLatestNode(P2::new(0.0, (y as f32 + 0.5) * grid_spacing),
                                                 false));
                    self.receive(&Commit(false, P2::new(0.0, 0.0)));
                    self.receive(&WithLatestNode(P2::new(grid_size as f32 * grid_spacing,
                                                         (y as f32 + 0.5) * grid_spacing),
                                                 false));
                    self.receive(&Commit(false, P2::new(0.0, 0.0)));
                    self.receive(&WithLatestNode(P2::new(grid_size as f32 * grid_spacing,
                                                         (y as f32 + 0.5) * grid_spacing),
                                                 false));
                    self.receive(&Commit(false, P2::new(0.0, 0.0)));
                }
                self.receive(&Commit(true, P2::new(0.0, 0.0)));
                MaterializedReality::id() <<
                Simulate {
                    requester: Self::id(),
                    delta: self.preview.delta.clone(),
                };
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
struct Materialize;

impl Recipient<Materialize> for CurrentPlan {
    fn receive(&mut self, _msg: &Materialize) -> Fate {
        MaterializedReality::id() <<
        Apply {
            requester: Self::id(),
            delta: self.current.delta.clone(),
        };
        *self = CurrentPlan::default();
        self.clear_selectables();
        self.clear_draggables();
        self.preview.ui_state.recreate_selectables = true;
        Fate::Live
    }
}

#[derive(Copy, Clone)]
struct SetSelectionMode(bool, bool);

impl Recipient<SetSelectionMode> for CurrentPlan {
    fn receive(&mut self, msg: &SetSelectionMode) -> Fate {
        match *msg {
            SetSelectionMode(select_parallel, select_opposite) => {
                self.preview.ui_state.select_parallel = select_parallel;
                self.preview.ui_state.select_opposite = select_opposite;
                self.current.ui_state.select_parallel = select_parallel;
                self.current.ui_state.select_opposite = select_opposite;
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
struct SetNLanes(usize);

impl Recipient<SetNLanes> for CurrentPlan {
    fn receive(&mut self, msg: &SetNLanes) -> Fate {
        match *msg {
            SetNLanes(n_lanes) => {
                self.preview.ui_state.n_lanes_per_side = n_lanes;
                self.current.ui_state.n_lanes_per_side = n_lanes;
                let at = match self.preview.ui_state.drawing_status {
                    DrawingStatus::ContinuingFrom(_, last_add) => last_add,
                    _ => P2::new(0.0, 0.0),
                };
                Self::id() << WithLatestNode(at, true);
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
struct ToggleBothSides;

impl Recipient<ToggleBothSides> for CurrentPlan {
    fn receive(&mut self, _msg: &ToggleBothSides) -> Fate {
        self.preview.ui_state.create_both_sides = !self.preview.ui_state.create_both_sides;
        self.current.ui_state.create_both_sides = self.preview.ui_state.create_both_sides;
        let at = match self.preview.ui_state.drawing_status {
            DrawingStatus::ContinuingFrom(_, last_add) => last_add,
            _ => P2::new(0.0, 0.0),
        };
        Self::id() << WithLatestNode(at, true);
        Fate::Live
    }
}

use self::materialized_reality::SimulationResult;

impl Recipient<SimulationResult> for CurrentPlan {
    fn receive(&mut self, msg: &SimulationResult) -> Fate {
        match *msg {
            SimulationResult { ref remaining_old_strokes, ref result_delta } => {
                self.preview.current_remaining_old_strokes = remaining_old_strokes.clone();
                // TODO: this is not really a nice solution
                if self.current.current_remaining_old_strokes.mapping.is_empty() {
                    self.current.current_remaining_old_strokes = remaining_old_strokes.clone();
                }
                self.preview.current_plan_result_delta = result_delta.clone();
                self.preview.ui_state.dirty = true;
                if self.preview.ui_state.recreate_selectables {
                    self.preview.ui_state.recreate_selectables = false;
                    self.create_selectables();
                }
                if self.preview.ui_state.recreate_draggables {
                    self.preview.ui_state.recreate_draggables = false;
                    self.create_draggables();
                    self.create_addables();
                }
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum SelectableStrokeRef {
    New(usize),
    RemainingOld(LaneStrokeRef),
}

impl CurrentPlan {
    fn create_selectables(&mut self) {
        for (stroke_idx, stroke) in self.preview.delta.new_strokes.iter().enumerate() {
            Swarm::<LaneStrokeSelectable>::all() <<
            CreateWith(LaneStrokeSelectable::new(SelectableStrokeRef::New(stroke_idx),
                                                 stroke.path().clone()),
                       AddToUI);
        }
        for (old_ref, stroke) in self.preview.current_remaining_old_strokes.mapping.pairs() {
            Swarm::<LaneStrokeSelectable>::all() <<
            CreateWith(LaneStrokeSelectable::new(SelectableStrokeRef::RemainingOld(*old_ref),
                                                 stroke.path().clone()),
                       AddToUI);
        }
    }

    fn create_draggables(&mut self) {
        if let DrawingStatus::WithSelections(ref selections, _) =
            self.preview.ui_state.drawing_status {
            for (&selection_ref, &(start, end)) in selections.pairs() {
                let stroke = match selection_ref {
                    SelectableStrokeRef::New(stroke_idx) => {
                        &self.preview.delta.new_strokes[stroke_idx]
                    }
                    SelectableStrokeRef::RemainingOld(old_stroke_ref) => {
                        self.preview
                            .current_remaining_old_strokes
                            .mapping
                            .get(old_stroke_ref)
                            .unwrap()
                    }
                };
                if let Some(subsection) = stroke.path().subsection(start, end) {
                    Swarm::<LaneStrokeDraggable>::all() <<
                    CreateWith(LaneStrokeDraggable::new(selection_ref, subsection), AddToUI);
                }
            }
        }
    }

    fn create_addables(&mut self) {
        if let DrawingStatus::WithSelections(ref selections, _) =
            self.preview.ui_state.drawing_status {
            for (&selection_ref, &(start, end)) in selections.pairs() {
                let stroke = match selection_ref {
                    SelectableStrokeRef::New(stroke_idx) => {
                        &self.preview.delta.new_strokes[stroke_idx]
                    }
                    SelectableStrokeRef::RemainingOld(old_stroke_ref) => {
                        self.preview
                            .current_remaining_old_strokes
                            .mapping
                            .get(old_stroke_ref)
                            .unwrap()
                    }
                };
                let start_position = stroke.path().along(start);
                let start_direction = stroke.path().direction_along(start);
                let end_position = stroke.path().along(end);
                let end_direction = stroke.path().direction_along(end);

                let is_right_of_stroke =
                    |other_stroke: &LaneStroke| if let Some(start_on_other_distance) =
                        other_stroke.path().project(start_position) {
                        let start_on_other = other_stroke.path().along(start_on_other_distance);
                        start_on_other.is_roughly_within(start_position, 6.0) &&
                        (start_on_other - start_position).dot(&start_direction.orthogonal()) > 0.0
                    } else if let Some(end_on_other_distance) =
                        other_stroke.path().project(end_position) {
                        let end_on_other = other_stroke.path().along(end_on_other_distance);
                        end_on_other.is_roughly_within(end_position, 6.0) &&
                        (end_on_other - end_position).dot(&end_direction.orthogonal()) > 0.0
                    } else {
                        false
                    };

                let mut all_strokes = self.preview
                    .delta
                    .new_strokes
                    .iter()
                    .chain(self.preview.current_remaining_old_strokes.mapping.values());

                if !all_strokes.any(is_right_of_stroke) {
                    if let Some(shifted_stroke) =
                        stroke.subsection(start, end).and_then(|subsection| {
                            LaneStroke::new(subsection.nodes()
                                    .iter()
                                    .map(|node| {
                                        LaneStrokeNode {
                                            position: node.position +
                                                      5.0 * node.direction.orthogonal(),
                                            direction: node.direction,
                                        }
                                    })
                                    .collect())
                                .ok()
                        }) {
                        Swarm::<LaneStrokeAddable>::all() <<
                        CreateWith(LaneStrokeAddable::new(shifted_stroke), AddToUI);
                    }
                }
            }
        }
    }

    fn clear_selectables(&mut self) {
        Swarm::<LaneStrokeSelectable>::all() << ClearSelectables;
    }

    fn clear_draggables(&mut self) {
        Swarm::<LaneStrokeDraggable>::all() << ClearDraggables;
        Swarm::<LaneStrokeAddable>::all() << ClearDraggables;
    }
}

#[derive(Compact, Clone)]
pub enum DrawingStatus {
    Nothing(()),
    WithStartPoint(P2),
    ContinuingFrom(CVec<LaneStrokeNodeRef>, P2),
    WithSelections(CDict<SelectableStrokeRef, (N, N)>, bool),
}

impl ::std::fmt::Debug for DrawingStatus {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            DrawingStatus::Nothing(..) => write!(f, "Nothing"),
            DrawingStatus::WithStartPoint(..) => write!(f, "WithStartPoint"),
            DrawingStatus::ContinuingFrom(..) => write!(f, "ContinuingFrom"),
            DrawingStatus::WithSelections(..) => write!(f, "WithSelections"),
        }
    }
}

#[derive(Compact, Clone)]
struct PlanUIState {
    create_both_sides: bool,
    n_lanes_per_side: usize,
    select_parallel: bool,
    select_opposite: bool,
    drawing_status: DrawingStatus,
    dirty: bool,
    recreate_selectables: bool,
    recreate_draggables: bool,
}

impl Default for PlanUIState {
    fn default() -> PlanUIState {
        PlanUIState {
            create_both_sides: true,
            n_lanes_per_side: 2,
            select_parallel: true,
            select_opposite: true,
            drawing_status: DrawingStatus::Nothing(()),
            dirty: true,
            recreate_selectables: false,
            recreate_draggables: false,
        }
    }
}

#[derive(Copy, Clone)]
struct AddToUI;

#[derive(Copy, Clone)]
struct ClearSelectables;

#[derive(Copy, Clone)]
struct ClearDraggables;

#[derive(Copy, Clone)]
struct ClearAddables;

pub fn setup(system: &mut ActorSystem) {
    system.add_actor(CurrentPlan::default());
    CurrentPlan::handle::<Commit>();
    CurrentPlan::handle::<Undo>();
    CurrentPlan::handle::<Redo>();
    CurrentPlan::handle::<WithLatestNode>();
    CurrentPlan::handle::<Select>();
    CurrentPlan::handle::<MaximizeSelection>();
    CurrentPlan::handle::<MoveSelection>();
    CurrentPlan::handle::<DeleteSelection>();
    CurrentPlan::handle::<AddStroke>();
    CurrentPlan::handle::<CreateGrid>();
    CurrentPlan::handle::<Materialize>();
    CurrentPlan::handle::<SetSelectionMode>();
    CurrentPlan::handle::<SetNLanes>();
    CurrentPlan::handle::<ToggleBothSides>();
    CurrentPlan::handle::<SimulationResult>();
    self::materialized_reality::setup(system);
    self::lane_stroke_canvas::setup(system);
    self::lane_stroke_selectable::setup(system);
    self::lane_stroke_draggable::setup(system);
    self::lane_stroke_addable::setup(system);
}
