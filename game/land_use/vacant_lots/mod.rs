use kay::{World, Fate, ActorSystem};
use descartes::{P2, V2, Area, WithUniqueOrthogonal, Path, FiniteCurve, Segment, PointContainer};
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
    pub fn width_depth_per_connection_point(&self) -> Vec<(P2, V2, f32, f32)> {
        let midpoints = self.area
            .primitives
            .iter()
            .flat_map(|primitive| {
                primitive.boundary.segments.iter().map(|segment| {
                    segment.midpoint()
                })
            })
            .collect::<Vec<_>>();

        self.connection_points
            .iter()
            .map(|&(point, direction)| {
                let depth_direction = direction;
                let width_direction = depth_direction.orthogonal();

                let depth = if let MinMaxResult::MinMax(front, back) =
                    midpoints
                        .iter()
                        .map(|midpoint| {
                            OrderedFloat((*midpoint - point).dot(&depth_direction))
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

                (point, direction, width, depth)
            })
            .collect()
    }

    pub fn split_for(
        &self,
        building_style: BuildingStyle,
        allow_left_split: bool,
        allow_right_split: bool,
    ) -> Option<Lot> {
        let needed_shape = ideal_lot_shape(building_style);
        let width_depth_per_connection_point = self.width_depth_per_connection_point();

        let maybe_suitable_connection_point = width_depth_per_connection_point.iter().find(
            |&&(_point, _direction, width, depth)| {

                println!(
                    "Trying to suggest lot for {:?}. Is: {:?} Needed: {:?}",
                    building_style,
                    (width, depth),
                    needed_shape
                );

                let width_ratio = width / needed_shape.0;
                let depth_ratio = depth / needed_shape.1;

                width_ratio > 0.5 && width_ratio < 2.0 && depth_ratio > 0.5 && depth_ratio < 2.0
            },
        );

        if let Some(&(point, direction, _, _)) = maybe_suitable_connection_point {
            // keep only the connection point for the building that matches its shape
            Some(Lot {
                connection_points: vec![(point, direction)].into(),
                ..self.clone()
            })
        } else {
            let maybe_too_wide_connection_points = width_depth_per_connection_point.iter().filter(
                |&&(_point, _direction, width, depth)| {
                    let width_ratio = width / needed_shape.0;
                    let depth_ratio = depth / needed_shape.1;

                    width_ratio > 2.0 && depth_ratio > 0.5 && depth_ratio < 2.0
                },
            );

            maybe_too_wide_connection_points
                .filter_map(|&(point, direction, _, _)| {
                    let orthogonal = direction.orthogonal();

                    let corners = (
                        point + needed_shape.1 * direction,
                        point + needed_shape.1 * direction + 1_000.0 * orthogonal,
                        point - 500.0 * direction + 1_000.0 * orthogonal,
                        point - 500.0 * direction,
                    );

                    let splitting_area = Area::new_simple(
                        Path::new(
                            vec![
                                Segment::line(corners.0, corners.1).unwrap(),
                                Segment::line(corners.1, corners.2).unwrap(),
                                Segment::line(corners.2, corners.3).unwrap(),
                                Segment::line(corners.3, corners.0).unwrap(),
                            ].into(),
                        ).unwrap(),
                    ).unwrap();

                    println!("Attempting width split");

                    let split = self.area.split(&splitting_area);

                    if allow_right_split {
                        split
                            .intersection()
                            .disjoint()
                            .into_iter()
                            .filter_map(|right_split| {
                                let split_lot = Lot {
                                    connection_points: self.connection_points
                                        .clone()
                                        .into_iter()
                                        .filter(|&(other_point, _)| {
                                            point != other_point &&
                                                right_split.contains(other_point)
                                        })
                                        .collect(),
                                    area: right_split,
                                    ..self.clone()
                                };

                                // recurse!
                                println!("Got right split lot, checking suitability");
                                split_lot.split_for(building_style, false, true)
                            })
                            .next()
                    } else if allow_left_split {
                        split
                            .a_minus_b()
                            .disjoint()
                            .into_iter()
                            .filter_map(|left_split| {
                                let split_lot = Lot {
                                    connection_points: self.connection_points
                                        .clone()
                                        .into_iter()
                                        .filter(|&(other_point, _)| {
                                            point != other_point && left_split.contains(other_point)
                                        })
                                        .collect(),
                                    area: left_split,
                                    ..self.clone()
                                };

                                // recurse!
                                println!("Got right split lot, checking suitability");
                                split_lot.split_for(building_style, true, false)
                            })
                            .next()
                    } else {
                        None
                    }
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
        if let Some(suitable_lot) = self.lot.split_for(building_style, true, true) {
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
