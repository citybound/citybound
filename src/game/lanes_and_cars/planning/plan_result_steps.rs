use descartes::{N, P2, Path, Norm, Band, Intersect, convex_hull, Curve, FiniteCurve, RoughlyComparable, Dot, WithUniqueOrthogonal, Segment};
use kay::{CVec, CDict};
use core::geometry::{CPath};
use core::disjoint_sets::DisjointSets;
use ordered_float::OrderedFloat;
use itertools::{Itertools};
use super::{RoadStroke, RoadStrokeNode, MIN_NODE_DISTANCE, Intersection, RoadStrokeRef};

const STROKE_INTERSECTION_WIDTH : N = 4.0;
const INTERSECTION_GROUPING_RADIUS : N = 30.0;

#[inline(never)]
pub fn find_intersections(strokes: &CVec<RoadStroke>) -> CVec<Intersection> {
    let mut intersection_point_groups = DisjointSets::from_individuals(find_intersection_points(strokes));

    intersection_point_groups.union_all_with(|point_i, point_j|
        (point_i.x - point_j.x).abs() < INTERSECTION_GROUPING_RADIUS
        && (point_i.y - point_j.y).abs() < INTERSECTION_GROUPING_RADIUS
        && (*point_i - *point_j).norm() < INTERSECTION_GROUPING_RADIUS
    );

    intersection_point_groups.sets().filter_map(|group|
        if group.len() >= 2 {
            Some(Intersection{
                shape: convex_hull::<CPath>(group).shift_orthogonally(-5.0).unwrap(),
                incoming: CDict::new(),
                outgoing: CDict::new(),
                strokes: CVec::new()
            })
        } else {None}
    ).collect::<CVec<_>>()
}

#[inline(never)]
fn find_intersection_points(strokes: &CVec<RoadStroke>) -> Vec<P2> {
    strokes.iter().enumerate().flat_map(|(i, stroke_1)| {
        let path_1 = stroke_1.path();
        let band_1 = Band::new(path_1.clone(), STROKE_INTERSECTION_WIDTH).outline();
        strokes[i+1..].iter().flat_map(|stroke_2| {
            let path_2 = stroke_2.path();
            let band_2 = Band::new(path_2.clone(), STROKE_INTERSECTION_WIDTH).outline();
            (&band_1, &band_2).intersect().iter().map(|intersection| intersection.position).collect::<Vec<_>>()
        }).collect::<Vec<_>>()
    }).collect::<Vec<_>>()
}

#[inline(never)]
pub fn trim_strokes_and_add_incoming_outgoing(strokes: &CVec<RoadStroke>, intersections: &mut CVec<Intersection>) -> CVec<RoadStroke> {
    let strokes = strokes.iter().cloned().enumerate().map(|(i, stroke)| (RoadStrokeRef(i), stroke)).collect::<Vec<_>>();

    strokes.iter().flat_map(|&(stroke_ref, ref stroke)| {
        let path = stroke.path();
        let mut start_trim = 0.0f32;
        let mut end_trim = path.length();
        let mut cuts = Vec::new();

        for ref mut intersection in intersections.iter_mut() {
            let intersection_points = (path, &intersection.shape).intersect();
            if intersection_points.len() >= 2 {
                let entry_distance = intersection_points.iter().map(|p| OrderedFloat(p.along_a)).min().unwrap();
                let exit_distance = intersection_points.iter().map(|p| OrderedFloat(p.along_a)).max().unwrap();
                intersection.incoming.insert(stroke_ref, RoadStrokeNode{
                    position: path.along(*entry_distance),
                    direction: path.direction_along(*entry_distance)
                });
                intersection.outgoing.insert(stroke_ref, RoadStrokeNode{
                    position: path.along(*exit_distance),
                    direction: path.direction_along(*exit_distance)
                });
                cuts.push((*entry_distance, *exit_distance));
            } else if intersection_points.len() == 1 {
                if intersection.shape.contains(stroke.nodes()[0].position) {
                    let exit_distance = intersection_points[0].along_a;
                    intersection.outgoing.insert(stroke_ref, RoadStrokeNode{
                        position: path.along(exit_distance),
                        direction: path.direction_along(exit_distance)
                    });
                    start_trim = start_trim.max(exit_distance);
                } else if intersection.shape.contains(stroke.nodes().last().unwrap().position) {
                    let entry_distance = intersection_points[0].along_a;
                    intersection.incoming.insert(stroke_ref, RoadStrokeNode{
                        position: path.along(entry_distance),
                        direction: path.direction_along(entry_distance)
                    });
                    end_trim = end_trim.min(entry_distance);
                }
            }
        }

        cuts.sort_by(|a, b| OrderedFloat(a.0).cmp(&OrderedFloat(b.0)));

        cuts.insert(0, (-1.0, start_trim));
        cuts.push((end_trim, path.length() + 1.0));

        cuts.windows(2).filter_map(|two_cuts| {
            let ((_, exit_distance), (entry_distance, _)) = (two_cuts[0], two_cuts[1]);
            stroke.subsection(exit_distance, entry_distance)
        }).collect::<Vec<_>>()
    }).collect()
}

