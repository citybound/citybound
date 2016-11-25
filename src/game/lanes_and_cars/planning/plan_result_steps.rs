use descartes::{N, P2, Path, Norm, Band, Intersect, convex_hull, Curve, FiniteCurve, RoughlyComparable, Dot, WithUniqueOrthogonal};
use kay::{CVec, CDict};
use core::geometry::{CPath};
use core::merge_groups::MergeGroups;
use ordered_float::OrderedFloat;
use itertools::{Itertools};
use super::{RoadStroke, RoadStrokeNode, MIN_NODE_DISTANCE, Intersection, RoadStrokeRef};

const STROKE_INTERSECTION_WIDTH : N = 4.0;
const INTERSECTION_GROUPING_RADIUS : N = 30.0;

pub fn find_intersections(strokes: &CVec<RoadStroke>) -> CVec<Intersection> {
    let mut intersection_point_groups = find_intersection_points(strokes)
        .into_iter().map(|point| vec![point]).collect::<Vec<_>>();

    intersection_point_groups.merge_groups(|group_1, group_2|
        group_1.iter().cartesian_product(group_2.iter())
            .any(|(point_i, point_j)|
                (*point_i - *point_j).norm() < INTERSECTION_GROUPING_RADIUS));

    intersection_point_groups.iter().filter_map(|group|
        if group.len() >= 2 {
            Some(Intersection{
                shape: convex_hull::<CPath>(group),
                incoming: CDict::new(),
                outgoing: CDict::new(),
                strokes: CVec::new()
            })
        } else {None}
    ).collect::<CVec<_>>()
}

