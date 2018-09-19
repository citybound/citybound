use compact::CVec;
use descartes::{P2, V2, Area, ClosedLinePath, LinePath, PointContainer,
AreaError, WithUniqueOrthogonal};
use land_use::buildings::BuildingStyle;
use ordered_float::OrderedFloat;

use transport::transport_planning::{RoadPrototype, LanePrototype};

use planning::{PlanHistory, VersionedGesture, PlanResult, Prototype, PrototypeID,
PrototypeKind, GestureIntent};

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub enum ZoneIntent {
    LandUse(LandUse),
    MaxHeight(u8),
    SetBack(u8),
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum LandUse {
    Residential,
    Commercial,
    Industrial,
    Agricultural,
    Recreational,
    Official,
}

impl ::std::fmt::Display for LandUse {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Debug::fmt(self, f)
    }
}

pub const LAND_USES: [LandUse; 6] = [
    LandUse::Residential,
    LandUse::Commercial,
    LandUse::Industrial,
    LandUse::Agricultural,
    LandUse::Recreational,
    LandUse::Official,
];

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub struct Lot {
    pub area: Area,
    pub land_uses: CVec<LandUse>,
    pub max_height: u8,
    pub set_back: u8,
    pub road_boundaries: CVec<LinePath>,
}

impl Lot {
    pub fn center_point(&self) -> P2 {
        let outline = &self.area.primitives[0].boundary.path();
        P2::from_coordinates(
            (0..10).into_iter().fold(V2::new(0.0, 0.0), |sum_point, i| {
                sum_point + outline.along(i as f32 * (outline.length() / 10.0)).coords
            }) / 10.0,
        )
    }

    pub fn best_road_connection(&self) -> (P2, V2) {
        let longest_boundary = self
            .road_boundaries
            .iter()
            .max_by_key(|path| OrderedFloat(path.length()))
            .expect("Should always have a boundary");
        let length = longest_boundary.length();
        (
            longest_boundary.along(length / 2.0),
            -longest_boundary.direction_along(length / 2.0).orthogonal(),
        )
    }