#[inline(never)]
pub fn find_transfer_strokes(trimmed_strokes: &CVec<RoadStroke>) -> Vec<RoadStroke> {
    trimmed_strokes.iter().enumerate().flat_map(|(i, stroke_1)| {
        let path_1 = stroke_1.path();
        trimmed_strokes.iter().skip(i + 1).flat_map(|stroke_2| {
            let path_2 = stroke_2.path();
            let aligned_segments = path_1.segments().iter().cartesian_product(path_2.segments().iter()).filter_map(|(segment_1, segment_2)|
                // TODO: would you look at that horrible mess!
                match (
                    segment_2.project(segment_1.start()), segment_2.project(segment_1.end()),
                    segment_1.project(segment_2.start()), segment_1.project(segment_2.end())
                ) {
                    (Some(start_1_on_2_dist), Some(end_1_on_2_dist), _, _) => {
                        let start_1_on_2 = segment_2.along(start_1_on_2_dist);
                        let end_1_on_2 = segment_2.along(end_1_on_2_dist);
                        if start_1_on_2.is_roughly_within(segment_1.start(), 6.0)
                        && end_1_on_2.is_roughly_within(segment_1.end(), 6.0)
                        && segment_2.direction_along(start_1_on_2_dist).is_roughly_within(segment_1.start_direction(), 0.05)
                        && segment_2.direction_along(end_1_on_2_dist).is_roughly_within(segment_1.end_direction(), 0.05) {
                            Some(Segment::arc_with_direction(
                                ((segment_1.start().to_vector() + start_1_on_2.to_vector()) / 2.0).to_point(),
                                segment_1.start_direction(),
                                ((segment_1.end().to_vector() + end_1_on_2.to_vector()) / 2.0).to_point(),
                            ))
                        } else {None}
                    }
                    (_, _, Some(start_2_on_1_dist), Some(end_2_on_1_dist)) => {
                        let start_2_on_1 = segment_1.along(start_2_on_1_dist);
                        let end_2_on_1 = segment_1.along(end_2_on_1_dist);
                        if start_2_on_1.is_roughly_within(segment_2.start(), 6.0)
                        && end_2_on_1.is_roughly_within(segment_2.end(), 6.0)
                        && segment_1.direction_along(start_2_on_1_dist).is_roughly_within(segment_2.start_direction(), 0.05)
                        && segment_1.direction_along(end_2_on_1_dist).is_roughly_within(segment_2.end_direction(), 0.05) {
                            Some(Segment::arc_with_direction(
                                ((segment_2.start().to_vector() + start_2_on_1.to_vector()) / 2.0).to_point(),
                                segment_2.start_direction(),
                                ((segment_2.end().to_vector() + end_2_on_1.to_vector()) / 2.0).to_point(),
                            ))
                        } else {None}
                    },
                    (None, Some(end_1_on_2_dist), Some(start_2_on_1_dist), _) => {
                        let start_2_on_1 = segment_1.along(start_2_on_1_dist);
                        let end_1_on_2 = segment_2.along(end_1_on_2_dist);
                        if start_2_on_1.is_roughly_within(segment_2.start(), 6.0)
                        && end_1_on_2.is_roughly_within(segment_1.end(), 6.0)
                        && !start_2_on_1.to_vector().is_roughly_within(end_1_on_2.to_vector(), 6.0)
                        && segment_1.direction_along(start_2_on_1_dist).is_roughly_within(segment_2.start_direction(), 0.05)
                        && segment_2.direction_along(end_1_on_2_dist).is_roughly_within(segment_1.end_direction(), 0.05) {
                            Some(Segment::arc_with_direction(
                                ((segment_2.start().to_vector() + start_2_on_1.to_vector()) / 2.0).to_point(),
                                segment_2.start_direction(),
                                ((segment_1.end().to_vector() + end_1_on_2.to_vector()) / 2.0).to_point(),
                            ))
                        } else {None}
                    }
                    (Some(start_1_on_2_dist), None, None, Some(end_2_on_1_dist)) => {
                        let start_1_on_2 = segment_2.along(start_1_on_2_dist);
                        let end_2_on_1 = segment_1.along(end_2_on_1_dist);
                        if start_1_on_2.is_roughly_within(segment_1.start(), 6.0)
                        && end_2_on_1.is_roughly_within(segment_2.end(), 6.0)
                        && !start_1_on_2.to_vector().is_roughly_within(end_2_on_1.to_vector(), 6.0)
                        && segment_2.direction_along(start_1_on_2_dist).is_roughly_within(segment_1.start_direction(), 0.05)
                        && segment_1.direction_along(end_2_on_1_dist).is_roughly_within(segment_2.end_direction(), 0.05) {
                            Some(Segment::arc_with_direction(
                                ((segment_1.start().to_vector() + start_1_on_2.to_vector()) / 2.0).to_point(),
                                segment_1.start_direction(),
                                ((segment_2.end().to_vector() + end_2_on_1.to_vector()) / 2.0).to_point(),
                            ))
                        } else {None}
                    }
                    _ => None
                }
            ).collect();

            let mut aligned_segment_sets = DisjointSets::from_individuals(aligned_segments);
            aligned_segment_sets.union_all_with(|segment_1, segment_2|
                segment_1.start().is_roughly_within(segment_2.end(), 0.1)
                || segment_1.end().is_roughly_within(segment_2.start(), 0.1)
            );

            let aligned_paths = aligned_segment_sets.sets().map(|set| {
                let mut sorted_segments = set.to_vec();
                sorted_segments.sort_by(|segment_1, segment_2|
                    if segment_1.start().is_roughly_within(segment_2.end(), 0.1) {
                        ::std::cmp::Ordering::Greater
                    } else if segment_1.end().is_roughly_within(segment_2.start(), 0.1) {
                        ::std::cmp::Ordering::Less
                    } else {
                        ::std::cmp::Ordering::Equal
                    }
                );
                sorted_segments
            });

            aligned_paths.map(|segments| RoadStroke::new(
                segments.iter().map(|segment|
                    RoadStrokeNode{position: segment.start(), direction: segment.start_direction()}
                ).chain(Some(
                    RoadStrokeNode{position: segments.last().unwrap().end(), direction: segments.last().unwrap().end_direction()}
                ).into_iter()).collect()
            )).collect::<Vec<_>>()
        }).collect::<Vec<_>>()
    }).collect::<Vec<_>>()
}

