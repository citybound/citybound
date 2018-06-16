use compact::CVec;
use descartes::{P2, V2, Segment, Area, Path, FiniteCurve, WithUniqueOrthogonal,
PointContainer, AreaError};
use land_use::buildings::BuildingStyle;
use itertools::Itertools;

use transport::transport_planning::RoadPrototype;

use planning::{Plan, PlanResult, Prototype, GestureIntent, Version};

pub mod interaction;

#[derive(Compact, Clone)]
pub enum ZoneIntent {
    LandUse(LandUse),
    MaxHeight(u8),
    SetBack(u8),
}

#[derive(Copy, Clone, PartialEq)]
pub enum LandUse {
    Residential,
    Commercial,
    Industrial,
    Agricultural,
    Recreational,
    Official,
}

#[derive(Compact, Clone)]
pub struct Lot {
    pub area: Area,
    pub land_uses: CVec<LandUse>,
    pub max_height: u8,
    pub set_back: u8,
    pub connection_points: CVec<(P2, V2)>,
}

impl Lot {
    pub fn center_point(&self) -> P2 {
        let outline = &self.area.primitives[0].boundary;
        P2::from_coordinates(
            (0..10u8)
                .into_iter()
                .fold(V2::new(0.0, 0.0), |sum_point, i| {
                    sum_point + outline.along(f32::from(i) * outline.length() / 10.0).coords
                }) / 10.0,
        )
    }
}

#[derive(Compact, Clone)]
pub struct BuildingIntent {
    pub lot: Lot,
    pub building_style: BuildingStyle,
}

#[derive(Compact, Clone)]
pub struct LotPrototype {
    pub lot: Lot,
    pub occupancy: LotOccupancy,
    pub based_on: Version,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum LotOccupancy {
    Vacant,
    Occupied(BuildingStyle),
}

pub fn calculate_prototypes(
    plan: &Plan,
    current_result: &PlanResult,
    based_on: Version,
) -> Result<Vec<Prototype>, AreaError> {
    let paved_area_areas = current_result
        .prototypes
        .values()
        .filter_map(|prototype| {
            if let Prototype::Road(RoadPrototype::PavedArea(ref shape)) = *prototype {
                Some(shape)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let building_prototypes = plan
        .gestures
        .values()
        .map(|gesture| {
            if let GestureIntent::Building(BuildingIntent {
                ref lot,
                building_style,
            }) = gesture.intent
            {
                let mut area = lot.area.clone();

                for paved_area_shape in &paved_area_areas {
                    let split = area.split(&paved_area_shape);

                    match split.a_minus_b() {
                        Ok(pieces) => if let Some(main_piece) = pieces
                            .disjoint()
                            .into_iter()
                            .find(|piece| piece.contains(lot.center_point()))
                        {
                            area = main_piece;
                        } else {
                            println!("No piece contains center");
                            return Ok(None);
                        },
                        Err(err) => return Err(err),
                    }
                }

                Ok(Some(Prototype::Lot(LotPrototype {
                    lot: Lot {
                        area,
                        ..lot.clone()
                    },
                    occupancy: LotOccupancy::Occupied(building_style),
                    based_on,
                })))
            } else {
                Ok(None)
            }
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter_map(|maybe_proto| maybe_proto)
        .collect::<Vec<_>>();

    let vacant_lot_prototypes = {
        let building_areas = plan
        .gestures
        .values()
        .filter_map(|gesture| {
            if let GestureIntent::Building(BuildingIntent {
                lot: Lot { ref area, .. },..
            }) = gesture.intent
            {
                Some(area)
            } else {
                None
            }
        }).collect::<Vec<_>>();

        let mut land_use_areas = plan
            .gestures
            .values()
            .filter_map(|gesture| {
                if let GestureIntent::Zone(ZoneIntent::LandUse(land_use)) = gesture.intent {
                    Some((land_use, &gesture.points))
                } else {
                    None
                }
            })
            .filter_map(|(land_use, points)| {
                Some((
                    land_use,
                    Area::new_simple(
                        Path::new(
                            points
                                .iter()
                                .chain(points.first())
                                .tuple_windows()
                                .filter_map(|(start, end)| Segment::line(*start, *end))
                                .collect(),
                        ).ok()?,
                    ).ok()?,
                ))
            })
            .collect::<Vec<_>>();

        let paved_or_built_area = paved_area_areas
            .clone()
            .into_iter()
            .chain(building_areas)
            .try_fold(Area::new(CVec::new()), |union, piece| {
                union.split(piece).union()
            })?;

        land_use_areas = land_use_areas
            .into_iter()
            .map(|(land_use, shape)| {
                shape
                    .split(&paved_or_built_area)
                    .a_minus_b()
                    .map(|cut_shapes| {
                        cut_shapes
                            .disjoint()
                            .into_iter()
                            .map(|cut_shape| (land_use, cut_shape))
                            .collect::<Vec<_>>()
                    })
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flat_map(|areas| areas)
            .collect();

        land_use_areas
            .into_iter()
            .filter_map(|(land_use, area)| {
                let connection_points = area.primitives[0]
                    .boundary
                    .segments
                    .iter()
                    .flat_map(|segment| {
                        let length = segment.length();
                        (&[0.25, 0.5, 0.75])
                            .iter()
                            .map(|ratio| {
                                (
                                    segment.along(length * ratio),
                                    -segment.direction_along(length * ratio).orthogonal(),
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .filter(|&(point, _dir)| {
                        // TODO: this is a horribly slow way to find connection points
                        paved_area_areas
                            .iter()
                            .any(|paved_area| paved_area.contains(point))
                    })
                    .collect::<CVec<_>>();

                if connection_points.is_empty() {
                    println!("No connection point found");
                    None
                } else {
                    Some(Prototype::Lot(LotPrototype {
                        lot: Lot {
                            land_uses: vec![land_use].into(),
                            max_height: 0,
                            set_back: 0,
                            connection_points,
                            area,
                        },
                        occupancy: LotOccupancy::Vacant,
                        based_on,
                    }))
                }
            })
            .collect::<Vec<_>>()
    };

    Ok(vacant_lot_prototypes
        .into_iter()
        .chain(building_prototypes)
        .collect())
}