    pub fn all_road_connections(&self) -> Vec<(P2, V2)> {
        self.road_boundaries
            .iter()
            .map(|boundary| {
                let length = boundary.length();
                (
                    boundary.along(length / 2.0),
                    -boundary.direction_along(length / 2.0).orthogonal(),
                )
            }).collect()
    }
}

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub struct BuildingIntent {
    pub lot: Lot,
    pub building_style: BuildingStyle,
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub struct LotPrototype {
    pub lot: Lot,
    pub occupancy: LotOccupancy,
}

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum LotOccupancy {
    Vacant,
    Occupied(BuildingStyle),
}

pub fn calculate_prototypes(
    history: &PlanHistory,
    current_result: &PlanResult,
) -> Result<Vec<Prototype>, AreaError> {
    let paved_area_areas = current_result
        .prototypes
        .values()
        .filter_map(|prototype| {
            if let Prototype {
                kind: PrototypeKind::Road(RoadPrototype::PavedArea(ref shape)),
                id,
            } = *prototype
            {
                Some((shape, id))
            } else {
                None
            }
        }).collect::<Vec<_>>();

    let building_prototypes = history
        .gestures
        .values()
        .map(|VersionedGesture(gesture, gesture_step_id)| {
            if let GestureIntent::Building(BuildingIntent {
                ref lot,
                building_style,
            }) = gesture.intent
            {
                let mut area = lot.area.clone();
                let mut influenced_id = PrototypeID::from_influences(gesture_step_id);

                for (paved_area_shape, paved_id) in &paved_area_areas {
                    let (has_split, maybe_main_piece) = {
                        if let Some(split) = area.split_if_intersects(&paved_area_shape) {
                            (
                                true,
                                split
                                    .a_minus_b()?
                                    .disjoint()
                                    .into_iter()
                                    .find(|piece| piece.contains(lot.center_point())),
                            )
                        } else {
                            (false, None)
                        }
                    };

                    if has_split {
                        if let Some(main_piece) = maybe_main_piece {
                            area = main_piece;
                            influenced_id = influenced_id.add_influences(paved_id)
                        } else {
                            println!("No piece contains center");
                            return Ok(None);
                        }
                    }
                }

                Ok(Some(Prototype {
                    kind: PrototypeKind::Lot(LotPrototype {
                        lot: Lot {
                            area,
                            ..lot.clone()
                        },
                        occupancy: LotOccupancy::Occupied(building_style),
                    }),
                    id: influenced_id,
                }))
            } else {
                Ok(None)
            }
        }).collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter_map(|maybe_proto| maybe_proto)
        .collect::<Vec<_>>();

    let mut neighboring_town_distance_per_octant = vec![
        (0.0, None),
        (0.0, None),
        (0.0, None),
        (0.0, None),
        (0.0, None),
        (0.0, None),
        (0.0, None),
        (0.0, None),
    ];

    for prototype in current_result.prototypes.values() {
        if let PrototypeKind::Road(RoadPrototype::Lane(LanePrototype(ref path, _))) = prototype.kind
        {
            let distance = (path.start() - P2::new(0.0, 0.0)).norm();
            if distance > 300.0 {
                let (x, y) = (path.start().x, path.start().y);
                let octant = if x > 0.0 {
                    if y > 0.0 {
                        if x > y {
                            0
                        } else {
                            1
                        }
                    } else if x > y {
                        2
                    } else {
                        3
                    }
                } else if y > 0.0 {
                    if x > y {
                        4
                    } else {
                        5
                    }
                } else if x > y {
                    6
                } else {
                    7
                };

                if distance > neighboring_town_distance_per_octant[octant].0 {
                    let direction = path.start_direction();
                    let direction_orth = path.start_direction().orthogonal();

                    let corners: CVec<P2> = vec![
                        path.start() + 3.0 * direction_orth,
                        path.start() + 3.0 * direction_orth + 10.0 * direction,
                        path.start() + 13.0 * direction_orth + 10.0 * direction,
                        path.start() + 13.0 * direction_orth,
                        path.start() + 3.0 * direction_orth,
                    ].into();

                    if let Some(road_boundary) = LinePath::new(vec![corners[0], corners[1]].into())
                    {
                        if let Some(path) = LinePath::new(corners) {
                            if let Some(area_boundary) = ClosedLinePath::new(path) {
                                neighboring_town_distance_per_octant[octant] = (
                                    distance,
                                    Some(Prototype {
                                        kind: PrototypeKind::Lot(LotPrototype {
                                            occupancy: LotOccupancy::Occupied(
                                                BuildingStyle::NeighboringTownConnection,
                                            ),
                                            lot: Lot {
                                                road_boundaries: vec![road_boundary].into(),
                                                area: Area::new_simple(area_boundary),
                                                land_uses: CVec::new(),
                                                max_height: 0,
                                                set_back: 0,
                                            },
                                        }),
                                        id: PrototypeID::from_influences(prototype.id),
                                    }),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    let vacant_lot_prototypes = {
        let building_areas = building_prototypes
            .iter()
            .map(|prototype| {
                if let PrototypeKind::Lot(LotPrototype {
                    lot: Lot { ref area, .. },
                    ..
                }) = prototype.kind
                {
                    (area, prototype.id)
                } else {
                    unreachable!()
                }
            }).collect::<Vec<_>>();

        let mut land_use_areas = history
            .gestures
            .values()
            .filter_map(|VersionedGesture(gesture, step_id)| {
                if let GestureIntent::Zone(ZoneIntent::LandUse(land_use)) = gesture.intent {
                    Some((land_use, &gesture.points, *step_id))
                } else {
                    None
                }
            }).filter_map(|(land_use, points, step_id)| {
                Some((
                    land_use,
                    Area::new_simple(
                        ClosedLinePath::new(LinePath::new(
                            points.iter().chain(points.first()).cloned().collect(),
                        )?)?.to_clockwise(),
                    ),
                    step_id,
                ))
            }).collect::<Vec<_>>();

        let paved_or_built_areas = || paved_area_areas.iter().chain(building_areas.iter());

        let land_use_areas_influenced: Vec<(LandUse, Area, PrototypeID)> = land_use_areas
            .into_iter()
            .flat_map(|(land_use, shape, gesture_step_id)| {
                let mut shapes = vec![(shape, PrototypeID::from_influences(gesture_step_id))];

                for (paved_or_built_area, paved_id) in paved_or_built_areas() {
                    shapes = shapes
                        .into_iter()
                        .flat_map(|(shape, current_id)| {
                            if let Some(split) = shape.split_if_intersects(paved_or_built_area) {
                                split
                                    .a_minus_b()
                                    .into_iter()
                                    .flat_map(|cut_shapes| {
                                        cut_shapes.disjoint().into_iter().enumerate().map(
                                            |(i, cut_shape)| {
                                                (
                                                    cut_shape,
                                                    current_id.add_influences((paved_id, i)),
                                                )
                                            },
                                        )
                                    }).collect()
                            } else {
                                vec![(shape.clone(), current_id)]
                            }
                        }).collect()
                }

                shapes
                    .into_iter()
                    .map(|(shape, id)| (land_use, shape, id))
                    .collect::<Vec<_>>()
            }).collect();

        land_use_areas_influenced
            .into_iter()
            .filter_map(|(land_use, area, id)| {
                let road_boundary_segments = area.primitives[0]
                    .boundary
                    .path()
                    .segments()
                    .filter(|&segment| {
                        // TODO: this is a horribly slow way to find connection points
                        paved_area_areas
                            .iter()
                            .any(|(paved_area, _)| paved_area.contains(segment.midpoint()))
                    }).collect::<Vec<_>>();

                if road_boundary_segments.is_empty() {
                    println!("No road boundary found");
                    None
                } else {
                    let mut road_boundary_paths: Vec<LinePath> = road_boundary_segments
                        .into_iter()
                        .map(|segment| {
                            LinePath::new(vec![segment.start(), segment.end()].into()).unwrap()
                        }).collect();

                    let _ = ::descartes::util::join_within_vec(
                        &mut road_boundary_paths,
                        |path_a, path_b| path_a.concat(path_b).map(Some),
                    );

                    Some(Prototype {
                        kind: PrototypeKind::Lot(LotPrototype {
                            lot: Lot {
                                land_uses: vec![land_use].into(),
                                max_height: 0,
                                set_back: 0,
                                road_boundaries: road_boundary_paths.into(),
                                area,
                            },
                            occupancy: LotOccupancy::Vacant,
                        }),
                        id,
                    })
                }
            }).collect::<Vec<_>>()
    };

    Ok(vacant_lot_prototypes
        .into_iter()
        .chain(building_prototypes)
        .chain(
            neighboring_town_distance_per_octant
                .into_iter()
                .filter_map(|pair| pair.1),
        ).collect())
}
