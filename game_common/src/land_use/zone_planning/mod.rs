use compact::CVec;
use descartes::{P2, V2, Area, ClosedLinePath, LinePath, PointContainer, AreaError};
use land_use::buildings::BuildingStyle;

use transport::transport_planning::RoadPrototype;

use planning::{PlanHistory, VersionedGesture, PlanResult, Prototype, PrototypeID,
PrototypeKind, GestureIntent, StepID};

#[derive(Compact, Clone, Serialize, Deserialize)]
pub enum ZoneIntent {
    LandUse(LandUse),
    MaxHeight(u8),
    SetBack(u8),
}

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum LandUse {
    Residential,
    Commercial,
    Industrial,
    Agricultural,
    Recreational,
    Official,
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub struct Lot {
    pub area: Area,
    pub land_uses: CVec<LandUse>,
    pub max_height: u8,
    pub set_back: u8,
    pub connection_points: CVec<(P2, V2)>,
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
}

#[derive(Compact, Clone, Serialize, Deserialize)]
pub struct BuildingIntent {
    pub lot: Lot,
    pub building_style: BuildingStyle,
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub struct LotPrototype {
    pub lot: Lot,
    pub occupancy: LotOccupancy,
    pub based_on: StepID,
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
                        based_on: history.latest_step_id(),
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
                    Area::new_simple(ClosedLinePath::new(LinePath::new(
                        points.iter().chain(points.first()).cloned().collect(),
                    )?)?),
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
                let connection_points = area.primitives[0]
                    .boundary
                    .path()
                    .segments()
                    .flat_map(|segment| {
                        let length = segment.length();
                        (&[0.25, 0.5, 0.75])
                            .iter()
                            .map(|ratio| (segment.along(length * ratio), -segment.direction()))
                            .collect::<Vec<_>>()
                    }).filter(|&(point, _dir)| {
                        // TODO: this is a horribly slow way to find connection points
                        paved_area_areas
                            .iter()
                            .any(|(paved_area, _)| paved_area.contains(point))
                    }).collect::<CVec<_>>();

                if connection_points.is_empty() {
                    println!("No connection point found");
                    None
                } else {
                    Some(Prototype {
                        kind: PrototypeKind::Lot(LotPrototype {
                            lot: Lot {
                                land_uses: vec![land_use].into(),
                                max_height: 0,
                                set_back: 0,
                                connection_points,
                                area,
                            },
                            occupancy: LotOccupancy::Vacant,
                            based_on: history.latest_step_id(),
                        }),
                        id,
                    })
                }
            }).collect::<Vec<_>>()
    };

    Ok(vacant_lot_prototypes
        .into_iter()
        .chain(building_prototypes)
        .collect())
}
