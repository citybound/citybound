use compact::CVec;
use descartes::{P2, V2, Segment, clipper, SimpleShape, Shape, Path, FiniteCurve,
                WithUniqueOrthogonal};
use stagemaster::geometry::{CShape, CPath};
use land_use::buildings::BuildingStyle;
use itertools::Itertools;

use transport::transport_planning::RoadPrototype;

use planning::{Plan, PlanResult, Prototype, GestureIntent};

pub mod interaction;

#[derive(Compact, Clone)]
pub enum ZoneIntent {
    LandUse(LandUse),
    MaxHeight(u8),
    SetBack(u8),
}

#[derive(Copy, Clone)]
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
    pub shape: CShape,
    pub land_uses: CVec<LandUse>,
    pub max_height: u8,
    pub set_back: u8,
    pub connection_points: CVec<(P2, V2)>,
}

impl Lot {
    pub fn center_point(&self) -> P2 {
        P2::from_coordinates(
            self.shape.outline().segments().iter().fold(
                V2::new(0.0, 0.0),
                |sum_point,
                 segment| {
                    sum_point + segment.start().coords
                },
            ) / self.shape.outline().segments().len() as f32,
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
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum LotOccupancy {
    Vacant,
    Occupied(BuildingStyle),
}

pub fn calculate_prototypes(plan: &Plan, current_result: &PlanResult) -> Vec<Prototype> {
    println!("Calculating protos");
    let paved_area_shapes = current_result
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

    let building_prototypes = plan.gestures
        .values()
        .filter_map(|gesture| {
            if let GestureIntent::Building(BuildingIntent { ref lot, building_style }) =
                gesture.intent
            {
                let mut shape = lot.shape.clone();

                for paved_area_shape in &paved_area_shapes {

                    if let Ok(clip_results) = clipper::clip(
                        clipper::Mode::Difference,
                        &shape,
                        paved_area_shape,
                    )
                    {
                        if let Some(main_piece) = clip_results.into_iter().find(|piece| {
                            piece.contains(lot.center_point())
                        })
                        {
                            shape = main_piece;
                        } else {
                            println!("No piece contains center");
                            return None;
                        }
                    } else {
                        println!("Building lot clip error");
                        return None;
                    }
                }

                Some(Prototype::Lot(LotPrototype {
                    lot: Lot { shape, ..lot.clone() },
                    occupancy: LotOccupancy::Occupied(building_style),
                }))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let vacant_lot_prototypes = {
        let building_shapes = building_prototypes
            .iter()
            .filter_map(|prototype| if let Prototype::Lot(LotPrototype {
                                                  occupancy: LotOccupancy::Occupied(_),
                                                  lot: Lot { ref shape, .. },
                                              }) = *prototype
            {
                Some(shape)
            } else {
                None
            })
            .collect::<Vec<_>>();

        let mut land_use_shapes = plan.gestures
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
                    CShape::new(CPath::new(
                        points
                            .iter()
                            .chain(points.first())
                            .tuple_windows()
                            .filter_map(|(start, end)| Segment::line(*start, *end))
                            .collect(),
                    ).ok()?).ok()?,
                ))
            })
            .collect::<Vec<_>>();

        for paved_area_or_building_shape in paved_area_shapes.iter().chain(building_shapes.iter()) {
            land_use_shapes = land_use_shapes
                .into_iter()
                .flat_map(|(land_use, shape)| {
                    clipper::clip(
                        clipper::Mode::Difference,
                        &shape,
                        paved_area_or_building_shape,
                    ).ok()
                        .map(|cut_shapes| {
                            cut_shapes
                                .into_iter()
                                .map(|cut_shape| (land_use, cut_shape))
                                .collect()
                        })
                        .unwrap_or_else(Vec::new)
                })
                .collect()
        }

        println!("{} land use shapes", land_use_shapes.len());

        land_use_shapes
            .into_iter()
            .filter_map(|(land_use, shape)| {
                let connection_points = shape
                    .outline()
                    .segments()
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
                        paved_area_shapes.iter().any(|paved_area| {
                            paved_area.contains(point)
                        })
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
                            shape,
                        },
                        occupancy: LotOccupancy::Vacant,
                    }))
                }
            })
            .collect::<Vec<_>>()
    };

    vacant_lot_prototypes
        .into_iter()
        .chain(building_prototypes)
        .collect()
}
