use descartes::{N, P2, Path, Norm, Band, Intersect, convex_hull, Curve, FiniteCurve,
                RoughlyComparable, Dot, WithUniqueOrthogonal, Segment, HasBoundingBox, BoundingBox};
use compact::{CVec, CDict};
use core::geometry::CPath;
use core::disjoint_sets::DisjointSets;
use ordered_float::OrderedFloat;
use itertools::Itertools;
use super::plan::{LaneStrokeRef, Intersection};
use super::lane_stroke::{LaneStroke, LaneStrokeNode};

const STROKE_INTERSECTION_WIDTH: N = 4.0;
const INTERSECTION_GROUPING_RADIUS: N = 30.0;

#[inline(never)]
pub fn find_intersections(strokes: &CVec<LaneStroke>) -> CVec<Intersection> {
    let points = find_intersection_points(strokes);
    let mut intersection_point_groups = DisjointSets::from_individuals(points);

    intersection_point_groups.union_all_with_accelerator(
        GridAccelerator::new(200.0),
        |&point, idx, accelerator|
            accelerator.add(idx, vec![BoundingBox::point(point).grown_by(
                INTERSECTION_GROUPING_RADIUS/2.0
            )].into_iter()),
        |accelerator|
            accelerator.colocated_pairs(),
        |point_i, point_j|
            (point_i.x - point_j.x).abs() < INTERSECTION_GROUPING_RADIUS
            && (point_i.y - point_j.y).abs() < INTERSECTION_GROUPING_RADIUS
            && (*point_i - *point_j).norm() < INTERSECTION_GROUPING_RADIUS
    );

    intersection_point_groups.sets()
        .filter_map(|group| if group.len() >= 2 {
            Some(Intersection {
                shape: convex_hull::<CPath>(group).shift_orthogonally(-5.0).unwrap(),
                incoming: CDict::new(),
                outgoing: CDict::new(),
                strokes: CVec::new(),
                timings: CVec::new(),
            })
        } else {
            None
        })
        .collect::<CVec<_>>()
}

use ::core::grid_accelerator::GridAccelerator;