fn find_intersection_points(strokes: &CVec<RoadStroke>) -> Vec<P2> {
    strokes.iter().enumerate().flat_map(|(i, stroke_1)| {
        let path_1 = stroke_1.path();
        let band_1 = Band::new(path_1.clone(), STROKE_INTERSECTION_WIDTH).outline();
        strokes[i+1..].iter().flat_map(|stroke_2| {
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
    }).collect::<Vec<_>>()
}

const MAX_PARALLEL_INTERSECTION_NODES_OFFSET : f32 = 10.0;

type InOrOutGroup = Vec<(RoadStrokeRef, RoadStrokeNode)>;
#[allow(ptr_arg)]
fn merge_incoming_or_outgoing_group(group_1: &InOrOutGroup, group_2: &InOrOutGroup) -> bool {
    let any_incoming_1 = group_1[0].1;
    let any_incoming_2 = group_2[0].1;
    any_incoming_1.direction.is_roughly(any_incoming_2.direction)
        && (any_incoming_1.position - any_incoming_2.position).dot(&any_incoming_1.direction).is_roughly_within(0.0, MAX_PARALLEL_INTERSECTION_NODES_OFFSET)
}

pub fn trim_strokes_and_add_incoming_outgoing(strokes: &CVec<RoadStroke>, intersections: &mut CVec<Intersection>) -> CDict<RoadStrokeRef, RoadStroke> {
    let mut strokes_todo = strokes.iter().cloned().enumerate().map(|(i, stroke)| (RoadStrokeRef(i), stroke)).collect::<CDict<_,_>>();
    let mut trimming_ongoing = true;
    let mut iters = 0;

    while trimming_ongoing {
        trimming_ongoing = false;
        let new_strokes = strokes_todo.pairs().map(|(stroke_ref, stroke)| {
            let stroke_path = stroke.path();
            let mut maybe_trimmed_stroke = intersections.iter_mut().filter_map(|intersection| {
                let intersection_points = (&stroke_path, &intersection.shape).intersect();
                if intersection_points.len() >= 2 {
                    let entry_distance = intersection_points.iter().map(|p| OrderedFloat(p.along_a)).min().unwrap();
                    let exit_distance = intersection_points.iter().map(|p| OrderedFloat(p.along_a)).max().unwrap();
                    intersection.incoming.insert(*stroke_ref, RoadStrokeNode{
                        position: stroke_path.along(*entry_distance),
                        direction: stroke_path.direction_along(*entry_distance)
                    });
                    intersection.outgoing.insert(*stroke_ref, RoadStrokeNode{
                        position: stroke_path.along(*exit_distance),
                        direction: stroke_path.direction_along(*exit_distance)
                    });
                    None
                } else if intersection_points.len() == 1 {
                    if intersection.shape.contains(stroke.nodes[0].position) {
                        let exit_distance = intersection_points[0].along_a;
                        if let Some(after_intersection) = stroke.cut_after(exit_distance + 1.0) {
                            intersection.outgoing.insert(*stroke_ref, after_intersection.nodes[0]);
                            Some(after_intersection)
                        } else {None}
                    } else if intersection.shape.contains(stroke.nodes.last().unwrap().position) {
                        let entry_distance = intersection_points[0].along_a;
                        if let Some(before_intersection) = stroke.cut_before(entry_distance - 1.0) {
                            intersection.incoming.insert(*stroke_ref, *before_intersection.nodes.last().unwrap());
                            Some(before_intersection)
                        } else {None}
                    } else {None}
                } else {None}
            });

            match maybe_trimmed_stroke.next() {
                Some(trimmed_stroke) => {
                    trimming_ongoing = true;
                    (*stroke_ref, trimmed_stroke)
                },
                None => (*stroke_ref, stroke.clone())
            }
        }).collect::<CDict<_, _>>();

        strokes_todo = new_strokes;
        iters += 1;
        if iters > 20 {
            panic!("Stuck!!!")
        }
    }

    strokes_todo
}

pub fn find_transfer_strokes(trimmed_strokes: &CDict<RoadStrokeRef, RoadStroke>) -> Vec<RoadStroke> {
    trimmed_strokes.values().enumerate().flat_map(|(i, stroke_1)|
        trimmed_strokes.values().skip(i + 1).flat_map(|stroke_2| {
            let aligned_node_groups = stroke_1.nodes.iter().map(|node_1| {
                let maybe_node_2 = stroke_2.nodes.iter().find(
                    |node_2| node_1.position.is_roughly_within(node_2.position, 6.0)
                                && node_1.direction.is_roughly_within(node_2.direction, 0.1)
                );
                maybe_node_2.map(|node_2| RoadStrokeNode{
                    position: ((node_1.position.to_vector() + node_2.position.to_vector())/2.0).to_point(),
                    direction: node_1.direction
                })
            }).group_by(|maybe_average_node| maybe_average_node.is_some());

            aligned_node_groups.into_iter().filter_map(|(aligned, aligned_node_group)|
                if aligned {
                    let nodes = aligned_node_group.map(|some_node| some_node.unwrap()).collect::<CVec<_>>();
                    if nodes.len() >= 2 {
                        Some(RoadStroke{nodes: nodes})
                    } else {None}
                } else {None}
            ).collect::<Vec<_>>()
        }).collect::<Vec<_>>()
    ).collect::<Vec<_>>()
}

pub fn cut_transfer_strokes(transfer_strokes: Vec<RoadStroke>, intersections: &CVec<Intersection>) -> CVec<RoadStroke> {
    let mut strokes_todo = transfer_strokes.into_iter().collect::<CVec<_>>();
    let mut cutting_ongoing = true;
    let mut iters = 0;

    while cutting_ongoing {
        cutting_ongoing = false;
        let new_strokes = strokes_todo.iter().flat_map(|stroke| {
            let stroke_path = stroke.path();
            let mut maybe_cut_strokes = intersections.iter().filter_map(|intersection| {
                let intersection_points = (&stroke_path, &intersection.shape).intersect();
                if intersection_points.len() >= 2 {
                    let entry_distance = intersection_points.iter().map(|p| OrderedFloat(p.along_a)).min().unwrap();
                    let exit_distance = intersection_points.iter().map(|p| OrderedFloat(p.along_a)).max().unwrap();

                    match (stroke.cut_before(*entry_distance - 1.0), stroke.cut_after(*exit_distance + 1.0)) {
                        (Some(before_intersection), Some(after_intersection)) => Some(vec![before_intersection, after_intersection]),
                        (Some(before_intersection), None) => Some(vec![before_intersection]),
                        (None, Some(after_intersection)) => Some(vec![after_intersection]),
                        (None, None) => None
                    }
                } else if intersection_points.len() == 1 {
                    if intersection.shape.contains(stroke.nodes[0].position) {
                        let exit_distance = intersection_points[0].along_a;
                        stroke.cut_after(exit_distance + 1.0).map(|after_intersection| vec![after_intersection])
                    } else if intersection.shape.contains(stroke.nodes.last().unwrap().position) {
                        let entry_distance = intersection_points[0].along_a;
                        stroke.cut_before(entry_distance - 1.0).map(|before_intersection| vec![before_intersection])
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
        }).collect::<CVec<_>>();

        strokes_todo = new_strokes;
        iters += 1;
        if iters > 20 {
            panic!("Stuck!!!")
        }
    }

    strokes_todo
}

pub fn create_connecting_strokes(intersections: &mut CVec<Intersection>) {
    for intersection in intersections.iter_mut() {
            let mut incoming_groups = intersection.incoming.pairs().map(
                |(incoming_ref, incoming)| vec![(*incoming_ref, *incoming)]).collect::<Vec<_>>();
            incoming_groups.merge_groups(merge_incoming_or_outgoing_group);
            for incoming_group in &mut incoming_groups {
                let base_position = incoming_group[0].1.position;
                let direction_right = -incoming_group[0].1.direction.orthogonal();
                incoming_group.sort_by_key(|group| OrderedFloat((group.1.position - base_position).dot(&direction_right)));
            }

            let mut outgoing_groups = intersection.outgoing.pairs().map(
                |(outgoing_ref, outgoing)| vec![(*outgoing_ref, *outgoing)]).collect::<Vec<_>>();
            outgoing_groups.merge_groups(merge_incoming_or_outgoing_group);
            for outgoing_group in &mut outgoing_groups {
                let base_position = outgoing_group[0].1.position;
                let direction_right = -outgoing_group[0].1.direction.orthogonal();
                outgoing_group.sort_by_key(|group| OrderedFloat((group.1.position - base_position).dot(&direction_right)));
            }

            intersection.strokes = incoming_groups.iter().flat_map(|incoming_group| {
                outgoing_groups.iter().filter_map(|outgoing_group| {
                    if groups_already_connected(incoming_group, outgoing_group) {
                        None
                    } else {
                        incoming_group.iter().zip(outgoing_group.iter()).filter_map(|(&(_, incoming), &(_, outgoing))|
                            if (incoming.position - outgoing.position).norm() > MIN_NODE_DISTANCE {
                                Some(RoadStroke::new(vec![incoming, outgoing].into()))
                            } else {None}
                        ).min_by_key(|stroke| OrderedFloat(stroke.path().length()))
                    }
                }).collect::<Vec<_>>()
            }).collect::<CVec<_>>();
        }
}

#[allow(ptr_arg)]
fn groups_already_connected(incoming_group: &InOrOutGroup, outgoing_group: &InOrOutGroup) -> bool {
    incoming_group.iter().all(|&(incoming_ref, _)|
        outgoing_group.iter().any(|&(outgoing_ref, _)| incoming_ref == outgoing_ref)
    )
}