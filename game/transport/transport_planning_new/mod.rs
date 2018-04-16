use kay::World;
use compact::{CHashMap, CVec};
use descartes::{N, P2, V2, Band, Segment, Path, FiniteCurve, Shape, SimpleShape, clipper,
                Intersect, WithUniqueOrthogonal, RoughlyComparable};
use monet::{RendererID, Instance, Geometry};
use stagemaster::geometry::{band_to_geometry, CPath, CShape};
use itertools::Itertools;
use style::colors;
use ordered_float::OrderedFloat;

use planning_new::{Plan, GestureIntent, PlanResult, Prototype};

#[derive(Compact, Clone)]
pub struct RoadIntent {
    n_lanes_forward: u8,
    n_lanes_backward: u8,
}

impl RoadIntent {
    pub fn new(n_lanes_forward: u8, n_lanes_backward: u8) -> Self {
        RoadIntent { n_lanes_forward, n_lanes_backward }
    }
}

#[derive(Compact, Clone)]
pub enum RoadPrototype {
    Lane(LanePrototype),
    Intersection(IntersectionPrototype),
}

#[derive(Compact, Clone)]
pub struct LanePrototype(CPath, CVec<bool>);

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
struct GestureSideID(isize);

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
    shape: CShape,
    incoming: CHashMap<GestureSideID, CVec<IntersectionConnector>>,
    outgoing: CHashMap<GestureSideID, CVec<IntersectionConnector>>,
    connecting_lanes: CHashMap<(GestureSideID, GestureSideID), CVec<LanePrototype>>,
}

const LANE_WIDTH: N = 6.0;
const LANE_DISTANCE: N = 0.8 * LANE_WIDTH;
const CENTER_LANE_DISTANCE: N = LANE_DISTANCE;

