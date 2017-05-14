use descartes::{N, RoughlyComparable};
use compact::{CVec, CDict};
use stagemaster::geometry::CPath;
use itertools::Itertools;
use super::lane_stroke::{LaneStroke, LaneStrokeNode};

#[derive(Clone, Compact)]
pub struct Plan {
    pub strokes: CVec<LaneStroke>,
}

impl Default for Plan {
    fn default() -> Plan {
        Plan { strokes: CVec::new() }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LaneStrokeRef(pub usize);

#[derive(Compact, Clone)]
pub struct PlanDelta {
    pub new_strokes: CVec<LaneStroke>,
    pub strokes_to_destroy: CDict<LaneStrokeRef, LaneStroke>,
}

impl Default for PlanDelta {
    fn default() -> PlanDelta {
        PlanDelta {
            new_strokes: CVec::new(),
            strokes_to_destroy: CDict::new(),
        }
    }
}

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
        self.incoming
            .values()
            .all(|self_incoming| {
                other
                    .incoming
                    .values()
                    .any(|other_incoming| {
                        self_incoming.is_roughly_within(other_incoming, tolerance)
                    })
            }) && self.outgoing.len() == other.outgoing.len() &&
        self.outgoing
            .values()
            .all(|self_outgoing| {
                other
                    .outgoing
                    .values()
                    .any(|other_outgoing| {
                        self_outgoing.is_roughly_within(other_outgoing, tolerance)
                    })
            }) && self.strokes.len() == other.strokes.len() &&
        self.strokes
            .iter()
            .all(|self_stroke| {
                other
                    .strokes
                    .iter()
                    .any(|other_stroke| self_stroke.is_roughly_within(other_stroke, tolerance))
            })
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct IntersectionRef(pub usize);

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct TrimmedStrokeRef(pub usize);

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct TransferStrokeRef(pub usize);

#[derive(Compact, Clone, Default)]
pub struct PlanResult {
    pub intersections: CDict<IntersectionRef, Intersection>,
    pub trimmed_strokes: CDict<TrimmedStrokeRef, LaneStroke>,
    pub transfer_strokes: CDict<TransferStrokeRef, LaneStroke>,
}

const RESULT_DELTA_TOLERANCE: N = 0.1;

impl PlanResult {
    pub fn delta(&self, old: &Self) -> PlanResultDelta {
        PlanResultDelta {
            intersections: ReferencedDelta::compare_roughly(&self.intersections,
                                                            &old.intersections,
                                                            RESULT_DELTA_TOLERANCE),
            trimmed_strokes: ReferencedDelta::compare_roughly(&self.trimmed_strokes,
                                                              &old.trimmed_strokes,
                                                              RESULT_DELTA_TOLERANCE),
            transfer_strokes: ReferencedDelta::compare_roughly(&self.transfer_strokes,
                                                               &old.transfer_strokes,
                                                               RESULT_DELTA_TOLERANCE),
        }
    }
}

#[derive(Compact, Clone)]
pub struct PlanResultDelta {
    pub intersections: ReferencedDelta<IntersectionRef, Intersection>,
    pub trimmed_strokes: ReferencedDelta<TrimmedStrokeRef, LaneStroke>,
    pub transfer_strokes: ReferencedDelta<TransferStrokeRef, LaneStroke>,
}

#[derive(Compact, Clone)]
pub struct ReferencedDelta<Ref: Copy + Eq, T: ::compact::Compact + Clone> {
    pub to_create: CDict<Ref, T>,
    pub to_destroy: CDict<Ref, T>,
    pub old_to_new: CDict<Ref, Ref>,
}

impl<Ref: Copy + Eq, T: ::compact::Compact + Clone> ReferencedDelta<Ref, T> {
    pub fn compare<'a, F: Fn(&'a T, &'a T) -> bool>(new: &'a CDict<Ref, T>,
                                                    old: &'a CDict<Ref, T>,
                                                    equivalent: F)
                                                    -> Self {
        let pairs = new.pairs().cartesian_product(old.pairs());
        let old_to_new = pairs
            .filter_map(|pair| match pair {
                            ((new_ref, new), (old_ref, old)) => {
                                if equivalent(new, old) {
                                    Some((*old_ref, *new_ref))
                                } else {
                                    None
                                }
                            }
                        })
            .collect::<CDict<_, _>>();

        let to_create = new.pairs()
            .filter_map(|(new_ref, new)| if
                old_to_new
                    .values()
                    .any(|not_really_new_ref| not_really_new_ref == new_ref) {
                            None
                        } else {
                            Some((*new_ref, new.clone()))
                        })
            .collect();

        let to_destroy = old.pairs()
            .filter_map(|(old_ref, old)| if
                old_to_new
                    .keys()
                    .any(|revived_old_ref| revived_old_ref == old_ref) {
                            None
                        } else {
                            Some((*old_ref, old.clone()))
                        })
            .collect();

        ReferencedDelta {
            to_create: to_create,
            to_destroy: to_destroy,
            old_to_new: old_to_new,
        }
    }

    pub fn compare_roughly<'a>(new: &'a CDict<Ref, T>, old: &'a CDict<Ref, T>, tolerance: N) -> Self
        where &'a T: RoughlyComparable + 'a
    {
        Self::compare(new,
                      old,
                      |new_item, old_item| new_item.is_roughly_within(old_item, tolerance))
    }
}

impl<Ref: Copy + Eq, T: ::compact::Compact + Clone> Default for ReferencedDelta<Ref, T> {
    fn default() -> Self {
        ReferencedDelta {
            to_create: CDict::new(),
            to_destroy: CDict::new(),
            old_to_new: CDict::new(),
        }
    }
}

impl Default for PlanResultDelta {
    fn default() -> PlanResultDelta {
        PlanResultDelta {
            intersections: ReferencedDelta::default(),
            trimmed_strokes: ReferencedDelta::default(),
            transfer_strokes: ReferencedDelta::default(),
        }
    }
}

#[derive(Compact, Clone)]
pub struct BuiltStrokes {
    pub mapping: CDict<LaneStrokeRef, LaneStroke>,
}

impl Default for BuiltStrokes {
    fn default() -> Self {
        BuiltStrokes { mapping: CDict::new() }
    }
}

use super::plan_result_steps::{find_intersections, trim_strokes_and_add_incoming_outgoing,
                               create_connecting_strokes, find_transfer_strokes,
                               determine_signal_timings};

impl Plan {
    pub fn with_delta(&self, delta: &PlanDelta) -> (Plan, BuiltStrokes) {
        let built_old_refs_and_strokes = self.strokes
            .iter()
            .enumerate()
            .filter_map(|(i, stroke)| if delta.strokes_to_destroy.contains_key(LaneStrokeRef(i)) {
                            None
                        } else {
                            Some((LaneStrokeRef(i), stroke.clone()))
                        })
            .collect::<CDict<_, _>>();
        let new_plan = Plan {
            strokes: built_old_refs_and_strokes
                .values()
                .chain(delta.new_strokes.iter())
                .cloned()
                .collect(),
        };
        (new_plan, BuiltStrokes { mapping: built_old_refs_and_strokes })
    }

    pub fn get_result(&self) -> PlanResult {
        let mut intersections = find_intersections(&self.strokes);
        let trimmed_strokes = trim_strokes_and_add_incoming_outgoing(&self.strokes,
                                                                     &mut intersections);
        create_connecting_strokes(&mut intersections);
        let transfer_strokes = find_transfer_strokes(&trimmed_strokes);
        determine_signal_timings(&mut intersections);

        PlanResult {
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
                .map(|(i, transfer_stroke)| (TransferStrokeRef(i), transfer_stroke))
                .collect(),
        }
    }
}
