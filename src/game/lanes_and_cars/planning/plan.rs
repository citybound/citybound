use descartes::{N, Path, Norm, Band, Intersect, convex_hull, Curve, FiniteCurve, RoughlyComparable};
use kay::{CVec, CDict};
use core::geometry::{CPath};
use ordered_float::OrderedFloat;
use itertools::{Itertools};
use super::{RoadStroke, RoadStrokeNode};

#[derive(Clone, Compact)]
pub struct Plan{
    strokes: CVec<RoadStroke>
}

impl Default for Plan{
    fn default() -> Plan {Plan{strokes: CVec::new()}}
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct RoadStrokeRef(pub usize);

#[derive(Compact, Clone)]
pub struct PlanDelta{
    pub new_strokes: CVec<RoadStroke>,
    pub strokes_to_destroy: CDict<RoadStrokeRef, RoadStroke>
}

impl Default for PlanDelta{
    fn default() -> PlanDelta{PlanDelta{
        new_strokes: CVec::new(), strokes_to_destroy: CDict::new()
    }}
}

#[derive(Compact, Clone)]
pub struct Intersection{
    pub shape: CPath,
    incoming: CVec<RoadStrokeNode>,
    outgoing: CVec<RoadStrokeNode>,
    pub strokes: CVec<RoadStroke>
}

impl<'a> RoughlyComparable for &'a Intersection{
    fn is_roughly_within(&self, other: &Intersection, tolerance: N) -> bool {
        (&self.shape).is_roughly_within(&other.shape, tolerance) &&
        self.incoming.len() == other.incoming.len() &&
            self.incoming.iter().all(|self_incoming| other.incoming.iter().any(|other_incoming|
                self_incoming.is_roughly_within(other_incoming, tolerance)
            )) &&
        self.outgoing.len() == other.outgoing.len() &&
            self.outgoing.iter().all(|self_outgoing| other.outgoing.iter().any(|other_outgoing|
                self_outgoing.is_roughly_within(other_outgoing, tolerance)
            )) &&
        self.strokes.len() == other.strokes.len() &&
            self.strokes.iter().all(|self_stroke| other.strokes.iter().any(|other_stroke|
                self_stroke.is_roughly_within(other_stroke, tolerance)
            ))
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct IntersectionRef(pub usize);

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct InbetweenStrokeRef(pub usize);

#[derive(Compact, Clone, Default)]
pub struct PlanResult{
    pub intersections: CVec<Intersection>,
    pub inbetween_strokes: CVec<RoadStroke>
}

const RESULT_DELTA_TOLERANCE : N = 0.1;

impl PlanResult{
    pub fn delta(&self, old: &Self) -> PlanResultDelta{
        let intersection_pairs = self.intersections.iter().enumerate().cartesian_product(old.intersections.iter().enumerate());
        let old_to_new_intersection_map = intersection_pairs.filter_map(|pair| match pair {
            ((new_i, new), (old_i, old)) => if (&new).is_roughly_within(old, RESULT_DELTA_TOLERANCE) {
                Some((IntersectionRef(old_i), IntersectionRef(new_i)))
            } else {None}
        }).collect::<CDict<_, _>>();

        let new_intersections = self.intersections.iter().enumerate().filter_map(|(new_i, new)|
            if old_to_new_intersection_map.values().any(|not_really_new_ref| *not_really_new_ref == IntersectionRef(new_i)) {
                None
            } else {Some((IntersectionRef(new_i), new.clone()))}
        ).collect();

        let intersections_to_destroy = old.intersections.iter().enumerate().filter_map(|(old_, old)|
            if old_to_new_intersection_map.keys().any(|old_ref| *old_ref == IntersectionRef(old_)) {
                None
            } else {Some((IntersectionRef(old_), old.clone()))}
        ).collect();

        let stroke_pairs = self.inbetween_strokes.iter().enumerate().cartesian_product(old.inbetween_strokes.iter().enumerate());
        let old_to_new_stroke_map = stroke_pairs.filter_map(|pair| match pair {
            ((new_i, new), (old_i, old)) => if (&new).is_roughly_within(old, RESULT_DELTA_TOLERANCE) {
                Some((InbetweenStrokeRef(old_i), InbetweenStrokeRef(new_i)))
            } else {None}
        }).collect::<CDict<_, _>>();

        let new_inbetween_strokes = self.inbetween_strokes.iter().enumerate().filter_map(|(new_i, new)|
            if old_to_new_stroke_map.values().any(|not_really_new_ref| *not_really_new_ref == InbetweenStrokeRef(new_i)) {
                None
            } else {Some((InbetweenStrokeRef(new_i), new.clone()))}
        ).collect();

        let inbetween_strokes_to_destroy = self.inbetween_strokes.iter().enumerate().filter_map(|(old_i, old)|
            if old_to_new_stroke_map.keys().any(|old_ref| *old_ref == InbetweenStrokeRef(old_i)) {
                None
            } else {Some((InbetweenStrokeRef(old_i), old.clone()))}
        ).collect();

        PlanResultDelta{
            new_intersections: new_intersections,
            intersections_to_destroy: intersections_to_destroy,
            old_to_new_intersection_map: old_to_new_intersection_map,
            new_inbetween_strokes: new_inbetween_strokes,
            inbetween_strokes_to_destroy: inbetween_strokes_to_destroy,
            old_to_new_stroke_map: old_to_new_stroke_map
        }
    }
}

#[derive(Compact, Clone)]
pub struct PlanResultDelta{
    pub new_intersections: CDict<IntersectionRef, Intersection>,
    pub intersections_to_destroy: CDict<IntersectionRef, Intersection>,
    pub old_to_new_intersection_map: CDict<IntersectionRef, IntersectionRef>,
    pub new_inbetween_strokes: CDict<InbetweenStrokeRef, RoadStroke>,
    pub inbetween_strokes_to_destroy: CDict<InbetweenStrokeRef, RoadStroke>,
    pub old_to_new_stroke_map: CDict<InbetweenStrokeRef, InbetweenStrokeRef>,
}

impl Default for PlanResultDelta{
    fn default() -> PlanResultDelta{
        PlanResultDelta{
            new_intersections: CDict::new(),
            intersections_to_destroy: CDict::new(),
            old_to_new_intersection_map: CDict::new(),
            new_inbetween_strokes: CDict::new(),
            inbetween_strokes_to_destroy: CDict::new(),
            old_to_new_stroke_map: CDict::new()
        }
    }
}

const STROKE_INTERSECTION_WIDTH : N = 8.0;
const INTERSECTION_GROUPING_RADIUS : N = 30.0;

impl Plan{
    pub fn with_delta(&self, delta: &PlanDelta) -> Plan{
        Plan{
            strokes: self.strokes.iter().enumerate().filter_map(|(i, stroke)|
                if delta.strokes_to_destroy.contains_key(RoadStrokeRef(i)) {
                    None
                } else {
                    Some(stroke)
                }
            ).chain(delta.new_strokes.iter()).cloned().collect()
        }
    }

    pub fn get_result(&self) -> PlanResult {

        // Find intersection points

        let mut point_groups = self.strokes.iter().enumerate().flat_map(|(i, stroke_1)| {
            let path_1 = stroke_1.path();
            let band_1 = Band::new(path_1.clone(), STROKE_INTERSECTION_WIDTH).outline();
            self.strokes[i+1..].iter().flat_map(|stroke_2| {
                let path_2 = stroke_2.path();
                let band_2 = Band::new(path_2.clone(), STROKE_INTERSECTION_WIDTH).outline();
                (&band_1, &band_2).intersect().iter().flat_map(|intersection| {
                    let point_1_distance = path_1.project(intersection.position);
                    let mirrored_point_1 = point_1_distance.map(|distance|
                        path_1.along(distance) + (path_1.along(distance) - intersection.position)
                    );
                    let point_2_distance = path_2.project(intersection.position);
                    let mirrored_point_2 = point_2_distance.map(|distance|
                        path_2.along(distance) + (path_2.along(distance) - intersection.position)
                    );
                    vec![intersection.position].into_iter()
                        .chain(mirrored_point_1.into_iter()).chain(mirrored_point_2.into_iter())
                }).collect::<Vec<_>>()
            }).collect::<Vec<_>>()
        }).map(|point| vec![point]).collect::<Vec<_>>();

        // Merge intersection point groups

        let mut merging_ongoing = true;

        while merging_ongoing {
            merging_ongoing = false;
            #[allow(needless_range_loop)]
            for i in 0..point_groups.len() {
                for j in ((i + 1)..point_groups.len()).rev() {
                    let merge_groups = point_groups[i].iter().cartesian_product(point_groups[j].iter())
                        .any(|(point_i, point_j)| (*point_i - *point_j).norm() < INTERSECTION_GROUPING_RADIUS);
                    if merge_groups {
                        let group_to_be_merged = point_groups[j].clone();
                        point_groups[i].extend_from_slice(&group_to_be_merged);
                        point_groups[j].clear();
                        merging_ongoing = true;
                    }
                }
            }
        }

        // Create intersections from point groups

        let mut intersections = point_groups.iter().filter_map(|group|
            if group.len() >= 2 {
                Some(Intersection{
                    shape: convex_hull::<CPath>(group),
                    incoming: CVec::new(),
                    outgoing: CVec::new(),
                    strokes: CVec::new()
                })
            } else {None}
        ).collect::<CVec<_>>();

        // Cut strokes at intersections

        let mut strokes_todo = self.strokes.iter().cloned().collect::<Vec<_>>();
        let mut cutting_ongoing = true;
        let mut iters = 0;

        while cutting_ongoing {
            cutting_ongoing = false;
            let new_strokes = strokes_todo.iter().flat_map(|stroke| {
                let stroke_path = stroke.path();
                let mut maybe_cut_strokes = intersections.iter_mut().filter_map(|intersection| {
                    let intersection_points = (&stroke_path, &intersection.shape).intersect();
                    if intersection_points.len() >= 2 {
                        let entry_distance = intersection_points.iter().map(|p| OrderedFloat(p.along_a)).min().unwrap();
                        let exit_distance = intersection_points.iter().map(|p| OrderedFloat(p.along_a)).max().unwrap();
                        let before_intersection = stroke.cut_before(*entry_distance - 1.0);
                        let after_intersection = stroke.cut_after(*exit_distance + 1.0);
                        intersection.incoming.push(*before_intersection.nodes.last().unwrap());
                        intersection.outgoing.push(after_intersection.nodes[0]);
                        Some(vec![before_intersection, after_intersection])
                    } else if intersection_points.len() == 1 {
                        if intersection.shape.contains(stroke.nodes[0].position) {
                            let exit_distance = intersection_points[0].along_a;
                            let after_intersection = stroke.cut_after(exit_distance + 1.0);
                            intersection.outgoing.push(after_intersection.nodes[0]);
                            Some(vec![after_intersection])
                        } else if intersection.shape.contains(stroke.nodes.last().unwrap().position) {
                            let entry_distance = intersection_points[0].along_a;
                            let before_intersection = stroke.cut_before(entry_distance - 1.0);
                            intersection.incoming.push(*before_intersection.nodes.last().unwrap());
                            Some(vec![before_intersection])
                        } else {None}
                    } else {None}
                });

                match maybe_cut_strokes.next() {
                    Some(cut_strokes) => {
                        cutting_ongoing = true;
                        cut_strokes
                    },
                    None => vec![stroke.clone()]
                }
            }).collect::<Vec<_>>();

            strokes_todo = new_strokes;
            iters += 1;
            if iters > 30 {panic!("Stuck!!!")}
        }

        let inbetween_strokes = strokes_todo.into();

        // Create connecting strokes on intersections

        for intersection in intersections.iter_mut() {
            intersection.strokes = intersection.incoming.iter().flat_map(|incoming|
                intersection.outgoing.iter().map(|outgoing|
                    RoadStroke{nodes: vec![*incoming, *outgoing].into()}
                ).collect::<Vec<_>>()
            ).collect::<CVec<_>>()
        }

        PlanResult{
            intersections: intersections,
            inbetween_strokes: inbetween_strokes
        }
    }
}