#[allow(cyclomatic_complexity)]
pub fn calculate_prototypes(plan: &Plan) -> Vec<Prototype> {
    let gesture_intent_smooth_paths = plan.gestures
        .pairs()
        .filter_map(|(gesture_id, gesture)| match gesture.intent {
            GestureIntent::Road(ref road_intent) if gesture.points.len() >= 2 => {

                let center_points = gesture
                    .points
                    .windows(2)
                    .map(|point_pair| {
                        P2::from_coordinates((point_pair[0].coords + point_pair[1].coords) / 2.0)
                    })
                    .collect::<Vec<_>>();

                // for each straight line segment, we have first: a point called END,
                // marking the end of the circular arc that smoothes the first corner of
                // this line segment and then second: a point called START,
                // marking the beginning of the circular arc that smoothes the second corner
                // of this line segments. Also, we remember the direction of the line segment

                let mut end_start_directions = Vec::new();

                for (i, point_pair) in gesture.points.windows(2).enumerate() {
                    let first_corner = point_pair[0];
                    let second_corner = point_pair[1];
                    let previous_center_point = if i < 1 {
                        &first_corner
                    } else {
                        &center_points[i - 1]
                    };
                    let this_center_point = center_points[i];
                    let next_center_point = center_points.get(i + 1).unwrap_or(&second_corner);
                    let line_direction = (second_corner - first_corner).normalize();

                    let shorter_distance_to_first_corner =
                        (first_corner - previous_center_point).norm().min(
                            (first_corner - this_center_point).norm(),
                        );
                    let shorter_distance_to_second_corner =
                        (second_corner - this_center_point).norm().min(
                            (second_corner - next_center_point).norm(),
                        );

                    let end = first_corner + line_direction * shorter_distance_to_first_corner;
                    let start = second_corner - line_direction * shorter_distance_to_second_corner;

                    end_start_directions.push((end, start, line_direction));
                }

                let mut segments = Vec::new();
                let mut previous_point = gesture.points[0];
                let mut previous_direction = (gesture.points[1] - gesture.points[0]).normalize();

                for (end, start, direction) in end_start_directions {
                    if let Some(valid_incoming_arc) =
                        Segment::arc_with_direction(previous_point, previous_direction, end)
                    {
                        segments.push(valid_incoming_arc);
                    }

                    if let Some(valid_connecting_line) = Segment::line(end, start) {
                        segments.push(valid_connecting_line);
                    }

                    previous_point = start;
                    previous_direction = direction;
                }

                CPath::new(segments).ok().map(|path| {
                    (gesture_id, road_intent, path)
                })

            }
            _ => None,
        })
        .collect::<Vec<_>>();


    let gesture_shapes_for_intersection = gesture_intent_smooth_paths
        .iter()
        .map(|&(_, road_intent, ref base_path)| {

            let extended_path = CPath::new(
                Segment::line(
                    base_path.start() - base_path.start_direction() * 10.0,
                    base_path.start(),
                ).into_iter()
                    .chain(base_path.segments().iter().cloned())
                    .chain(Segment::line(
                        base_path.end(),
                        base_path.end() + base_path.end_direction() * 10.0,
                    ))
                    .collect(),
            ).expect("Extending should always work");

            let right_path = extended_path
                .shift_orthogonally(
                    CENTER_LANE_DISTANCE / 2.0 + road_intent.n_lanes_forward as f32 * LANE_DISTANCE,
                )
                .unwrap_or_else(|| extended_path.clone())
                .reverse();
            let left_path =
                extended_path
                    .shift_orthogonally(
                        -(CENTER_LANE_DISTANCE / 2.0 +
                              road_intent.n_lanes_backward as f32 * LANE_DISTANCE),
                    )
                    .unwrap_or_else(|| extended_path.clone());

            let outline_segments = left_path
                .segments()
                .iter()
                .cloned()
                .chain(Segment::line(left_path.end(), right_path.start()))
                .chain(right_path.segments().iter().cloned())
                .chain(Segment::line(right_path.end(), left_path.start()))
                .collect();

            CShape::new(CPath::new(outline_segments).expect(
                "Road outline path should be valid",
            )).expect("Road outline shape should be valid")
        })
        .collect::<Vec<_>>();

    let mut intersection_shapes = gesture_shapes_for_intersection
        .iter()
        .enumerate()
        .cartesian_product(gesture_shapes_for_intersection.iter().enumerate())
        .flat_map(|((i_a, shape_a), (i_b, shape_b))| {
            println!("{} {}", i_a, i_a);
            if i_a == i_b {
                vec![]
            } else {
                match clipper::clip(clipper::Mode::Intersection, shape_a, shape_b) {
                    Ok(shapes) => shapes,
                    Err(err) => {
                        println!("Intersection clipping error: {:?}", err);
                        vec![]
                    }
                }

            }
        })
        .collect::<Vec<_>>();

    let mut i = 0;

    // union overlapping intersections

    while i < intersection_shapes.len() {
        let mut advance = true;

        for j in (i + 1)..intersection_shapes.len() {
            match clipper::clip(
                clipper::Mode::Union,
                &intersection_shapes[i],
                &intersection_shapes[j],
            ) {
                Ok(results) => {
                    if results.len() >= 1 {
                        intersection_shapes[i] = results[0].clone();
                        intersection_shapes.remove(j);
                        advance = false;
                        break;
                    }
                }
                Err(err) => {
                    println!("Intersection combining clipping error: {:?}", err);
                }
            }
        }

        if advance {
            i += 1;
        }
    }

    let mut intersection_prototypes: Vec<_> = intersection_shapes
        .into_iter()
        .map(|shape| {
            Prototype::Road(RoadPrototype::Intersection(IntersectionPrototype {
                shape: shape,
                incoming: CHashMap::new(),
                outgoing: CHashMap::new(),
                connecting_lanes: CHashMap::new(),
            }))
        })
        .collect();

    let lane_prototypes = {
        let raw_lane_paths = gesture_intent_smooth_paths
            .iter()
            .enumerate()
            .flat_map(|(gesture_i, &(_, road_intent, ref path))| {
                (0..road_intent.n_lanes_forward)
                    .into_iter()
                    .map(|lane_i| {
                        CENTER_LANE_DISTANCE / 2.0 + lane_i as f32 * LANE_DISTANCE
                    })
                    .chain((0..road_intent.n_lanes_backward).into_iter().map(
                        |lane_i| {
                            -(CENTER_LANE_DISTANCE / 2.0 + lane_i as f32 * LANE_DISTANCE)
                        },
                    ))
                    .filter_map(|offset| {
                        path.shift_orthogonally(offset).map(|path| if offset < 0.0 {
                            (GestureSideID::new_backward(gesture_i), path.reverse())
                        } else {
                            (GestureSideID::new_forward(gesture_i), path)
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let intersected_lane_paths = raw_lane_paths.into_iter().flat_map(|(gesture_side_id,
          raw_lane_path)| {
            let mut start_trim = 0.0f32;
            let mut end_trim = raw_lane_path.length();
            let mut cuts = Vec::new();

            for intersection in &mut intersection_prototypes {
                if let Prototype::Road(RoadPrototype::Intersection(ref mut intersection)) =
                    *intersection
                {
                    let intersection_points = (&raw_lane_path, intersection.shape.outline())
                        .intersect();
                    if intersection_points.len() >= 2 {
                        let entry_distance = intersection_points
                            .iter()
                            .map(|p| OrderedFloat(p.along_a))
                            .min()
                            .unwrap();
                        let exit_distance = intersection_points
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
                    } else if intersection_points.len() == 1 {
                        if intersection.shape.contains(raw_lane_path.start()) {
                            let exit_distance = intersection_points[0].along_a;
                            intersection.outgoing.push_at(
                                gesture_side_id,
                                IntersectionConnector::new(
                                    raw_lane_path.along(exit_distance),
                                    raw_lane_path.direction_along(exit_distance),
                                ),
                            );
                            start_trim = start_trim.max(exit_distance);
                        } else if intersection.shape.contains(raw_lane_path.end()) {
                            let entry_distance = intersection_points[0].along_a;
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
        });

        intersected_lane_paths
            .into_iter()
            .map(|path| {
                Prototype::Road(RoadPrototype::Lane(LanePrototype(path, CVec::new())))
            })
            .collect::<Vec<_>>()
    };

    for prototype in &mut intersection_prototypes {
        if let Prototype::Road(RoadPrototype::Intersection(ref mut intersection)) = *prototype {
            // sort intersection connectors from inner to outer lanes
            for incoming_group in intersection.incoming.values_mut() {
                let base_position = incoming_group[0].position;
                let direction_right = incoming_group[0].direction.orthogonal();
                incoming_group.sort_by_key(|connector| {
                    OrderedFloat((connector.position - base_position).dot(&direction_right))
                });
            }

            for outgoing_group in intersection.outgoing.values_mut() {
                let base_position = outgoing_group[0].position;
                let direction_right = outgoing_group[0].direction.orthogonal();
                outgoing_group.sort_by_key(|connector| {
                    OrderedFloat((connector.position - base_position).dot(&direction_right))
                });
            }

            const STRAIGHT_ANGLE_THRESHOLD: f32 = ::std::f32::consts::FRAC_PI_6;

            fn role_between_groups(
                incoming: &[IntersectionConnector],
                outgoing: &[IntersectionConnector],
            ) -> ConnectionRole {
                let straight = incoming[0].direction;
                let connection_direction = outgoing[0].position - incoming[0].position;

                if ::descartes::angle_to(straight, connection_direction).abs() <
                    STRAIGHT_ANGLE_THRESHOLD
                {
                    ConnectionRole {
                        straight: true,
                        inner_turn: false,
                        u_turn: false,
                        outer_turn: false,
                    }
                } else {
                    let is_right_of = connection_direction.dot(&straight.orthogonal()) > 0.0;

                    if is_right_of {
                        ConnectionRole {
                            straight: false,
                            inner_turn: false,
                            u_turn: false,
                            outer_turn: true,
                        }
                    } else {
                        let is_uturn = outgoing[0].position.is_roughly_within(
                            incoming[0].position,
                            LANE_DISTANCE * 4.0,
                        ) &&
                            outgoing[0].direction.is_roughly_within(
                                -incoming[0].direction,
                                0.1,
                            );

                        if is_uturn {
                            ConnectionRole {
                                straight: false,
                                inner_turn: false,
                                u_turn: true,
                                outer_turn: false,
                            }
                        } else {
                            ConnectionRole {
                                straight: false,
                                inner_turn: true,
                                u_turn: false,
                                outer_turn: false,
                            }
                        }
                    }
                }
            }

            // assign roles to connectors
            {
                for incoming_group in intersection.incoming.values_mut() {
                    let n_lanes = incoming_group.len();

                    let has_inner_turn = intersection.outgoing.values().any(|outgoing_group| {
                        role_between_groups(incoming_group, outgoing_group).inner_turn
                    });
                    let has_straight = intersection.outgoing.values().any(|outgoing_group| {
                        role_between_groups(incoming_group, outgoing_group).straight
                    });
                    let has_outer_turn = intersection.outgoing.values().any(|outgoing_group| {
                        role_between_groups(incoming_group, outgoing_group).outer_turn
                    });

                    let (n_inner_turn_lanes, n_outer_turn_lanes) =
                        match (has_inner_turn, has_straight, has_outer_turn) {
                            (true, true, true) => ((n_lanes / 4).max(1), (n_lanes / 4).max(1)),
                            (false, true, true) => (0, (n_lanes / 3).max(1)),
                            (true, true, false) => ((n_lanes / 3).max(1), 0),
                            (false, _, false) => (0, 0),
                            (true, false, false) => (n_lanes, 0),
                            (false, false, true) => (0, n_lanes),
                            (true, false, true) => ((n_lanes / 2).max(1), (n_lanes / 2).max(1)),
                        };

                    for (l, incoming_lane) in incoming_group.iter_mut().enumerate() {
                        if l == 0 && has_inner_turn {
                            incoming_lane.role.u_turn = true;
                        }
                        if l < n_inner_turn_lanes {
                            incoming_lane.role.inner_turn = true;
                        }
                        if n_lanes < 3 ||
                            (l >= n_inner_turn_lanes && l < n_lanes - n_outer_turn_lanes)
                        {
                            incoming_lane.role.straight = true;
                        }
                        if l >= n_lanes - n_outer_turn_lanes {
                            incoming_lane.role.outer_turn = true;
                        }
                    }
                }

                for outgoing_group in intersection.outgoing.values_mut() {
                    let n_lanes = outgoing_group.len();

                    let has_inner_turn = intersection.incoming.values().any(|incoming_group| {
                        role_between_groups(incoming_group, outgoing_group).inner_turn
                    });
                    let has_straight = intersection.incoming.values().any(|incoming_group| {
                        role_between_groups(incoming_group, outgoing_group).straight
                    });
                    let has_outer_turn = intersection.incoming.values().any(|incoming_group| {
                        role_between_groups(incoming_group, outgoing_group).outer_turn
                    });

                    let (n_inner_turn_lanes, n_outer_turn_lanes) =
                        match (has_inner_turn, has_straight, has_outer_turn) {
                            (true, true, true) => ((n_lanes / 4).max(1), (n_lanes / 4).max(1)),
                            (false, true, true) => (0, (n_lanes / 3).max(1)),
                            (true, true, false) => ((n_lanes / 3).max(1), 0),
                            (false, _, false) => (0, 0),
                            (true, false, false) => (n_lanes, 0),
                            (false, false, true) => (0, n_lanes),
                            (true, false, true) => ((n_lanes / 2).max(1), (n_lanes / 2).max(1)),
                        };

                    for (l, incoming_lane) in outgoing_group.iter_mut().enumerate() {
                        if l == 0 && has_inner_turn {
                            incoming_lane.role.u_turn = true;
                        }
                        if l < n_inner_turn_lanes {
                            incoming_lane.role.inner_turn = true;
                        }
                        if n_lanes < 3 ||
                            l >= n_inner_turn_lanes && l < n_lanes - n_outer_turn_lanes
                        {
                            incoming_lane.role.straight = true;
                        }
                        if l >= n_lanes - n_outer_turn_lanes {
                            incoming_lane.role.outer_turn = true;
                        }
                    }
                }

                let mut connecting_lane_bundles = intersection
                    .incoming
                    .pairs()
                    .flat_map(|(incoming_gesture_side_id, incoming_group)| {
                        intersection
                            .outgoing
                            .pairs()
                            .map(|(outgoing_gesture_side_id, outgoing_group)| {
                                let role = role_between_groups(incoming_group, outgoing_group);

                                let relevant_incoming_connectors = incoming_group
                                    .iter()
                                    .filter(|connector| {
                                        (role.u_turn && connector.role.u_turn) ||
                                            (role.inner_turn && connector.role.inner_turn) ||
                                            (role.straight && connector.role.straight) ||
                                            (role.outer_turn && connector.role.outer_turn)
                                    })
                                    .collect::<Vec<_>>();
                                let relevant_incoming_len = relevant_incoming_connectors.len();

                                let relevant_outgoing_connectors = outgoing_group
                                    .iter()
                                    .filter(|connector| {
                                        (role.u_turn && connector.role.u_turn) ||
                                            (role.inner_turn && connector.role.inner_turn) ||
                                            (role.straight && connector.role.straight) ||
                                            (role.outer_turn && connector.role.outer_turn)
                                    })
                                    .collect::<Vec<_>>();
                                let relevant_outgoing_len = relevant_outgoing_connectors.len();

                                let lanes =
                                    if relevant_incoming_len > 0 && relevant_outgoing_len > 0 {
                                        (0..relevant_incoming_len.max(relevant_outgoing_len))
                                            .into_iter()
                                            .filter_map(|l| {
                                                let start = relevant_incoming_connectors[l.min(
                                                    relevant_incoming_len -
                                                        1,
                                                )];
                                                let end = relevant_outgoing_connectors[l.min(
                                                    relevant_outgoing_len -
                                                        1,
                                                )];
                                                let path = CPath::new(Segment::biarc(
                                                    start.position,
                                                    start.direction,
                                                    end.position,
                                                    end.direction,
                                                )?).ok()?;

                                                Some(LanePrototype(path, CVec::new()))
                                            })
                                            .collect::<Vec<_>>()
                                    } else {
                                        vec![]
                                    };

                                (
                                    (role, *incoming_gesture_side_id, *outgoing_gesture_side_id),
                                    lanes,
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();

                // find traffic light timings
                let mut phases = Vec::<(Vec<(GestureSideID, GestureSideID)>, usize)>::new();

                let mut unused_connecting_bundles = connecting_lane_bundles.clone();

                intersection.connecting_lanes = connecting_lane_bundles
                    .iter()
                    .map(|&((_, incoming_id, outgoing_id), ref lanes)| {
                        ((incoming_id, outgoing_id), lanes.clone().into())
                    })
                    .collect();

                fn compatible(lanes_a: &[LanePrototype], lanes_b: &[LanePrototype]) -> bool {
                    lanes_a.iter().cartesian_product(lanes_b).all(
                        |(&LanePrototype(ref path_a, _), &LanePrototype(ref path_b, _))| {
                            path_a.start().is_roughly_within(path_b.start(), 0.1) ||
                                (!path_a.end().is_roughly_within(path_b.end(), 0.1) &&
                                     (path_a, path_b).intersect().is_empty())
                        },
                    )
                }

                let mut iteration = 0;
                while !unused_connecting_bundles.is_empty() {
                    let mut current_lanes = vec![];
                    phases.push((Vec::new(), 0));

                    {
                        let mut pop_unused_compatible_where =
                            |role_check: fn(ConnectionRole) -> bool,
                             current_lanes: &mut Vec<LanePrototype>,
                             iteration: usize| {
                                unused_connecting_bundles.retain(|&((role,
                                    incoming_id,
                                    outgoing_id),
                                   ref lanes)| if role_check(role) &&
                                    compatible(
                                        lanes,
                                        current_lanes,
                                    )
                                {
                                    current_lanes.extend(lanes.iter().cloned());
                                    phases[iteration].0.push((incoming_id, outgoing_id));
                                    false
                                } else {
                                    true
                                });
                            };

                        if iteration % 2 == 0 {
                            // straight phase: consider nonconflicting straights, then outer, then inner/u turns
                            pop_unused_compatible_where(
                                |role| role.straight,
                                &mut current_lanes,
                                iteration,
                            );
                            pop_unused_compatible_where(
                                |role| role.outer_turn,
                                &mut current_lanes,
                                iteration,
                            );
                            pop_unused_compatible_where(
                                |role| (role.inner_turn || role.u_turn),
                                &mut current_lanes,
                                iteration,
                            );
                        } else {
                            // inner phase: consider nonconflicting inner/u turns, then outer turns, then straights
                            pop_unused_compatible_where(
                                |role| (role.inner_turn || role.u_turn),
                                &mut current_lanes,
                                iteration,
                            );
                            pop_unused_compatible_where(
                                |role| role.outer_turn,
                                &mut current_lanes,
                                iteration,
                            );
                            pop_unused_compatible_where(
                                |role| role.straight,
                                &mut current_lanes,
                                iteration,
                            );
                        }
                    }

                    {
                        let mut reuse_compatible_where =
                            |role_check: fn(ConnectionRole) -> bool,
                             current_lanes: &mut Vec<LanePrototype>,
                             iteration: usize| {
                                for &((role, incoming_id, outgoing_id), ref lanes) in
                                    &connecting_lane_bundles
                                {
                                    if role_check(role) && compatible(lanes, current_lanes) {
                                        current_lanes.extend(lanes.iter().cloned());
                                        phases[iteration].0.push((incoming_id, outgoing_id));
                                    }
                                }
                            };

                        if iteration % 2 == 0 {
                            // straight phase: consider nonconflicting straights, then outer, then inner/u turns
                            reuse_compatible_where(
                                |role| role.straight,
                                &mut current_lanes,
                                iteration,
                            );
                            reuse_compatible_where(
                                |role| role.outer_turn,
                                &mut current_lanes,
                                iteration,
                            );
                            reuse_compatible_where(
                                |role| (role.inner_turn || role.u_turn),
                                &mut current_lanes,
                                iteration,
                            );
                        } else {
                            // inner phase: consider nonconflicting inner/u turns, then outer turns, then straights
                            reuse_compatible_where(
                                |role| (role.inner_turn || role.u_turn),
                                &mut current_lanes,
                                iteration,
                            );
                            reuse_compatible_where(
                                |role| role.outer_turn,
                                &mut current_lanes,
                                iteration,
                            );
                            reuse_compatible_where(
                                |role| role.straight,
                                &mut current_lanes,
                                iteration,
                            );
                        }
                    }

                    phases[iteration].1 = current_lanes.len();

                    iteration += 1;
                }

                for ((incoming_id, outgoing_id), ref mut lanes) in
                    intersection.connecting_lanes.pairs_mut()
                {
                    let timings: CVec<bool> = phases
                        .iter()
                        .flat_map(|&(ref connections_in_phase, duration)| {
                            let in_phase =
                                connections_in_phase.contains(&(incoming_id, outgoing_id));
                            vec![in_phase; duration]
                        })
                        .collect();

                    for &mut LanePrototype(_, ref mut lane_timings) in lanes.iter_mut() {
                        *lane_timings = timings.clone()
                    }
                }
            }
        } else {
            unreachable!()
        }
    }

    intersection_prototypes
        .into_iter()
        .chain(lane_prototypes)
        .collect()
}

pub fn render_preview(
    result_preview: &PlanResult,
    renderer_id: RendererID,
    scene_id: usize,
    frame: usize,
    world: &mut World,
) {
    let mut lane_geometry = Geometry::empty();
    let mut intersection_geometry = Geometry::empty();

    for (i, prototype) in result_preview.prototypes.iter().enumerate() {
        match *prototype {
            Prototype::Road(RoadPrototype::Lane(LanePrototype(ref lane_path, _))) => {
                lane_geometry +=
                    band_to_geometry(&Band::new(lane_path.clone(), LANE_WIDTH * 0.7), 0.1);


            }
            Prototype::Road(RoadPrototype::Intersection(IntersectionPrototype {
                                                            ref shape,
                                                            ref connecting_lanes,
                                                            ..
                                                        })) => {
                intersection_geometry +=
                    band_to_geometry(&Band::new(shape.outline().clone(), 0.1), 0.1);

                for &LanePrototype(ref lane_path, ref timings) in
                    connecting_lanes.values().flat_map(|lanes| lanes)
                {
                    if timings[(frame / 10) % timings.len()] {
                        lane_geometry +=
                            band_to_geometry(&Band::new(lane_path.clone(), LANE_WIDTH * 0.1), 0.1);
                    }
                }
            }
            _ => {}
        }
    }

    renderer_id.update_individual(
        scene_id,
        18_000,
        lane_geometry,
        Instance::with_color(colors::STROKE_BASE),
        true,
        world,
    );

    renderer_id.update_individual(
        scene_id,
        18_001,
        intersection_geometry,
        Instance::with_color(colors::SELECTION_STROKE),
        true,
        world,
    );
}