use compact::CVec;
use descartes::{P2, V2, Area, ClosedLinePath, LinePath, PointContainer,
AreaError, WithUniqueOrthogonal, AreaEmbedding, AreaFilter};
use land_use::buildings::BuildingStyle;
use ordered_float::OrderedFloat;
use itertools::Itertools;

use transport::transport_planning::{RoadPrototype, LanePrototype};

use planning::{PlanHistory, VersionedGesture, PlanResult, Prototype, PrototypeID,
PrototypeKind, GestureIntent, GestureID, StepID};

#[derive(Compact, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
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
            -longest_boundary
                .direction_along(length / 2.0)
                .orthogonal_right(),
        )
    }

    pub fn all_road_connections(&self) -> Vec<(P2, V2)> {
        self.road_boundaries
            .iter()
            .map(|boundary| {
                let length = boundary.length();
                (
                    boundary.along(length / 2.0),
                    -boundary.direction_along(length / 2.0).orthogonal_right(),
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
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    enum ZoneEmbeddingLabel {
        Paved(PrototypeID),
        Building(GestureID, StepID),
        Zone(ZoneIntent, GestureID, StepID),
    };

    let mut zone_embedding = AreaEmbedding::new(30.0);

    for prototype in current_result.prototypes.values() {
        if let Prototype {
            kind: PrototypeKind::Road(RoadPrototype::PavedArea(ref area)),
            id,
        } = *prototype
        {
            zone_embedding.insert(area.clone(), ZoneEmbeddingLabel::Paved(id))
        }
    }

    for (gesture_id, VersionedGesture(gesture, step_id)) in history.gestures.pairs() {
        if let GestureIntent::Building(BuildingIntent { ref lot, .. }) = gesture.intent {
            zone_embedding.insert(
                lot.area.clone(),
                ZoneEmbeddingLabel::Building(*gesture_id, *step_id),
            );
        }
    }

    // see what's left of original building lots after subtracting (potentially new) paved areas
    let building_prototypes = history
        .gestures
        .pairs()
        .map(|(&gesture_id, &VersionedGesture(ref gesture, step_id))| {
            if let GestureIntent::Building(BuildingIntent {
                ref lot,
                building_style,
            }) = gesture.intent
            {
                let leftover_areas_with_pieces = zone_embedding
                    .view(
                        AreaFilter::Function(Box::new(move |labels| {
                            labels.contains(&ZoneEmbeddingLabel::Building(gesture_id, step_id))
                        })).and(AreaFilter::Function(Box::new(|labels| {
                            labels.iter().all(|label| match label {
                                ZoneEmbeddingLabel::Paved(_) => false,
                                _ => true,
                            })
                        }))),
                    ).get_areas_with_pieces()?;

                let maybe_main_area_with_pieces = leftover_areas_with_pieces
                    .into_iter()
                    .find(|(area, _pieces)| area.contains(lot.center_point()));

                if let Some((main_area, main_area_pieces)) = maybe_main_area_with_pieces {
                    let mut influenced_id = PrototypeID::from_influences(gesture_id);
                    influenced_id = influenced_id.add_influences(step_id);

                    influenced_id = influenced_id.add_influences(vec![
                        main_area_pieces[0].0.points[0].x.to_bits(),
                        main_area_pieces[0].0.points[0].y.to_bits(),
                    ]);

                    for paved_id in main_area_pieces
                        .into_iter()
                        .flat_map(|(_piece, piece_area_label)| {
                            Some(piece_area_label.own_right_label)
                                .into_iter()
                                .chain(piece_area_label.right_labels)
                        }).filter(|label| match label {
                            ZoneEmbeddingLabel::Paved(_) => true,
                            _ => false,
                        }).unique()
                    {
                        influenced_id = influenced_id.add_influences(paved_id);
                    }

                    Ok(Some(Prototype {
                        kind: PrototypeKind::Lot(LotPrototype {
                            lot: Lot {
                                area: main_area,
                                ..lot.clone()
                            },
                            occupancy: LotOccupancy::Occupied(building_style),
                        }),
                        id: influenced_id,
                    }))
                } else {
                    Ok(None)
                }
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
                    let direction_orth = path.start_direction().orthogonal_right();

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

    for (gesture_id, VersionedGesture(gesture, step_id)) in history.gestures.pairs() {
        if let GestureIntent::Zone(ref zone_intent) = gesture.intent {
            if let Some(area) = LinePath::new(
                gesture
                    .points
                    .iter()
                    .chain(gesture.points.first())
                    .cloned()
                    .collect(),
            ).and_then(|line_path| ClosedLinePath::new(line_path))
            .map(|closed_line_path| Area::new_simple(closed_line_path.to_clockwise()))
            {
                zone_embedding.insert(
                    area,
                    ZoneEmbeddingLabel::Zone(zone_intent.clone(), *gesture_id, *step_id),
                );
            }
        }
    }

    // remove paved and existing buildings to get vacant lots
    let mut vacant_lot_prototypes = vec![];

    for &land_use in &LAND_USES {
        let areas_with_pieces = zone_embedding
            .view(
                AreaFilter::Function(Box::new(move |labels| {
                    labels.iter().any(|label| match label {
                        ZoneEmbeddingLabel::Zone(ZoneIntent::LandUse(label_land_use), ..)
                            if *label_land_use == land_use =>
                        {
                            true
                        }
                        _ => false,
                    })
                })).and(AreaFilter::Function(Box::new(|labels| {
                    labels.iter().all(|label| match label {
                        ZoneEmbeddingLabel::Building(..) => false,
                        ZoneEmbeddingLabel::Paved(_) => false,
                        _ => true,
                    })
                }))),
            ).get_areas_with_pieces()?;
        for (area, pieces) in areas_with_pieces {
            let influenced_id = PrototypeID::from_influences(
                pieces
                    .iter()
                    .flat_map(|(_piece, piece_area_label)| {
                        Some(&piece_area_label.own_right_label)
                            .into_iter()
                            .chain(piece_area_label.right_labels.iter())
                    }).unique()
                    .collect::<Vec<_>>(),
            );

            let road_boundaries = pieces.into_iter().filter_map(|(piece, piece_area_label)| {
                if Some(&piece_area_label.own_right_label)
                    .into_iter()
                    .chain(piece_area_label.left_labels.iter())
                    .any(|label| match label {
                        ZoneEmbeddingLabel::Paved(_) => true,
                        _ => false,
                    }) {
                    Some(piece)
                } else {
                    None
                }
            });

            vacant_lot_prototypes.push(Prototype {
                kind: PrototypeKind::Lot(LotPrototype {
                    lot: Lot {
                        land_uses: vec![land_use].into(),
                        max_height: 0,
                        set_back: 0,
                        road_boundaries: road_boundaries.collect(),
                        area,
                    },
                    occupancy: LotOccupancy::Vacant,
                }),
                id: influenced_id,
            })
        }
    }

    Ok(vacant_lot_prototypes
        .into_iter()
        .chain(building_prototypes)
        .chain(
            neighboring_town_distance_per_octant
                .into_iter()
                .filter_map(|pair| pair.1),
        ).collect())
}
