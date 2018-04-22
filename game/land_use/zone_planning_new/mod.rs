use compact::CVec;
use descartes::{Segment, clipper, SimpleShape, Path};
use stagemaster::geometry::{CShape, CPath};
use land_use::buildings::architecture::BuildingStyle;
use itertools::Itertools;

use transport::transport_planning_new::RoadPrototype;

use planning_new::{Plan, PlanResult, Prototype, GestureIntent};

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
pub struct LotIntent {
    original_shape: CShape,
    current_shape: CShape,
}

#[derive(Compact, Clone)]
pub struct LotPrototype {
    shape: CShape,
    land_uses: CVec<LandUse>,
    max_height: u8,
    set_back: u8,
    occupancy: LotOccupancy,
}

#[derive(Copy, Clone)]
pub enum LotOccupancy {
    Vacant,
    Occupied { building_style: BuildingStyle },
}

pub fn calculate_prototypes(plan: &Plan, current_result: &PlanResult) -> Vec<Prototype> {
    let paved_area_shapes = current_result.prototypes.iter().filter_map(|prototype| {
        if let Prototype::Road(RoadPrototype::PavedArea(ref shape)) = *prototype {
            Some(shape)
        } else {
            None
        }
    });

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

    for paved_area_shape in paved_area_shapes {
        land_use_shapes = land_use_shapes
            .into_iter()
            .flat_map(|(land_use, shape)| {
                clipper::clip(clipper::Mode::Difference, &shape, paved_area_shape)
                    .ok()
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

    land_use_shapes
        .into_iter()
        .map(|(land_use, shape)| {
            Prototype::Lot(LotPrototype {
                land_uses: vec![land_use].into(),
                shape,
                max_height: 0,
                set_back: 0,
                occupancy: LotOccupancy::Vacant,
            })
        })
        .collect()
}