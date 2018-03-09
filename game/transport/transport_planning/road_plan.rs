use compact::{CVec, CDict};
use descartes::{N, RoughlyComparable};
use stagemaster::geometry::CPath;
use super::lane_stroke::{LaneStroke, LaneStrokeNode};
use planning::plan::ReferencedDelta;

#[derive(Compact, Clone, Default)]
pub struct RoadPlan {
    pub strokes: CVec<LaneStroke>,
}

#[derive(Compact, Clone, Default)]
pub struct RoadPlanDelta {
    pub new_strokes: CVec<LaneStroke>,
    pub strokes_to_destroy: CDict<LaneStrokeRef, LaneStroke>,
}

#[derive(Compact, Clone, Default)]
pub struct RoadPlanResult {
    pub intersections: CDict<IntersectionRef, Intersection>,
    pub trimmed_strokes: CDict<TrimmedStrokeRef, LaneStroke>,
    pub transfer_strokes: CDict<TransferStrokeRef, LaneStroke>,
}

#[derive(Compact, Clone, Default)]
pub struct RoadPlanResultDelta {
    pub intersections: ReferencedDelta<IntersectionRef, Intersection>,
    pub trimmed_strokes: ReferencedDelta<TrimmedStrokeRef, LaneStroke>,
    pub transfer_strokes: ReferencedDelta<TransferStrokeRef, LaneStroke>,
}

impl RoadPlanResult {
    pub fn delta(&self, old: &Self) -> RoadPlanResultDelta {
        RoadPlanResultDelta {
            intersections: ReferencedDelta::compare_roughly(
                &self.intersections,
                &old.intersections,
                RESULT_DELTA_TOLERANCE,
            ),
            trimmed_strokes: ReferencedDelta::compare_roughly(
                &self.trimmed_strokes,
                &old.trimmed_strokes,
                RESULT_DELTA_TOLERANCE,
            ),
            transfer_strokes: ReferencedDelta::compare_roughly(
                &self.transfer_strokes,
                &old.transfer_strokes,
                RESULT_DELTA_TOLERANCE,
            ),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LaneStrokeRef(pub usize);

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct IntersectionRef(pub usize);

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct TrimmedStrokeRef(pub usize);

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct TransferStrokeRef(pub usize);

#[derive(Compact, Clone)]
pub struct Intersection {
    pub shape: CPath,
    pub incoming: CDict<LaneStrokeRef, LaneStrokeNode>,
    pub outgoing: CDict<LaneStrokeRef, LaneStrokeNode>,
    pub strokes: CVec<LaneStroke>,
    pub timings: CVec<CVec<bool>>,
}

impl<'a> RoughlyComparable for &'a Intersection {
    fn is_roughly_within(&self, other: &Intersection, tolerance: N) -> bool {
        (&self.shape).is_roughly_within(&other.shape, tolerance) &&
            self.incoming.len() == other.incoming.len() &&
            self.incoming.values().all(|self_incoming| {
                other.incoming.values().any(|other_incoming| {
                    self_incoming.is_roughly_within(other_incoming, tolerance)
                })
            }) && self.outgoing.len() == other.outgoing.len() &&
            self.outgoing.values().all(|self_outgoing| {
                other.outgoing.values().any(|other_outgoing| {
                    self_outgoing.is_roughly_within(other_outgoing, tolerance)
                })
            }) && self.strokes.len() == other.strokes.len() &&
            self.strokes.iter().all(|self_stroke| {
                other.strokes.iter().any(|other_stroke| {
                    self_stroke.is_roughly_within(other_stroke, tolerance)
                })
            })
    }
}

const RESULT_DELTA_TOLERANCE: N = 0.1;

use super::road_result_steps::{find_intersections, trim_strokes_and_add_incoming_outgoing,
                               create_connecting_strokes, find_transfer_strokes,
                               determine_signal_timings};

impl RoadPlan {
    pub fn with_delta(&self, delta: &RoadPlanDelta) -> Self {
        let built_old_refs_and_strokes = self.strokes
            .iter()
            .enumerate()
            .filter_map(|(i, stroke)| if delta.strokes_to_destroy.contains_key(
                LaneStrokeRef(i),
            )
            {
                None
            } else {
                Some((LaneStrokeRef(i), stroke.clone()))
            })
            .collect::<CDict<_, _>>();
        RoadPlan {
            strokes: built_old_refs_and_strokes
                .values()
                .chain(delta.new_strokes.iter())
                .cloned()
                .collect(),
        }
        //(
        //BuiltStrokes { mapping: built_old_refs_and_strokes },
        //)
    }

    pub fn get_result(&self) -> RoadPlanResult {
        let mut intersections = find_intersections(&self.strokes);
        let trimmed_strokes =
            trim_strokes_and_add_incoming_outgoing(&self.strokes, &mut intersections);
        create_connecting_strokes(&mut intersections);
        let transfer_strokes = find_transfer_strokes(&trimmed_strokes);
        determine_signal_timings(&mut intersections);

        RoadPlanResult {
            intersections: intersections
                .into_iter()
                .enumerate()
                .map(|(i, intersection)| (IntersectionRef(i), intersection))
                .collect(),
            trimmed_strokes: trimmed_strokes
                .into_iter()
                .enumerate()
                .map(|(i, stroke)| (TrimmedStrokeRef(i), stroke))
                .collect(),
            transfer_strokes: transfer_strokes
                .into_iter()
                .enumerate()
                .map(|(i, transfer_stroke)| {
                    (TransferStrokeRef(i), transfer_stroke)
                })
                .collect(),
        }
    }
}
