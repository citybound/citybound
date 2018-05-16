use kay::{World, Fate, ActorSystem};
use descartes::{P2, V2, Shape, SimpleShape, WithUniqueOrthogonal, Path, FiniteCurve, Segment};
use descartes::clipper;
use stagemaster::geometry::{CShape, CPath, add_debug_path};
use ordered_float::OrderedFloat;

use land_use::zone_planning::{Lot, BuildingIntent};
use land_use::buildings::BuildingStyle;
use land_use::buildings::architecture::ideal_lot_shape;
use economy::immigration_and_development::DevelopmentManagerID;
use itertools::{Itertools, MinMaxResult};

use construction::{ConstructionID, Constructable, ConstructableID};
use planning::{Prototype, Version};

#[derive(Compact, Clone)]
pub struct VacantLot {
    pub id: VacantLotID,
    pub lot: Lot,
    based_on: Version,
}

impl Lot {
    pub fn width_height_per_connection_point(&self) -> Vec<(P2, V2, f32, f32)> {
        let midpoints = self.shape
            .outline()
            .segments()
            .iter()
            .map(|segment| segment.midpoint())
            .collect::<Vec<_>>();

        self.connection_points
            .iter()
            .map(|&(point, direction)| {
                let length_direction = direction;
                let width_direction = length_direction.orthogonal();

                let length = if let MinMaxResult::MinMax(front, back) =
                    midpoints
                        .iter()
                        .map(|midpoint| {
                            OrderedFloat((*midpoint - point).dot(&length_direction))
                        })
                        .minmax()
                {
                    *back - *front
                } else {
                    0.0
                };

                let width = if let MinMaxResult::MinMax(left, right) =
                    midpoints
                        .iter()
                        .map(|midpoint| {
                            OrderedFloat((*midpoint - point).dot(&width_direction))
                        })
                        .minmax()
                {
                    *right - *left
                } else {
                    0.0
                };

                (point, direction, width, length)
            })
            .collect()
    }

    pub fn split_for(&self, building_style: BuildingStyle, world: &mut World) -> Option<Lot> {
        let needed_shape = ideal_lot_shape(building_style);
        let width_height_per_connection_point = self.width_height_per_connection_point();

        let maybe_suitable_connection_point = width_height_per_connection_point.iter().find(
            |&&(_point, _direction, width, height)| {

                println!(
                    "Trying to suggest lot for {:?}. Is: {:?} Needed: {:?}",
                    building_style,
                    (width, height),
                    needed_shape
                );

                let width_ratio = width / needed_shape.0;
                let length_ratio = height / needed_shape.1;

                width_ratio > 0.5 && width_ratio < 2.0 && length_ratio > 0.5 && length_ratio < 2.0
            },
        );

        if let Some(&(point, direction, _, _)) = maybe_suitable_connection_point {
            // keep only the connection point for the building that matches its shape
            Some(Lot {
                connection_points: vec![(point, direction)].into(),
                ..self.clone()
            })
        } else {
            let maybe_too_wide_connection_points = width_height_per_connection_point
                .iter()
                .filter(|&&(_point, _direction, width, height)| {
                    let width_ratio = width / needed_shape.0;
                    let length_ratio = height / needed_shape.1;

                    width_ratio > 2.0 && length_ratio > 0.5 && length_ratio < 2.0
                });

            maybe_too_wide_connection_points
                .filter_map(|&(point, direction, _, _)| {
                    let orthogonal = direction.orthogonal();

                    let corners = (
                        point + 500.0 * direction,
                        point + 500.0 * direction + 1_000.0 * orthogonal,
                        point - 500.0 * direction + 1_000.0 * orthogonal,
                        point - 500.0 * direction,
                    );

                    let splitting_shape = CShape::new(
                        CPath::new(vec![
                            Segment::line(corners.0, corners.1).unwrap(),
                            Segment::line(corners.1, corners.2).unwrap(),
                            Segment::line(corners.2, corners.3).unwrap(),
                            Segment::line(corners.3, corners.0).unwrap(),
                        ]).unwrap(),
                    ).unwrap();

                    // add_debug_path(
                    //     splitting_shape.outline().clone(),
                    //     [1.0, 0.0, 0.0],
                    //     0.5,
                    //     world,
                    // );

                    println!("Attempting width split");

                    clipper::clip(clipper::Mode::Intersection, &self.shape, &splitting_shape)
                        .ok()
                        .into_iter()
                        .chain(
                            clipper::clip(clipper::Mode::Difference, &self.shape, &splitting_shape)
                                .ok(),
                        )
                        .flat_map(|sub_results| sub_results)
                        .filter_map(|split_shape| {
                            // add_debug_path(
                            //     split_shape.outline().clone(),
                            //     [0.0, 0.1, 0.0],
                            //     0.7,
                            //     world,
                            // );

                            let split_lot = Lot {
                                connection_points: self.connection_points
                                    .clone()
                                    .into_iter()
                                    .filter(|&(other_point, _)| {
                                        point != other_point && split_shape.contains(other_point)
                                    })
                                    .collect(),
                                shape: split_shape,
                                ..self.clone()
                            };

                            // recurse!
                            println!("Got split lot, checking suitability");
                            split_lot.split_for(building_style, world)
                        })
                        .next()
                })
                .next()
        }
    }
}

impl VacantLot {
    pub fn spawn(id: VacantLotID, lot: &Lot, based_on: Version, _world: &mut World) -> VacantLot {
        VacantLot { id, based_on, lot: lot.clone() }
    }

    pub fn suggest_lot(
        &mut self,
        building_style: BuildingStyle,
        requester: DevelopmentManagerID,
        world: &mut World,
    ) {
        println!("Trying suggest");
        if let Some(suitable_lot) = self.lot.split_for(building_style, world) {
            requester.on_suggested_lot(
                BuildingIntent { lot: suitable_lot, building_style },
                self.based_on,
                world,
            )
        }
    }
}

impl Constructable for VacantLot {
    fn morph(&mut self, _: &Prototype, _report_to: ConstructionID, _world: &mut World) {
        unreachable!()
    }

    fn destruct(&mut self, report_to: ConstructionID, world: &mut World) -> Fate {
        report_to.action_done(self.id.into(), world);
        Fate::Die
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<VacantLot>();

    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
