use compact::{CHashMap, CVec};
use descartes::{N, P2, V2, Band, LinePath, ClosedLinePath, Area, Intersect, WithUniqueOrthogonal,
RoughEq, PointContainer, AreaError};
use itertools::Itertools;
use ordered_float::OrderedFloat;

use planning::{Plan, PlanResult, GestureIntent, Prototype, GestureID, Version};

mod intersection_connections;
mod smooth_path;
pub mod interaction;
pub use self::interaction::setup;
use style::dimensions::{LANE_DISTANCE, CENTER_LANE_DISTANCE, MIN_SWITCHING_LANE_LENGTH,
SWITCHING_LANE_OVERLAP_TOLERANCE};

#[derive(Copy, Clone)]
pub struct RoadIntent {
    n_lanes_forward: u8,
    n_lanes_backward: u8,
}

impl RoadIntent {
    pub fn new(n_lanes_forward: u8, n_lanes_backward: u8) -> Self {
        RoadIntent {
            n_lanes_forward,
            n_lanes_backward,
        }
    }
}

#[derive(Compact, Clone)]
pub enum RoadPrototype {
    Lane(LanePrototype),
    SwitchLane(SwitchLanePrototype),
    Intersection(IntersectionPrototype),
    PavedArea(Area),
}

impl RoadPrototype {
    pub fn morphable_from(&self, other: &RoadPrototype) -> bool {
        match (self, other) {
            (&RoadPrototype::Lane(ref lane_1), &RoadPrototype::Lane(ref lane_2)) => {
                lane_1.morphable_from(lane_2)
            }
            (&RoadPrototype::SwitchLane(ref lane_1), &RoadPrototype::SwitchLane(ref lane_2)) => {
                lane_1.morphable_from(lane_2)
            }
            (
                &RoadPrototype::Intersection(ref intersection_1),
                &RoadPrototype::Intersection(ref intersection_2),
            ) => intersection_1.morphable_from(intersection_2),
            _ => false,
        }
    }
}

#[derive(Compact, Clone)]
pub struct LanePrototype(pub LinePath, pub CVec<bool>);

impl LanePrototype {
    pub fn morphable_from(&self, other: &LanePrototype) -> bool {
        match (self, other) {
            (
                &LanePrototype(ref path_1, ref timings_1),
                &LanePrototype(ref path_2, ref timings_2),
            ) => path_1.rough_eq_by(path_2, 0.05) && timings_1[..] == timings_2[..],
        }
    }
}

#[derive(Compact, Clone)]
pub struct SwitchLanePrototype(pub LinePath);

