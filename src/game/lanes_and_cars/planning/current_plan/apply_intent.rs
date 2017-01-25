use compact::CVec;
use descartes::{N, P2, Segment, Norm, FiniteCurve, RelativeToBasis, WithUniqueOrthogonal,
                RoughlyComparable};

use super::{PlanStep, Settings, Intent, LaneStrokeRef, ContinuationMode};
use super::super::plan::PlanDelta;
use super::super::lane_stroke::{LaneStroke, LaneStrokeNode};

const LANE_DISTANCE: N = 5.0;
const CENTER_LANE_DISTANCE: N = 6.0;

pub fn apply_intent(current: &PlanStep, settings: &Settings) -> PlanStep {
    match current.intent {
        Intent::None => current.clone(),

        Intent::NewRoad(ref points) => {
            // drawing a new road is equivalent to continuing a road
            // that consists of only its start points
            let mut one_point_strokes = CVec::<LaneStroke>::new();
            let mut continue_from = CVec::new();
            let base_idx = current.plan_delta.new_strokes.len();
            let direction = (points[1] - points[0]).normalize();
            let n_per_side = settings.n_lanes_per_side;
            let offset = |lane_idx: usize| {
                direction.orthogonal() *
                (CENTER_LANE_DISTANCE / 2.0 + LANE_DISTANCE * lane_idx as N)
            };

            for lane_idx in 0..n_per_side {
                one_point_strokes.push(LaneStroke::with_single_node(LaneStrokeNode {
                    position: points[0] + offset(lane_idx),
                    direction: direction,
                }));
                continue_from.push((LaneStrokeRef(base_idx + lane_idx), ContinuationMode::Append));
            }

            if settings.create_both_sides {
                for lane_idx in 0..n_per_side {
                    one_point_strokes.push(LaneStroke::with_single_node(LaneStrokeNode {
                        position: points[0] - offset(lane_idx),
                        direction: -direction,
                    }));
                    continue_from.push((LaneStrokeRef(base_idx + lane_idx + n_per_side),
                                        ContinuationMode::Prepend));
                }
            }

            let mut new_new_strokes = current.plan_delta.new_strokes.clone();
            new_new_strokes.extend(one_point_strokes);

            let equivalent_current = PlanStep {
                plan_delta: PlanDelta {
                    new_strokes: new_new_strokes,
                    ..current.plan_delta.clone()
                },
                intent: Intent::ContinueRoad(continue_from,
                                             points.iter().cloned().skip(1).collect(),
                                             points[0]),
                ..current.clone()
            };

            apply_intent(&equivalent_current, settings)
        }

        Intent::ContinueRoad(ref continue_from, ref additional_points, start_reference_point) => {
            let mut previous_reference_point = start_reference_point;
            let mut new_plan_delta = current.plan_delta.clone();

            for next_reference_point in additional_points {
                // TODO: not really nice that we have to care about that here...
                if next_reference_point.is_roughly_within(previous_reference_point, ::descartes::MIN_START_TO_END) {
                    continue;
                }

                for &(LaneStrokeRef(stroke_idx), mode) in continue_from {
                    let stroke = &mut new_plan_delta.new_strokes[stroke_idx];
                    let (previous_position, previous_direction, next_direction) = match mode {
                        ContinuationMode::Append => {
                            let node = stroke.nodes().last().unwrap();
                            (node.position,
                             node.direction,
                             Segment::arc_with_direction(previous_reference_point,
                                                         node.direction,
                                                         *next_reference_point)
                                 .end_direction())

                        }
                        ContinuationMode::Prepend => {
                            let node = stroke.nodes()[0];
                            (node.position,
                             node.direction,
                             -Segment::arc_with_direction(previous_reference_point,
                                                          -node.direction,
                                                          *next_reference_point)
                                 .end_direction())
                        }
                    };
                    let next_position = *next_reference_point +
                                        (previous_position - previous_reference_point)
                        .to_basis(previous_direction)
                        .from_basis(next_direction);

                    let next_node = LaneStrokeNode {
                        position: next_position,
                        direction: next_direction,
                    };

                    match mode {
                        ContinuationMode::Append => {
                            stroke.nodes_mut().push(next_node);
                            if !stroke.well_formed() {
                                stroke.nodes_mut().pop();
                            }
                        }
                        ContinuationMode::Prepend => {
                            stroke.nodes_mut().insert(0, next_node);
                            if !stroke.well_formed() {
                                stroke.nodes_mut().remove(0);
                            }
                        }
                    }
                }

                previous_reference_point = *next_reference_point;
            }

            PlanStep {
                plan_delta: new_plan_delta,
                selections: CVec::new(),
                intent: Intent::None,
            }
        }
    }
}