#[allow(let_and_return)]
// stupid lifetime complaining otherwise
#[inline(never)]
fn find_intersection_points(strokes: &CVec<LaneStroke>) -> Vec<P2> {
    let mut grid = GridAccelerator::new(400.0);
    let mut bands = Vec::new();
    for (i, stroke) in strokes.iter().enumerate() {
        bands.push(Band::new(stroke.path().clone(), STROKE_INTERSECTION_WIDTH).outline());
        grid.add(i,
                 stroke.path()
                     .segments()
                     .iter()
                     .map(|segment| segment.bounding_box().grown_by(STROKE_INTERSECTION_WIDTH)));
    }

    let points = grid.colocated_pairs()
        .into_iter()
        .flat_map(|&(stroke_idx_a, ref stroke_idx_b_bmap)| {
            stroke_idx_b_bmap.iter()
                .flat_map(|stroke_idx_b| if stroke_idx_a != stroke_idx_b as usize {
                    (&bands[stroke_idx_a], strokes[stroke_idx_b as usize].path())
                        .intersect()
                        .iter()
                        .map(|intersection| intersection.position)
                        .collect::<Vec<_>>()
                } else {
                    vec![]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    points
}

#[inline(never)]
pub fn trim_strokes_and_add_incoming_outgoing(strokes: &CVec<LaneStroke>,
                                              intersections: &mut CVec<Intersection>)
                                              -> CVec<LaneStroke> {
    let mut strokes = strokes.clone();
    let mut first_new_index = 0;
    // resolve self-intersections
    while {
        let mut something_happened = false;
        let mut split_strokes = Vec::new();

        for i in (first_new_index..strokes.len()).rev() {
            if let Some(self_intersection) =
                strokes[i].path().self_intersections().into_iter().next() {
                let division_distance = (self_intersection.along_a + self_intersection.along_b) /
                                        2.0;
                split_strokes.extend(strokes[i].subsection(0.0, division_distance));
                split_strokes.extend(strokes[i].subsection(division_distance,
                                                           strokes[i].path().length()));
                strokes.remove(i);
                something_happened = true;
            }
        }

        first_new_index = strokes.len();
        strokes.extend(split_strokes);

        something_happened
    } {}

    let ref_strokes = strokes.into_iter().enumerate().map(|(i, stroke)| (LaneStrokeRef(i), stroke));

    ref_strokes.flat_map(|(stroke_ref, stroke)| {
            let path = stroke.path();
            let mut start_trim = 0.0f32;
            let mut end_trim = path.length();
            let mut cuts = Vec::new();

            for ref mut intersection in intersections.iter_mut() {
                let intersection_points = (path, &intersection.shape).intersect();
                if intersection_points.len() >= 2 {
                    let entry_distance =
                        intersection_points.iter().map(|p| OrderedFloat(p.along_a)).min().unwrap();
                    let exit_distance =
                        intersection_points.iter().map(|p| OrderedFloat(p.along_a)).max().unwrap();
                    intersection.incoming.insert(stroke_ref,
                                                 LaneStrokeNode {
                                                     position: path.along(*entry_distance),
                                                     direction:
                                                         path.direction_along(*entry_distance),
                                                 });
                    intersection.outgoing.insert(stroke_ref,
                                                 LaneStrokeNode {
                                                     position: path.along(*exit_distance),
                                                     direction:
                                                         path.direction_along(*exit_distance),
                                                 });
                    cuts.push((*entry_distance, *exit_distance));
                } else if intersection_points.len() == 1 {
                    if intersection.shape.contains(stroke.nodes()[0].position) {
                        let exit_distance = intersection_points[0].along_a;
                        intersection.outgoing.insert(stroke_ref,
                                                     LaneStrokeNode {
                                                         position: path.along(exit_distance),
                                                         direction:
                                                             path.direction_along(exit_distance),
                                                     });
                        start_trim = start_trim.max(exit_distance);
                    } else if intersection.shape.contains(stroke.nodes().last().unwrap().position) {
                        let entry_distance = intersection_points[0].along_a;
                        intersection.incoming.insert(stroke_ref,
                                                     LaneStrokeNode {
                                                         position: path.along(entry_distance),
                                                         direction:
                                                             path.direction_along(entry_distance),
                                                     });
                        end_trim = end_trim.min(entry_distance);
                    }
                }
            }

            cuts.sort_by(|a, b| OrderedFloat(a.0).cmp(&OrderedFloat(b.0)));

            cuts.insert(0, (-1.0, start_trim));
            cuts.push((end_trim, path.length() + 1.0));

            cuts.windows(2)
                .filter_map(|two_cuts| {
                    let ((_, exit_distance), (entry_distance, _)) = (two_cuts[0], two_cuts[1]);
                    stroke.subsection(exit_distance, entry_distance)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[inline(never)]
pub fn find_transfer_strokes(trimmed_strokes: &CVec<LaneStroke>) -> Vec<LaneStroke> {
    let mut grid = GridAccelerator::new(200.0);
    for (i, stroke) in trimmed_strokes.iter().enumerate() {
        grid.add(i,
                 stroke.path()
                     .segments()
                     .iter()
                     .map(|segment| segment.bounding_box().grown_by(6.0)));
    }

    grid.colocated_pairs()
        .into_iter()
        .flat_map(|&(stroke_1_idx, ref stroke_2_idx_bmap)| {
            let stroke_1 = &trimmed_strokes[stroke_1_idx];
            stroke_2_idx_bmap.iter()
                .filter(|stroke_2_idx| stroke_1_idx != *stroke_2_idx as usize)
                .flat_map(|stroke_2_idx| {
                    let stroke_2 = &trimmed_strokes[stroke_2_idx as usize];
                    let path_1 = stroke_1.path();
                    let path_2 = stroke_2.path();
                    let aligned_segments = path_1.segments()
                        .iter()
                        .cartesian_product(path_2.segments().iter())
                        .filter_map(|(segment_1, segment_2)|
                    // TODO: would you look at that horrible mess!
                        match (
                            segment_2.project(segment_1.start()),
                            segment_2.project(segment_1.end()),
                            segment_1.project(segment_2.start()),
                            segment_1.project(segment_2.end())
                        ) {
                            (Some(start_1_on_2_dist), Some(end_1_on_2_dist), _, _) => {
                                let start_1_on_2 = segment_2.along(start_1_on_2_dist);
                                let end_1_on_2 = segment_2.along(end_1_on_2_dist);
                                if start_1_on_2.is_roughly_within(segment_1.start(), 6.0)
                                && end_1_on_2.is_roughly_within(segment_1.end(), 6.0)
                                && segment_2.direction_along(start_1_on_2_dist)
                                    .is_roughly_within(segment_1.start_direction(), 0.05)
                                && segment_2.direction_along(end_1_on_2_dist)
                                    .is_roughly_within(segment_1.end_direction(), 0.05)
                                {
                                    Some(Segment::arc_with_direction(
                                        ((segment_1.start().to_vector()
                                            + start_1_on_2.to_vector()) / 2.0)
                                            .to_point(),
                                        segment_1.start_direction(),
                                        ((segment_1.end().to_vector()
                                            + end_1_on_2.to_vector()) / 2.0)
                                            .to_point(),
                                    ))
                                } else {None}
                            }
                            (_, _, Some(start_2_on_1_dist), Some(end_2_on_1_dist)) => {
                                let start_2_on_1 = segment_1.along(start_2_on_1_dist);
                                let end_2_on_1 = segment_1.along(end_2_on_1_dist);
                                if start_2_on_1.is_roughly_within(segment_2.start(), 6.0)
                                && end_2_on_1.is_roughly_within(segment_2.end(), 6.0)
                                && segment_1.direction_along(start_2_on_1_dist)
                                    .is_roughly_within(segment_2.start_direction(), 0.05)
                                && segment_1.direction_along(end_2_on_1_dist)
                                    .is_roughly_within(segment_2.end_direction(), 0.05)
                                {
                                    Some(Segment::arc_with_direction(
                                        ((segment_2.start().to_vector()
                                            + start_2_on_1.to_vector()) / 2.0)
                                            .to_point(),
                                        segment_2.start_direction(),
                                        ((segment_2.end().to_vector()
                                            + end_2_on_1.to_vector()) / 2.0)
                                            .to_point(),
                                    ))
                                } else {None}
                            },
                            (None, Some(end_1_on_2_dist), Some(start_2_on_1_dist), _) => {
                                let start_2_on_1 = segment_1.along(start_2_on_1_dist);
                                let end_1_on_2 = segment_2.along(end_1_on_2_dist);
                                if start_2_on_1.is_roughly_within(segment_2.start(), 6.0)
                                && end_1_on_2.is_roughly_within(segment_1.end(), 6.0)
                                && !start_2_on_1.to_vector()
                                    .is_roughly_within(end_1_on_2.to_vector(), 6.0)
                                && segment_1.direction_along(start_2_on_1_dist)
                                    .is_roughly_within(segment_2.start_direction(), 0.05)
                                && segment_2.direction_along(end_1_on_2_dist)
                                    .is_roughly_within(segment_1.end_direction(), 0.05)
                                {
                                    Some(Segment::arc_with_direction(
                                        ((segment_2.start().to_vector()
                                            + start_2_on_1.to_vector()) / 2.0)
                                            .to_point(),
                                        segment_2.start_direction(),
                                        ((segment_1.end().to_vector()
                                            + end_1_on_2.to_vector()) / 2.0)
                                            .to_point(),
                                    ))
                                } else {None}
                            }
                            (Some(start_1_on_2_dist), None, None, Some(end_2_on_1_dist)) => {
                                let start_1_on_2 = segment_2.along(start_1_on_2_dist);
                                let end_2_on_1 = segment_1.along(end_2_on_1_dist);
                                if start_1_on_2.is_roughly_within(segment_1.start(), 6.0)
                                && end_2_on_1.is_roughly_within(segment_2.end(), 6.0)
                                && !start_1_on_2.to_vector()
                                    .is_roughly_within(end_2_on_1.to_vector(), 6.0)
                                && segment_2.direction_along(start_1_on_2_dist)
                                    .is_roughly_within(segment_1.start_direction(), 0.05)
                                && segment_1.direction_along(end_2_on_1_dist)
                                    .is_roughly_within(segment_2.end_direction(), 0.05)
                                {
                                    Some(Segment::arc_with_direction(
                                        ((segment_1.start().to_vector()
                                            + start_1_on_2.to_vector()) / 2.0)
                                            .to_point(),
                                        segment_1.start_direction(),
                                        ((segment_2.end().to_vector()
                                            + end_2_on_1.to_vector()) / 2.0)
                                            .to_point(),
                                    ))
                                } else {None}
                            }
                            _ => None
                        })
                        .collect();

                    let mut aligned_segment_sets = DisjointSets::from_individuals(aligned_segments);
                    aligned_segment_sets.union_all_with(|segment_1, segment_2| {
                        segment_1.start().is_roughly_within(segment_2.end(), 0.1) ||
                        segment_1.end().is_roughly_within(segment_2.start(), 0.1)
                    });

                    let aligned_paths = aligned_segment_sets.sets().map(|set| {
                        let mut sorted_segments = set.to_vec();
                        sorted_segments.sort_by(|segment_1, segment_2| if segment_1.start()
                            .is_roughly_within(segment_2.end(), 0.1) {
                            ::std::cmp::Ordering::Greater
                        } else if segment_1.end()
                            .is_roughly_within(segment_2.start(), 0.1) {
                            ::std::cmp::Ordering::Less
                        } else {
                            ::std::cmp::Ordering::Equal
                        });
                        sorted_segments
                    });

                    aligned_paths.flat_map(|segments| {
                            LaneStroke::new(segments.iter()
                                    .map(|segment| {
                                        LaneStrokeNode {
                                            position: segment.start(),
                                            direction: segment.start_direction(),
                                        }
                                    })
                                    .chain(Some(LaneStrokeNode {
                                            position: segments.last().unwrap().end(),
                                            direction: segments.last().unwrap().end_direction(),
                                        })
                                        .into_iter())
                                    .collect())
                                .into_iter()
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

const MAX_PARALLEL_INTERSECTION_NODES_OFFSET: f32 = 10.0;

#[inline(never)]
pub fn create_connecting_strokes(intersections: &mut CVec<Intersection>) {
    for intersection in intersections.iter_mut() {
        let mut incoming_groups_sets =
            DisjointSets::from_individuals(intersection.incoming.pairs().collect());
        incoming_groups_sets.union_all_with(|&(_, incoming_1), &(_, incoming_2)| {
            incoming_1.direction.is_roughly_within(incoming_2.direction, 0.05) &&
            (incoming_1.position - incoming_2.position)
                .dot(&incoming_1.direction)
                .is_roughly_within(0.0, MAX_PARALLEL_INTERSECTION_NODES_OFFSET)
        });
        let mut incoming_groups =
            incoming_groups_sets.sets().map(|set| set.to_vec()).collect::<Vec<_>>();
        for incoming_group in &mut incoming_groups {
            let base_position = incoming_group[0].1.position;
            let direction_right = incoming_group[0].1.direction.orthogonal();
            incoming_group.sort_by_key(|group| {
                OrderedFloat((group.1.position - base_position).dot(&direction_right))
            });
        }

        let mut outgoing_groups_sets =
            DisjointSets::from_individuals(intersection.outgoing.pairs().collect());
        outgoing_groups_sets.union_all_with(|&(_, outgoing_1), &(_, outgoing_2)| {
            outgoing_1.direction.is_roughly_within(outgoing_2.direction, 0.05) &&
            (outgoing_1.position - outgoing_2.position)
                .dot(&outgoing_1.direction)
                .is_roughly_within(0.0, MAX_PARALLEL_INTERSECTION_NODES_OFFSET)
        });
        let mut outgoing_groups =
            outgoing_groups_sets.sets().map(|set| set.to_vec()).collect::<Vec<_>>();
        for outgoing_group in &mut outgoing_groups {
            let base_position = outgoing_group[0].1.position;
            let direction_right = outgoing_group[0].1.direction.orthogonal();
            outgoing_group.sort_by_key(|group| {
                OrderedFloat((group.1.position - base_position).dot(&direction_right))
            });
        }

        intersection.strokes = incoming_groups.iter()
            .flat_map(|incoming_group| {
                if outgoing_groups.iter()
                    .any(|outgoing_group| groups_correspond(incoming_group, outgoing_group)) {
                    // continues after intersection
                    outgoing_groups.iter()
                        .flat_map(|outgoing_group| {
                            if groups_correspond(incoming_group, outgoing_group) {
                                // straight connection
                                connect_as_much_as_possible(incoming_group, outgoing_group)
                                    .into_iter()
                                    .skip((incoming_group.len() as f32 / 3.0).ceil() as usize - 1)
                                    .take(incoming_group.len() -
                                          2 *
                                          ((incoming_group.len() as f32 / 3.0).ceil() as usize - 1))
                                    .collect::<Vec<_>>()
                            } else {
                                connect_as_much_as_possible(incoming_group, outgoing_group)
                                    .into_iter()
                                    .take((incoming_group.len() as f32 / 3.0).ceil() as usize)
                                    .collect::<Vec<_>>()
                            }
                        })
                        .collect::<Vec<_>>()
                } else {
                    // ends in intersection
                    outgoing_groups.iter()
                        .flat_map(|outgoing_group| {
                            connect_as_much_as_possible(incoming_group, outgoing_group)
                                .into_iter()
                                .take((incoming_group.len() as f32 / 2.0).ceil() as usize)
                                .collect::<Vec<_>>()
                        })
                        .collect()
                }
            })
            .collect::<CVec<_>>();
    }
}

#[allow(ptr_arg)]
fn groups_correspond(incoming_group: &Vec<(&LaneStrokeRef, &LaneStrokeNode)>,
                     outgoing_group: &Vec<(&LaneStrokeRef, &LaneStrokeNode)>)
                     -> bool {
    incoming_group.iter().all(|&(incoming_ref, _)| {
        outgoing_group.iter().any(|&(outgoing_ref, _)| incoming_ref == outgoing_ref)
    })
}

#[allow(ptr_arg)]
fn connect_as_much_as_possible(incoming_group: &Vec<(&LaneStrokeRef, &LaneStrokeNode)>,
                               outgoing_group: &Vec<(&LaneStrokeRef, &LaneStrokeNode)>)
                               -> Vec<LaneStroke> {
    let is_right_of = (outgoing_group[0].1.position - incoming_group[0].1.position)
        .dot(&incoming_group[0].1.direction.orthogonal()) > 0.0;

    if is_right_of {
        incoming_group.iter()
            .rev()
            .zip(outgoing_group.iter().rev())
            .flat_map(|(&(_, incoming), &(_, outgoing))| {
                LaneStroke::new(vec![*incoming, *outgoing].into()).into_iter()
            })
            .collect()
    } else {
        let is_uturn =
            outgoing_group[0].1.position.is_roughly_within(incoming_group[0].1.position, 7.0) &&
            outgoing_group[0].1.direction.is_roughly_within(-incoming_group[0].1.direction, 0.1);

        if is_uturn {
            LaneStroke::new(vec![*incoming_group[0].1, *outgoing_group[0].1].into())
                .into_iter()
                .collect()
        } else {
            incoming_group.iter()
                .zip(outgoing_group.iter())
                .flat_map(|(&(_, incoming), &(_, outgoing))| {
                    LaneStroke::new(vec![*incoming, *outgoing].into()).into_iter()
                })
                .collect()
        }
    }
}

pub fn determine_signal_timings(intersections: &mut CVec<Intersection>) {
    use ::roaring::RoaringBitmap;

    for intersection in intersections.iter_mut() {
        // find maximal cliques of compatible lanes using Bron-Kerbosch

        fn compatible(stroke_a: &LaneStroke, stroke_b: &LaneStroke) -> bool {
            let first_a = stroke_a.nodes()[0];
            let first_b = stroke_b.nodes()[0];
            let last_a = stroke_a.nodes().last().unwrap();
            let last_b = stroke_b.nodes().last().unwrap();
            let a_is_uturn = first_a.position.is_roughly_within(last_a.position, 7.0) &&
                             first_a.direction.is_roughly_within(-last_a.direction, 0.1);
            let b_is_uturn = first_b.position.is_roughly_within(last_b.position, 7.0) &&
                             first_b.direction.is_roughly_within(-last_b.direction, 0.1);

            a_is_uturn || b_is_uturn || first_a.position.is_roughly_within(first_b.position, 0.1) ||
            (!last_a.position.is_roughly_within(last_b.position, 0.1) &&
             (stroke_a.path(), stroke_b.path()).intersect().is_empty())

        }

        use ::fnv::FnvHashMap;

        let mut compatabilities = FnvHashMap::<usize, RoaringBitmap<u32>>::default();

        for (a, stroke_a) in intersection.strokes.iter().enumerate() {
            for (b, stroke_b) in intersection.strokes.iter().enumerate().skip(a + 1) {
                if compatible(stroke_a, stroke_b) {
                    compatabilities.entry(a)
                        .or_insert_with(RoaringBitmap::<u32>::new)
                        .insert(b as u32);
                    compatabilities.entry(b)
                        .or_insert_with(RoaringBitmap::<u32>::new)
                        .insert(a as u32);
                }
            }
        }

        #[allow(len_zero)]
        fn bron_kerbosch_helper(r: RoaringBitmap<u32>,
                                mut p: RoaringBitmap<u32>,
                                mut x: RoaringBitmap<u32>,
                                neighbors_map: &FnvHashMap<usize, RoaringBitmap<u32>>,
                                out_max_cliques: &mut Vec<RoaringBitmap<u32>>) {
            let empty_set = RoaringBitmap::<u32>::new();
            let neighbors = |v: u32| neighbors_map.get(&(v as usize)).unwrap_or(&empty_set);
            // TODO: roaring::RoaringBitmap::is_empty is buggy!!
            // https://github.com/Nemo157/roaring-rs/issues/18
            // TODO: Blocked by dependency conflict on num_bigint -_-
            if p.len() == 0 && x.len() == 0 {
                out_max_cliques.push(r);
            } else {
                let pivot =
                    p.union(&x).max_by_key(|&v| (neighbors)(v).len()).expect("should have a pivot");
                for v in p.clone() - (neighbors)(pivot) {
                    let mut just_v = RoaringBitmap::new();
                    just_v.insert(v);
                    bron_kerbosch_helper(r.clone() | just_v,
                                         p.clone() & (neighbors)(v),
                                         x.clone() & (neighbors)(v),
                                         neighbors_map,
                                         out_max_cliques);
                    p.remove(v);
                    x.insert(v);
                }
            }
        }

        fn bron_kerbosch(p: RoaringBitmap<u32>,
                         neighbors: &FnvHashMap<usize, RoaringBitmap<u32>>)
                         -> Vec<RoaringBitmap<u32>> {
            let mut max_cliques = Vec::new();
            bron_kerbosch_helper(RoaringBitmap::<u32>::new(),
                                 p,
                                 RoaringBitmap::<u32>::new(),
                                 neighbors,
                                 &mut max_cliques);
            max_cliques
        }

        let stroke_idx_max_cliques =
            bron_kerbosch((0u32..(intersection.strokes.len() as u32)).into_iter().collect(),
                          &compatabilities);

        let mut cliques_with_parallelity = stroke_idx_max_cliques.into_iter()
            .map(|clique| {
                let mut parallel_groups = Vec::new();
                for stroke_idx in clique.iter() {
                    let start_direction = intersection.strokes[stroke_idx as usize].nodes()[0]
                        .direction;
                    let found = if let Some(&mut (_, ref mut n_members)) =
                        parallel_groups.iter_mut().find(|&&mut (group_direction, _)| {
                            start_direction.is_roughly_within(group_direction, 0.1)
                        }) {
                        *n_members += 1;
                        true
                    } else {
                        false
                    };
                    if !found {
                        parallel_groups.push((start_direction, 1));
                    }
                }

                let parallelity: isize =
                    parallel_groups.into_iter().map(|(_, n_members)| n_members * n_members).sum();
                (clique, parallelity)
            })
            .collect::<Vec<_>>();

        cliques_with_parallelity.sort_by_key(|&(_, ref parallelity)| -parallelity);

        let stroke_idx_max_cliques =
            cliques_with_parallelity.into_iter().map(|(clique, _)| clique).collect::<Vec<_>>();

        // TODO: improvement: reorder here in a way that always tends to the longest waiting lane

        let mut stroke_idx_covered = vec![false; intersection.strokes.len()];

        let stroke_idx_max_cliques = stroke_idx_max_cliques.into_iter()
            .take_while(|clique| {
                let all_covered = stroke_idx_covered.iter().any(|covered| !covered);

                for stroke_idx in clique.iter() {
                    stroke_idx_covered[stroke_idx as usize] = true;
                }

                all_covered
            })
            .collect::<Vec<_>>();

        const SIGNAL_TIMING_BUFFER: usize = 4;
        const MIN_CLIQUE_DURATION: usize = 6;
        use ::std::cmp::max;

        let total_cycle_duration =
            stroke_idx_max_cliques.iter()
                .map(|clique| {
                    max(clique.len() as usize * 2, MIN_CLIQUE_DURATION) + SIGNAL_TIMING_BUFFER
                })
                .sum();

        intersection.timings =
            vec![vec![false; total_cycle_duration].into(); intersection.strokes.len()].into();

        let mut current_offset = 0;

        for (clique, next_clique) in
            stroke_idx_max_cliques.iter().chain(stroke_idx_max_cliques.get(0)).tuple_windows() {
            let clique_duration = max(clique.len() as usize * 2, MIN_CLIQUE_DURATION);

            for stroke_idx in clique.iter() {
                let end_offset = if next_clique.contains(stroke_idx) {
                    current_offset + clique_duration + SIGNAL_TIMING_BUFFER
                } else {
                    current_offset + clique_duration
                };
                for t in current_offset..end_offset {
                    intersection.timings[stroke_idx as usize][t] = true;
                }
            }

            current_offset += clique_duration + SIGNAL_TIMING_BUFFER;
        }
    }
}