impl SwitchLanePrototype {
    pub fn morphable_from(&self, other: &SwitchLanePrototype) -> bool {
        match (self, other) {
            (&SwitchLanePrototype(ref path_1), &SwitchLanePrototype(ref path_2)) => {
                path_1.rough_eq_by(path_2, 0.05)
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct ConnectionRole {
    straight: bool,
    u_turn: bool,
    inner_turn: bool,
    outer_turn: bool,
}

#[derive(Compact, Clone)]
pub struct IntersectionConnector {
    position: P2,
    direction: V2,
    role: ConnectionRole,
}

impl IntersectionConnector {
    fn new(position: P2, direction: V2) -> Self {
        IntersectionConnector {
            position,
            direction,
            role: ConnectionRole {
                straight: false,
                u_turn: false,
                inner_turn: false,
                outer_turn: false,
            },
        }
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct GestureSideID(isize);

impl GestureSideID {
    pub fn new_forward(gesture_idx: usize) -> Self {
        GestureSideID((gesture_idx + 1) as isize)
    }

    pub fn new_backward(gesture_idx: usize) -> Self {
        GestureSideID(-((gesture_idx + 1) as isize))
    }
}

#[derive(Compact, Clone)]
pub struct IntersectionPrototype {
    area: Area,
    incoming: CHashMap<GestureSideID, CVec<IntersectionConnector>>,
    outgoing: CHashMap<GestureSideID, CVec<IntersectionConnector>>,
    pub connecting_lanes: CHashMap<(GestureSideID, GestureSideID), CVec<LanePrototype>>,
}

impl IntersectionPrototype {
    pub fn morphable_from(&self, other: &IntersectionPrototype) -> bool {
        // TODO: make this better!!
        (&self.area).rough_eq_by(&other.area, 0.1)
    }
}

fn gesture_intent_smooth_paths(plan: &Plan) -> Vec<(GestureID, RoadIntent, LinePath)> {
    plan.gestures
        .pairs()
        .filter_map(|(gesture_id, gesture)| match gesture.intent {
            GestureIntent::Road(ref road_intent) if gesture.points.len() >= 2 => {
                smooth_path::smooth_path_from(&gesture.points)
                    .map(|path| (*gesture_id, *road_intent, path.to_line_path()))
            }
            _ => None,
        })
        .collect::<Vec<_>>()
}

#[cfg_attr(feature = "cargo-clippy", allow(cyclomatic_complexity))]
pub fn calculate_prototypes(
    plan: &Plan,
    _current_result: &PlanResult,
    _based_on: Version,
) -> Result<Vec<Prototype>, AreaError> {
    let gesture_intent_smooth_paths = gesture_intent_smooth_paths(plan);

    let gesture_areas_for_intersection = gesture_intent_smooth_paths
        .iter()
        .map(|&(_, road_intent, ref path)| {
            Band::new_asymmetric(
                path.clone(),
                f32::from(road_intent.n_lanes_backward) * LANE_DISTANCE + 0.4 * LANE_DISTANCE,
                f32::from(road_intent.n_lanes_forward) * LANE_DISTANCE + 0.4 * LANE_DISTANCE,
            ).as_area()
        })
        .collect::<Vec<_>>();

    let mut intersection_areas = gesture_areas_for_intersection
        .iter()
        .enumerate()
        .cartesian_product(gesture_areas_for_intersection.iter().enumerate())
        .filter_map(|((i_a, shape_a), (i_b, shape_b))| {
            if i_a == i_b {
                None
            } else {
                let split = shape_a.split(shape_b);
                Some(split.intersection())
            }
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flat_map(|intersection| intersection.disjoint())
        .collect::<Vec<_>>();

    // add intersections at the starts and ends of gestures
    const END_INTERSECTION_DEPTH: N = 15.0;

    intersection_areas.extend(gesture_intent_smooth_paths.iter().flat_map(
        |&(_, road_intent, ref path)| {
            [
                (path.start(), path.start_direction()),
                (path.end(), path.end_direction()),
            ].into_iter()
                .map(|&(point, direction)| {
                    let orthogonal = direction.orthogonal();
                    let half_depth = direction * END_INTERSECTION_DEPTH / 2.0;
                    let width_backward = orthogonal
                        * (f32::from(road_intent.n_lanes_backward) * LANE_DISTANCE
                            + 0.4 * LANE_DISTANCE);
                    let width_forward = orthogonal
                        * (f32::from(road_intent.n_lanes_forward) * LANE_DISTANCE
                            + 0.4 * LANE_DISTANCE);
                    Area::new_simple(
                        ClosedLinePath::new(
                            LinePath::new(
                                vec![
                                    point - half_depth - width_backward,
                                    point + half_depth - width_backward,
                                    point + half_depth + width_forward,
                                    point - half_depth + width_forward,
                                    point - half_depth - width_backward,
                                ].into(),
                            ).expect("End intersection path should be valid"),
                        ).expect("End intersection path should be closed"),
                    )
                })
                .collect::<Vec<_>>()
        },
    ));

    // union overlapping intersections

    let mut unioned_intersection_area = Area::new(CVec::new());

    for intersection_area in &intersection_areas {
        unioned_intersection_area = unioned_intersection_area.split(intersection_area).union()?;
    }

    let intersection_areas = unioned_intersection_area.disjoint();

    let mut intersection_prototypes: Vec<_> = intersection_areas
        .into_iter()
        .map(|area| {
            Prototype::Road(RoadPrototype::Intersection(IntersectionPrototype {
                area,
                incoming: CHashMap::new(),
                outgoing: CHashMap::new(),
                connecting_lanes: CHashMap::new(),
            }))
        })
        .collect();

    let intersected_lane_paths = {
        let raw_lane_paths = gesture_intent_smooth_paths
            .iter()
            .enumerate()
            .flat_map(|(gesture_i, &(_, road_intent, ref path))| {
                (0..road_intent.n_lanes_forward)
                    .into_iter()
                    .map(|lane_i| CENTER_LANE_DISTANCE / 2.0 + f32::from(lane_i) * LANE_DISTANCE)
                    .chain((0..road_intent.n_lanes_backward).into_iter().map(|lane_i| {
                        -(CENTER_LANE_DISTANCE / 2.0 + f32::from(lane_i) * LANE_DISTANCE)
                    }))
                    .filter_map(|offset| {
                        path.shift_orthogonally(offset).map(|path| {
                            if offset < 0.0 {
                                (GestureSideID::new_backward(gesture_i), path.reverse())
                            } else {
                                (GestureSideID::new_forward(gesture_i), path)
                            }
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        raw_lane_paths
            .into_iter()
            .flat_map(|(gesture_side_id, raw_lane_path)| {
                let mut start_trim = 0.0f32;
                let mut end_trim = raw_lane_path.length();
                let mut cuts = Vec::new();

                for intersection in &mut intersection_prototypes {
                    if let Prototype::Road(RoadPrototype::Intersection(ref mut intersection)) =
                        *intersection
                    {
                        let points = (
                            &raw_lane_path,
                            intersection.area.primitives[0].boundary.path(),
                        ).intersect();

                        if points.len() >= 2 {
                            let entry_distance = points
                                .iter()
                                .map(|p| OrderedFloat(p.along_a))
                                .min()
                                .unwrap();
                            let exit_distance = points
                                .iter()
                                .map(|p| OrderedFloat(p.along_a))
                                .max()
                                .unwrap();
                            intersection.incoming.push_at(
                                gesture_side_id,
                                IntersectionConnector::new(
                                    raw_lane_path.along(*entry_distance),
                                    raw_lane_path.direction_along(*entry_distance),
                                ),
                            );
                            intersection.outgoing.push_at(
                                gesture_side_id,
                                IntersectionConnector::new(
                                    raw_lane_path.along(*exit_distance),
                                    raw_lane_path.direction_along(*exit_distance),
                                ),
                            );
                            cuts.push((*entry_distance, *exit_distance));
                        } else if points.len() == 1 {
                            if intersection.area.contains(raw_lane_path.start()) {
                                let exit_distance = points[0].along_a;
                                intersection.outgoing.push_at(
                                    gesture_side_id,
                                    IntersectionConnector::new(
                                        raw_lane_path.along(exit_distance),
                                        raw_lane_path.direction_along(exit_distance),
                                    ),
                                );
                                start_trim = start_trim.max(exit_distance);
                            } else if intersection.area.contains(raw_lane_path.end()) {
                                let entry_distance = points[0].along_a;
                                intersection.incoming.push_at(
                                    gesture_side_id,
                                    IntersectionConnector::new(
                                        raw_lane_path.along(entry_distance),
                                        raw_lane_path.direction_along(entry_distance),
                                    ),
                                );
                                end_trim = end_trim.min(entry_distance);
                            }
                        }
                    } else {
                        unreachable!()
                    }
                }

                cuts.sort_by(|a, b| OrderedFloat(a.0).cmp(&OrderedFloat(b.0)));

                cuts.insert(0, (-1.0, start_trim));
                cuts.push((end_trim, raw_lane_path.length() + 1.0));

                cuts.windows(2)
                    .filter_map(|two_cuts| {
                        let ((_, exit_distance), (entry_distance, _)) = (two_cuts[0], two_cuts[1]);
                        raw_lane_path.subsection(exit_distance, entry_distance)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };

    let switch_lane_paths = {
        let right_lane_paths_outlines_bands = intersected_lane_paths
            .iter()
            .filter_map(|path| {
                path.shift_orthogonally(0.5 * LANE_DISTANCE)
                    .map(|right_path| {
                        let band = Band::new(right_path.clone(), SWITCHING_LANE_OVERLAP_TOLERANCE);
                        (right_path, band.outline(), band)
                    })
            })
            .collect::<Vec<_>>();

        let left_lane_paths_outlines_bands = intersected_lane_paths
            .iter()
            .filter_map(|path| {
                path.shift_orthogonally(-0.5 * LANE_DISTANCE)
                    .map(|left_path| {
                        let band = Band::new(left_path.clone(), SWITCHING_LANE_OVERLAP_TOLERANCE);
                        (left_path, band.outline(), band)
                    })
            })
            .collect::<Vec<_>>();

        right_lane_paths_outlines_bands
            .iter()
            .cartesian_product(left_lane_paths_outlines_bands.iter())
            .flat_map(
                |(
                    (right_path, right_outline, right_band),
                    (left_path, left_outline, left_band),
                )| {
                    let mut intersections = (right_outline, left_outline).intersect();

                    if intersections.len() < 2 {
                        vec![]
                    } else {
                        intersections.sort_by_key(|intersection| {
                            OrderedFloat(
                                right_band.outline_distance_to_path_distance(intersection.along_a),
                            )
                        });

                        intersections
                            .windows(2)
                            .filter_map(|intersection_pair| {
                                let first_along_right = right_band
                                    .outline_distance_to_path_distance(
                                        intersection_pair[0].along_a,
                                    );
                                let second_along_right = right_band
                                    .outline_distance_to_path_distance(
                                        intersection_pair[1].along_a,
                                    );
                                let first_along_left = left_band.outline_distance_to_path_distance(
                                    intersection_pair[0].along_b,
                                );
                                let second_along_left = left_band
                                    .outline_distance_to_path_distance(
                                        intersection_pair[1].along_b,
                                    );
                                // intersecting subsections go in the same direction on both
                                // lanes?
                                if first_along_left < second_along_left {
                                    // are the midpoints of subsections on each side still in
                                    // range?
                                    if right_path
                                        .along((first_along_right + second_along_right) / 2.0)
                                        .rough_eq_by(
                                            left_path.along(
                                                (first_along_left + second_along_left) / 2.0,
                                            ),
                                            SWITCHING_LANE_OVERLAP_TOLERANCE,
                                        ) {
                                        right_path.subsection(first_along_right, second_along_right)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .coalesce(|prev_subsection, next_subsection| {
                                prev_subsection
                                    .concat(&next_subsection)
                                    .map_err(|_| (prev_subsection, next_subsection))
                            })
                            .collect()
                    }
                },
            )
            .filter(|path| path.length() > MIN_SWITCHING_LANE_LENGTH)
            .collect::<Vec<_>>()
    };

    for prototype in &mut intersection_prototypes {
        if let Prototype::Road(RoadPrototype::Intersection(ref mut intersection)) = *prototype {
            intersection_connections::create_connecting_lanes(intersection);
        } else {
            unreachable!()
        }
    }

    Ok(intersection_prototypes
        .into_iter()
        .chain(
            intersected_lane_paths
                .into_iter()
                .map(|path| Prototype::Road(RoadPrototype::Lane(LanePrototype(path, CVec::new())))),
        )
        .chain(
            switch_lane_paths
                .into_iter()
                .map(|path| Prototype::Road(RoadPrototype::SwitchLane(SwitchLanePrototype(path)))),
        )
        .chain(
            gesture_areas_for_intersection
                .into_iter()
                .map(|shape| Prototype::Road(RoadPrototype::PavedArea(shape))),
        )
        .collect())
}
