use compact::CVec;
use descartes::{CurvedPath, Intersect, WithUniqueOrthogonal,
RoughEq};
use itertools::Itertools;
use ordered_float::OrderedFloat;

use super::{IntersectionPrototype, IntersectionConnector, ConnectionRole, LANE_DISTANCE,
LanePrototype, GestureSideID};

pub fn create_connecting_lanes(intersection: &mut IntersectionPrototype) {
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

        if ::descartes::angle_to(straight, connection_direction).abs() < STRAIGHT_ANGLE_THRESHOLD {
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
                let is_uturn = outgoing[0]
                    .position
                    .rough_eq_by(incoming[0].position, LANE_DISTANCE * 4.0)
                    && outgoing[0]
                        .direction
                        .rough_eq_by(-incoming[0].direction, 0.1);

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
                let role = role_between_groups(incoming_group, outgoing_group);
                role.inner_turn || role.u_turn
            });
            let has_straight = intersection
                .outgoing
                .values()
                .any(|outgoing_group| role_between_groups(incoming_group, outgoing_group).straight);
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
                if n_lanes < 3 || (l >= n_inner_turn_lanes && l < n_lanes - n_outer_turn_lanes) {
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
                let role = role_between_groups(incoming_group, outgoing_group);
                role.inner_turn || role.u_turn
            });
            let has_straight = intersection
                .incoming
                .values()
                .any(|incoming_group| role_between_groups(incoming_group, outgoing_group).straight);
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

            for (l, outgoing_lane) in outgoing_group.iter_mut().enumerate() {
                if l == 0 && has_inner_turn {
                    outgoing_lane.role.u_turn = true;
                }
                if l < n_inner_turn_lanes {
                    outgoing_lane.role.inner_turn = true;
                }
                if n_lanes < 3 || l >= n_inner_turn_lanes && l < n_lanes - n_outer_turn_lanes {
                    outgoing_lane.role.straight = true;
                }
                if l >= n_lanes - n_outer_turn_lanes {
                    outgoing_lane.role.outer_turn = true;
                }
            }
        }

        let connecting_lane_bundles = intersection
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
                                (role.u_turn && connector.role.u_turn)
                                    || (role.inner_turn && connector.role.inner_turn)
                                    || (role.straight && connector.role.straight)
                                    || (role.outer_turn && connector.role.outer_turn)
                            }).collect::<Vec<_>>();
                        let relevant_incoming_len = relevant_incoming_connectors.len();

                        let relevant_outgoing_connectors = outgoing_group
                            .iter()
                            .filter(|connector| {
                                (role.u_turn && connector.role.u_turn)
                                    || (role.inner_turn && connector.role.inner_turn)
                                    || (role.straight && connector.role.straight)
                                    || (role.outer_turn && connector.role.outer_turn)
                            }).collect::<Vec<_>>();
                        let relevant_outgoing_len = relevant_outgoing_connectors.len();

                        let lanes = if relevant_incoming_len > 0 && relevant_outgoing_len > 0 {
                            (0..relevant_incoming_len.max(relevant_outgoing_len))
                                .into_iter()
                                .filter_map(|l| {
                                    let start = relevant_incoming_connectors
                                        [l.min(relevant_incoming_len - 1)];
                                    let end = relevant_outgoing_connectors
                                        [l.min(relevant_outgoing_len - 1)];
                                    let path = CurvedPath::biarc(
                                        start.position,
                                        start.direction,
                                        end.position,
                                        end.direction,
                                    )?.to_line_path_with_max_angle(0.6);

                                    Some(LanePrototype(path, CVec::new()))
                                }).collect::<Vec<_>>()
                        } else {
                            vec![]
                        };

                        (
                            (role, *incoming_gesture_side_id, *outgoing_gesture_side_id),
                            lanes,
                        )
                    }).collect::<Vec<_>>()
            }).collect::<Vec<_>>();

        // find traffic light timings
        let mut phases = Vec::<(Vec<(GestureSideID, GestureSideID)>, usize)>::new();

        let mut unused_connecting_bundles = connecting_lane_bundles.clone();

        intersection.connecting_lanes = connecting_lane_bundles
            .iter()
            .map(|&((_, incoming_id, outgoing_id), ref lanes)| {
                ((incoming_id, outgoing_id), lanes.clone().into())
            }).collect();

        fn compatible(lanes_a: &[LanePrototype], lanes_b: &[LanePrototype]) -> bool {
            lanes_a.iter().cartesian_product(lanes_b).all(
                |(&LanePrototype(ref path_a, _), &LanePrototype(ref path_b, _))| {
                    path_a.start().rough_eq_by(path_b.start(), 0.1)
                        || (!path_a.end().rough_eq_by(path_b.end(), 0.1)
                            && (path_a, path_b).intersect().is_empty())
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
                        unused_connecting_bundles.retain(
                            |&((role, incoming_id, outgoing_id), ref lanes)| {
                                if role_check(role) && compatible(lanes, current_lanes) {
                                    current_lanes.extend(lanes.iter().cloned());
                                    phases[iteration].0.push((incoming_id, outgoing_id));
                                    false
                                } else {
                                    true
                                }
                            },
                        );
                    };

                if iteration % 2 == 0 {
                    // straight phase: consider nonconflicting straights,
                    // then outer, then inner/u turns
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
                    // inner phase: consider nonconflicting inner/u turns,
                    // then outer turns, then straights
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
                    // straight phase: consider nonconflicting straights,
                    // then outer, then inner/u turns
                    reuse_compatible_where(|role| role.straight, &mut current_lanes, iteration);
                    reuse_compatible_where(|role| role.outer_turn, &mut current_lanes, iteration);
                    reuse_compatible_where(
                        |role| (role.inner_turn || role.u_turn),
                        &mut current_lanes,
                        iteration,
                    );
                } else {
                    // inner phase: consider nonconflicting inner/u turns,
                    // then outer turns, then straights
                    reuse_compatible_where(
                        |role| (role.inner_turn || role.u_turn),
                        &mut current_lanes,
                        iteration,
                    );
                    reuse_compatible_where(|role| role.outer_turn, &mut current_lanes, iteration);
                    reuse_compatible_where(|role| role.straight, &mut current_lanes, iteration);
                }
            }

            phases[iteration].1 = current_lanes.len();

            iteration += 1;
        }

        for ((incoming_id, outgoing_id), ref mut lanes) in intersection.connecting_lanes.pairs_mut()
        {
            let timings: CVec<bool> = phases
                .iter()
                .flat_map(|&(ref connections_in_phase, duration)| {
                    let in_phase = connections_in_phase.contains(&(incoming_id, outgoing_id));
                    vec![in_phase; duration]
                }).collect();

            for &mut LanePrototype(_, ref mut lane_timings) in lanes.iter_mut() {
                *lane_timings = timings.clone()
            }
        }
    }
}
