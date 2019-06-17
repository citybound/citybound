use compact::{CHashMap, CVec};
use descartes::{N, P2, V2, Band, LinePath, ClosedLinePath, Area, Intersect, WithUniqueOrthogonal,
RoughEq, PointContainer, AreaError, ArcOrLineSegment, Segment, AreaEmbedding, AreaFilter};
use ordered_float::OrderedFloat;

use cb_planning::{VersionedGesture, StepID, PrototypeID, PlanHistory, PlanResult,
Prototype, GestureID};
use planning::{CBPrototypeKind, CBGestureIntent};

mod intersection_connections;
pub mod smooth_path;
use dimensions::{LANE_DISTANCE, CENTER_LANE_DISTANCE, MIN_SWITCHING_LANE_LENGTH,
SWITCHING_LANE_OVERLAP_TOLERANCE};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct RoadIntent {
    pub n_lanes_forward: u8,
    pub n_lanes_backward: u8,
}

impl RoadIntent {
    pub fn new(n_lanes_forward: u8, n_lanes_backward: u8) -> Self {
        RoadIntent {
            n_lanes_forward,
            n_lanes_backward,
        }
    }
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
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

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
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

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
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

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct ConnectionRole {
    straight: bool,
    u_turn: bool,
    inner_turn: bool,
    outer_turn: bool,
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
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

#[derive(Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct GestureSideID(i16);

impl GestureSideID {
    pub fn new_forward(gesture_idx: usize) -> Self {
        GestureSideID((gesture_idx + 1) as i16)
    }

    pub fn new_backward(gesture_idx: usize) -> Self {
        GestureSideID(-((gesture_idx + 1) as i16))
    }
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
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

pub fn simplify_road_path(points: CVec<P2>) -> CVec<P2> {
    smooth_path::smooth_path_from(&points)
        .map(|path| {
            path.segments()
                .flat_map(|segment| match segment {
                    ArcOrLineSegment::Line(line) => vec![line.start()],
                    ArcOrLineSegment::Arc(arc) => vec![arc.start(), arc.apex()],
                })
                .chain(points.last().cloned())
                .collect()
        })
        .unwrap_or(points)
}

pub fn gesture_intent_smooth_paths(
    history: &PlanHistory<CBGestureIntent>,
) -> Vec<(GestureID, StepID, RoadIntent, LinePath)> {
    history
        .gestures
        .pairs()
        .filter_map(
            |(gesture_id, VersionedGesture(gesture, step_id))| match gesture.intent {
                CBGestureIntent::Road(ref road_intent) if gesture.points.len() >= 2 => {
                    smooth_path::smooth_path_from(&gesture.points).map(|path| {
                        (
                            *gesture_id,
                            *step_id,
                            *road_intent,
                            path.to_line_path_with_max_angle(0.12),
                        )
                    })
                }
                _ => None,
            },
        )
        .collect::<Vec<_>>()
}

#[allow(clippy::cognitive_complexity)]
pub fn calculate_prototypes(
    history: &PlanHistory<CBGestureIntent>,
    _current_result: &PlanResult<CBPrototypeKind>,
) -> Result<Vec<Prototype<CBPrototypeKind>>, AreaError> {
    let gesture_intent_smooth_paths = gesture_intent_smooth_paths(history);

    let gesture_areas_for_intersection = gesture_intent_smooth_paths
        .iter()
        .map(|&(gesture_id, step_id, road_intent, ref path)| {
            (
                Band::new_asymmetric(
                    path.clone(),
                    f32::from(road_intent.n_lanes_backward) * LANE_DISTANCE
                        + if road_intent.n_lanes_backward > 0 {
                            1.2 * LANE_DISTANCE
                        } else {
                            0.4 * LANE_DISTANCE
                        },
                    f32::from(road_intent.n_lanes_forward) * LANE_DISTANCE
                        + if road_intent.n_lanes_forward > 0 {
                            1.2 * LANE_DISTANCE
                        } else {
                            0.4 * LANE_DISTANCE
                        },
                )
                .as_area(),
                gesture_id,
                step_id,
            )
        })
        .collect::<Vec<_>>();

    let mut road_intersection_embedding = AreaEmbedding::new(15.0);

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    enum RoadPart {
        StartCap,
        Road,
        EndCap,
    }

    for (gesture_area, gesture_id, step_id) in &gesture_areas_for_intersection {
        road_intersection_embedding.insert(
            gesture_area.clone(),
            (*gesture_id, *step_id, RoadPart::Road),
        );
    }

    // add intersections at the starts and ends of gestures
    const ROAD_CAP_DEPTH: N = 15.0;

    let road_caps = gesture_intent_smooth_paths.iter().flat_map(
        |&(gesture_id, step_id, road_intent, ref path)| {
            [
                (path.start(), path.start_direction(), RoadPart::StartCap),
                (path.end(), path.end_direction(), RoadPart::EndCap),
            ]
            .iter()
            .map(|&(point, direction, role)| {
                let orthogonal = direction.orthogonal_right();
                let half_depth = direction * ROAD_CAP_DEPTH / 2.0;
                let width_backward = orthogonal
                    * (f32::from(road_intent.n_lanes_backward) * LANE_DISTANCE
                        + 0.4 * LANE_DISTANCE);
                let width_forward = orthogonal
                    * (f32::from(road_intent.n_lanes_forward) * LANE_DISTANCE
                        + 0.4 * LANE_DISTANCE);
                (
                    Area::new_simple(
                        ClosedLinePath::new(
                            LinePath::new(
                                vec![
                                    point - half_depth - width_backward,
                                    point + half_depth - width_backward,
                                    point + half_depth + width_forward,
                                    point - half_depth + width_forward,
                                    point - half_depth - width_backward,
                                ]
                                .into(),
                            )
                            .expect("End intersection path should be valid"),
                        )
                        .expect("End intersection path should be closed"),
                    ),
                    (gesture_id, step_id, role),
                )
            })
            .collect::<Vec<_>>()
        },
    );

    for (road_cap_area, road_cap_label) in road_caps {
        road_intersection_embedding.insert(road_cap_area, road_cap_label);
    }

    let mut intersection_prototypes: Vec<_> = road_intersection_embedding
        .view(AreaFilter::Function(Box::new(|labels| labels.len() >= 2)))
        .get_areas_with_pieces()?
        .into_iter()
        .map(|(area, pieces)| {
            let mut influenced_id = PrototypeID::from_influences(
                pieces
                    .iter()
                    .map(|(_piece, label)| label.own_right_label)
                    .collect::<Vec<_>>(),
            );
            influenced_id = influenced_id.add_influences(vec![
                pieces[0].0.start().x.to_bits(),
                pieces[0].0.start().y.to_bits(),
            ]);
            Prototype {
                representative_position: area.primitives[0].boundary.path().points[0],
                kind: CBPrototypeKind::Road(RoadPrototype::Intersection(IntersectionPrototype {
                    area,
                    incoming: CHashMap::new(),
                    outgoing: CHashMap::new(),
                    connecting_lanes: CHashMap::new(),
                })),
                id: influenced_id,
            }
        })
        .collect();

    let intersected_lane_paths = {
        let raw_lane_paths = gesture_intent_smooth_paths
            .iter()
            .enumerate()
            .flat_map(
                |(gesture_i, &(gesture_id, step_id, road_intent, ref path))| {
                    (0..road_intent.n_lanes_forward)
                        .map(|lane_i| {
                            (
                                CENTER_LANE_DISTANCE / 2.0 + f32::from(lane_i) * LANE_DISTANCE,
                                lane_i as i8 + 1,
                            )
                        })
                        .chain((0..road_intent.n_lanes_backward).map(|lane_i| {
                            (
                                -(CENTER_LANE_DISTANCE / 2.0 + f32::from(lane_i) * LANE_DISTANCE),
                                -(lane_i as i8) - 1,
                            )
                        }))
                        .filter_map(|(offset, offset_i)| {
                            path.shift_orthogonally(offset).map(|path| {
                                (
                                    if offset < 0.0 {
                                        GestureSideID::new_backward(gesture_i)
                                    } else {
                                        GestureSideID::new_forward(gesture_i)
                                    },
                                    PrototypeID::from_influences((gesture_id, step_id, offset_i)),
                                    if offset < 0.0 { path.reverse() } else { path },
                                )
                            })
                        })
                        .collect::<Vec<_>>()
                },
            )
            .collect::<Vec<_>>();

        raw_lane_paths
            .into_iter()
            .flat_map(|(gesture_side_id, lane_influence_id, raw_lane_path)| {
                let mut start_trim = 0.0f32;
                let mut start_influence = lane_influence_id;
                let mut end_trim = raw_lane_path.length();
                let mut end_influence = lane_influence_id;
                let mut cuts = Vec::new();

                use ::planning::CBPrototypeKind::Road;

                for prototype in &mut intersection_prototypes {
                    if let Prototype {
                        id: intersection_id,
                        kind: Road(RoadPrototype::Intersection(ref mut intersection)),
                        ..
                    } = prototype
                    {
                        let points = (
                            &raw_lane_path,
                            intersection.area.primitives[0].boundary.path(),
                        )
                            .intersect();

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
                            cuts.push((*entry_distance, *exit_distance, *intersection_id));
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
                                if exit_distance > start_trim {
                                    start_trim = exit_distance;
                                    start_influence = *intersection_id;
                                }
                            } else if intersection.area.contains(raw_lane_path.end()) {
                                let entry_distance = points[0].along_a;
                                intersection.incoming.push_at(
                                    gesture_side_id,
                                    IntersectionConnector::new(
                                        raw_lane_path.along(entry_distance),
                                        raw_lane_path.direction_along(entry_distance),
                                    ),
                                );
                                if entry_distance < end_trim {
                                    end_trim = entry_distance;
                                    end_influence = *intersection_id;
                                }
                            }
                        }
                    } else {
                        unreachable!()
                    }
                }

                cuts.sort_by(|a, b| OrderedFloat(a.0).cmp(&OrderedFloat(b.0)));

                cuts.insert(0, (-1.0, start_trim, start_influence));
                cuts.push((end_trim, raw_lane_path.length() + 1.0, end_influence));

                cuts.windows(2)
                    .filter_map(|two_cuts| {
                        let (
                            (_, exit_distance, exit_influence),
                            (entry_distance, _, entry_influence),
                        ) = (two_cuts[0], two_cuts[1]);
                        let subsection_id =
                            lane_influence_id.add_influences((exit_influence, entry_influence));
                        raw_lane_path
                            .subsection(exit_distance, entry_distance)
                            .map(|subsection| (subsection, subsection_id))
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };

    let switch_lane_paths = {
        #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
        enum SwitchLaneLabel {
            Left(PrototypeID),
            Right(PrototypeID),
        };

        let mut switch_lane_embedding = AreaEmbedding::new(30.0);

        let right_lane_bands = intersected_lane_paths.iter().filter_map(|(path, id)| {
            path.shift_orthogonally(0.5 * LANE_DISTANCE + 0.5 * SWITCHING_LANE_OVERLAP_TOLERANCE)
                .map(|right_path| {
                    let band =
                        Band::new(right_path.clone(), SWITCHING_LANE_OVERLAP_TOLERANCE * 2.0);
                    (band.as_area(), *id)
                })
        });

        for (band_area, id) in right_lane_bands {
            switch_lane_embedding.insert(band_area, SwitchLaneLabel::Right(id))
        }

        let left_lane_bands = intersected_lane_paths.iter().filter_map(|(path, id)| {
            path.shift_orthogonally(-0.5 * LANE_DISTANCE - 0.5 * SWITCHING_LANE_OVERLAP_TOLERANCE)
                .map(|left_path| {
                    let band = Band::new(left_path.clone(), SWITCHING_LANE_OVERLAP_TOLERANCE * 2.0);
                    (band.as_area(), *id)
                })
        });

        for (band_area, id) in left_lane_bands {
            switch_lane_embedding.insert(band_area, SwitchLaneLabel::Left(id))
        }

        switch_lane_embedding
            .view(AreaFilter::Function(Box::new(|labels| {
                labels.iter().any(|label| {
                    if let SwitchLaneLabel::Left(_) = label {
                        true
                    } else {
                        false
                    }
                }) && labels.iter().any(|label| {
                    if let SwitchLaneLabel::Right(_) = label {
                        true
                    } else {
                        false
                    }
                })
            })))
            .get_unique_pieces()
            .into_iter()
            .filter_map(|(piece, piece_area_label)| {
                if let SwitchLaneLabel::Right(own_id) = piece_area_label.own_right_label {
                    if piece.length() > MIN_SWITCHING_LANE_LENGTH {
                        let mut influenced_id = PrototypeID::from_influences(own_id);
                        influenced_id = influenced_id.add_influences(
                            piece_area_label.left_labels.iter().collect::<Vec<_>>(),
                        );
                        influenced_id = influenced_id.add_influences(
                            piece_area_label.right_labels.iter().collect::<Vec<_>>(),
                        );
                        influenced_id = influenced_id.add_influences(piece.points[0].x.to_bits());
                        influenced_id = influenced_id.add_influences(piece.points[0].y.to_bits());
                        Some((piece, influenced_id))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
    };

    for prototype in &mut intersection_prototypes {
        if let CBPrototypeKind::Road(RoadPrototype::Intersection(ref mut intersection)) =
            prototype.kind
        {
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
                .map(|(path, id)| Prototype {
                    representative_position: path.points[0],
                    kind: CBPrototypeKind::Road(RoadPrototype::Lane(LanePrototype(
                        path,
                        CVec::new(),
                    ))),
                    id,
                }),
        )
        .chain(switch_lane_paths.map(|(path, id)| Prototype {
            representative_position: path.points[0],
            kind: CBPrototypeKind::Road(RoadPrototype::SwitchLane(SwitchLanePrototype(path))),
            id,
        }))
        .chain(
            gesture_areas_for_intersection
                .into_iter()
                .map(|(area, gesture_id, step_id)| Prototype {
                    representative_position: area.primitives[0].boundary.path().points[0],
                    kind: CBPrototypeKind::Road(RoadPrototype::PavedArea(area)),
                    id: PrototypeID::from_influences((gesture_id, step_id)),
                }),
        )
        .collect())
}