const MAX_PARALLEL_INTERSECTION_NODES_OFFSET : f32 = 10.0;

#[inline(never)]
pub fn create_connecting_strokes(intersections: &mut CVec<Intersection>) {
    for intersection in intersections.iter_mut() {
            let mut incoming_groups_sets = DisjointSets::from_individuals(intersection.incoming.pairs().collect());
            incoming_groups_sets.union_all_with(|&(_, incoming_1), &(_, incoming_2)|
                incoming_1.direction.is_roughly_within(incoming_2.direction, 0.05)
                && (incoming_1.position - incoming_2.position).dot(&incoming_1.direction).is_roughly_within(0.0, MAX_PARALLEL_INTERSECTION_NODES_OFFSET)
            );
            let mut incoming_groups = incoming_groups_sets.sets().map(|set| set.to_vec()).collect::<Vec<_>>();
            for incoming_group in &mut incoming_groups {
                let base_position = incoming_group[0].1.position;
                let direction_right = -incoming_group[0].1.direction.orthogonal();
                incoming_group.sort_by_key(|group| OrderedFloat((group.1.position - base_position).dot(&direction_right)));
            }

            let mut outgoing_groups_sets = DisjointSets::from_individuals(intersection.outgoing.pairs().collect());
            outgoing_groups_sets.union_all_with(|&(_, outgoing_1), &(_, outgoing_2)|
                outgoing_1.direction.is_roughly_within(outgoing_2.direction, 0.05)
                && (outgoing_1.position - outgoing_2.position).dot(&outgoing_1.direction).is_roughly_within(0.0, MAX_PARALLEL_INTERSECTION_NODES_OFFSET)
            );
            let mut outgoing_groups = outgoing_groups_sets.sets().map(|set| set.to_vec()).collect::<Vec<_>>();
            for outgoing_group in &mut outgoing_groups {
                let base_position = outgoing_group[0].1.position;
                let direction_right = -outgoing_group[0].1.direction.orthogonal();
                outgoing_group.sort_by_key(|group| OrderedFloat((group.1.position - base_position).dot(&direction_right)));
            }

            intersection.strokes = incoming_groups.iter().flat_map(|incoming_group| {
                outgoing_groups.iter().flat_map(|outgoing_group| {
                    if groups_correspond(incoming_group, outgoing_group) {
                        // straight connection
                        incoming_group.iter().cartesian_product(outgoing_group.iter()).filter_map(
                            |(&(incoming_ref, incoming), &(outgoing_ref, outgoing))|
                            if incoming_ref == outgoing_ref {
                                Some(RoadStroke::new(vec![*incoming, *outgoing].into()))
                            } else {None}
                        ).into_iter().collect::<Vec<_>>()
                    } else {
                        incoming_group.iter().zip(outgoing_group.iter()).filter_map(|(&(_, incoming), &(_, outgoing))|
                            if (incoming.position - outgoing.position).norm() > MIN_NODE_DISTANCE {
                                Some(RoadStroke::new(vec![*incoming, *outgoing].into()))
                            } else {None}
                        ).min_by_key(|stroke| OrderedFloat(stroke.path().length())).into_iter().collect::<Vec<_>>()
                    }
                }).collect::<Vec<_>>()
            }).collect::<CVec<_>>();
        }
}

#[allow(ptr_arg)]
fn groups_correspond(incoming_group: &Vec<(&RoadStrokeRef, &RoadStrokeNode)>, outgoing_group: &Vec<(&RoadStrokeRef, &RoadStrokeNode)>) -> bool {
    incoming_group.iter().all(|&(incoming_ref, _)|
        outgoing_group.iter().any(|&(outgoing_ref, _)| incoming_ref == outgoing_ref)
    )
}