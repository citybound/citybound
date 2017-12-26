use compact::CVec;
use kay::{ActorSystem, World, Fate, Actor, TypedID};
use descartes::{N, P2, Band, Curve, FiniteCurve, Path, RoughlyComparable, Intersect,
                WithUniqueOrthogonal};
use itertools::Itertools;
use stagemaster::geometry::CPath;
use ordered_float::OrderedFloat;

use super::lane::{Lane, LaneID, TransferLane, TransferLaneID};
use super::lane::connectivity::{Interaction, InteractionKind, OverlapKind};
use super::microtraffic::LaneLikeID;

use planning::materialized_reality::MaterializedRealityID;
use super::planning::materialized_roads::BuildableRef;

const CONNECTION_TOLERANCE: f32 = 0.1;

#[derive(Compact, Clone)]
pub struct ConstructionInfo {
    pub length: f32,
    pub path: CPath,
    pub progress: f32,
    unbuilding_for: Option<MaterializedRealityID>,
    disconnects_remaining: u8,
}

impl ConstructionInfo {
    pub fn from_path(path: CPath) -> Self {
        ConstructionInfo {
            length: path.length(),
            path: path,
            progress: 0.0,
            unbuilding_for: None,
            disconnects_remaining: 0,
        }
    }
}

pub trait Unbuildable {
    fn disconnect(&mut self, other_id: UnbuildableID, world: &mut World);
    fn unbuild(&mut self, report_to: MaterializedRealityID, world: &mut World) -> Fate;
    fn on_confirm_disconnect(&mut self, world: &mut World) -> Fate;
}

use fnv::FnvHashMap;
use std::cell::UnsafeCell;
thread_local! (
    static MEMOIZED_BANDS_OUTLINES: UnsafeCell<
        FnvHashMap<LaneLikeID, (Band<CPath>, CPath)>
        > = UnsafeCell::new(FnvHashMap::default());
);

impl Lane {
    pub fn start_connecting_and_report(
        &mut self,
        report_to: MaterializedRealityID,
        report_as: BuildableRef,
        world: &mut World,
    ) {
        Lane::global_broadcast(world).connect(
            self.id,
            self.construction.path.start(),
            self.construction.path.end(),
            self.construction.path.length(),
            true,
            world,
        );
        TransferLane::global_broadcast(world)
            .connect_transfer_to_normal(self.id, self.construction.path.clone(), world);
        report_to.on_lane_built(self.id_as(), report_as, world);
    }

    pub fn start_connecting_overlaps(&mut self, lanes: &CVec<LaneID>, world: &mut World) {
        for &lane_id in lanes.iter() {
            lane_id.connect_overlaps(self.id, self.construction.path.clone(), true, world);
        }
    }

    pub fn connect(
        &mut self,
        other_id: LaneID,
        other_start: P2,
        other_end: P2,
        other_length: N,
        reply_needed: bool,
        world: &mut World,
    ) {
        if other_id == self.id {
            return;
        };

        let mut connected = false;

        if other_start.is_roughly_within(self.construction.path.end(), CONNECTION_TOLERANCE) {
            connected = true;

            let already_a_partner = self.connectivity.interactions.iter().any(|interaction| {
                match *interaction {
                    Interaction {
                        partner_lane,
                        kind: InteractionKind::Next { .. },
                        ..
                    } => partner_lane == other_id.into(),
                    _ => false,
                }
            });
            if !already_a_partner {
                self.connectivity.interactions.push(Interaction {
                    partner_lane: other_id.into(),
                    start: self.construction.length,
                    partner_start: 0.0,
                    kind: InteractionKind::Next { green: false },
                });
            }

            super::pathfinding::on_connect(self);
        }

        if other_end.is_roughly_within(self.construction.path.start(), CONNECTION_TOLERANCE) {
            connected = true;

            let already_a_partner = self.connectivity.interactions.iter().any(|interaction| {
                match *interaction {
                    Interaction {
                        partner_lane,
                        kind: InteractionKind::Previous { .. },
                        ..
                    } => partner_lane == other_id.into(),
                    _ => false,
                }
            });
            if !already_a_partner {
                self.connectivity.interactions.push(Interaction {
                    partner_lane: other_id.into(),
                    start: 0.0,
                    partner_start: other_length,
                    kind: InteractionKind::Previous,
                });
            }

            super::pathfinding::on_connect(self);
        }

        if reply_needed && connected {
            let path = &self.construction.path;
            other_id.connect(
                self.id,
                path.start(),
                path.end(),
                path.length(),
                false,
                world,
            );
        }
    }

    pub fn connect_overlaps(
        &mut self,
        other_id: LaneID,
        other_path: &CPath,
        reply_needed: bool,
        world: &mut World,
    ) {
        MEMOIZED_BANDS_OUTLINES.with(|memoized_bands_outlines_cell| {
            let memoized_bands_outlines = unsafe { &mut *memoized_bands_outlines_cell.get() };
            let &(ref lane_band, ref lane_outline) = memoized_bands_outlines
                .entry(self.id_as())
                .or_insert_with(|| {
                    let band = Band::new(self.construction.path.clone(), 4.5);
                    let outline = band.outline();
                    (band, outline)
                }) as &(Band<CPath>, CPath);

            let memoized_bands_outlines = unsafe { &mut *memoized_bands_outlines_cell.get() };
            let &(ref other_band, ref other_outline) = memoized_bands_outlines
                .entry(other_id.into())
                .or_insert_with(|| {
                    let band = Band::new(other_path.clone(), 4.5);
                    let outline = band.outline();
                    (band, outline)
                }) as &(Band<CPath>, CPath);

            let intersections = (lane_outline, other_outline).intersect();
            if intersections.len() >= 2 {
                if let ::itertools::MinMaxResult::MinMax((entry_intersection, entry_distance),
                                                         (exit_intersection, exit_distance)) =
                    intersections
                        .iter()
                        .map(|intersection| {
                            (
                                intersection,
                                lane_band.outline_distance_to_path_distance(intersection.along_a),
                            )
                        })
                        .minmax_by_key(|&(_, distance)| OrderedFloat(distance))
                {
                    let other_entry_distance =
                        other_band.outline_distance_to_path_distance(entry_intersection.along_b);
                    let other_exit_distance =
                        other_band.outline_distance_to_path_distance(exit_intersection.along_b);

                    let overlap_kind = if other_path
                        .direction_along(other_entry_distance)
                        .is_roughly_within(
                            self.construction.path.direction_along(entry_distance),
                            0.1,
                        ) ||
                        other_path
                            .direction_along(other_exit_distance)
                            .is_roughly_within(
                                self.construction.path.direction_along(exit_distance),
                                0.1,
                            )
                    {
                        // ::stagemaster::geometry::CPath::add_debug_path(
                        //     self.construction.path
                        //         .subsection(entry_distance, exit_distance).unwrap(),
                        //     [1.0, 0.5, 0.0],
                        //     0.3
                        // );
                        OverlapKind::Parallel
                    } else {
                        // ::stagemaster::geometry::CPath::add_debug_path(
                        //     self.construction.path
                        //         .subsection(entry_distance, exit_distance).unwrap(),
                        //     [1.0, 0.0, 0.0],
                        //     0.3
                        // );
                        OverlapKind::Conflicting
                    };

                    self.connectivity.interactions.push(Interaction {
                        partner_lane: other_id.into(),
                        start: entry_distance,
                        partner_start: other_entry_distance.min(other_exit_distance),
                        kind: InteractionKind::Overlap {
                            end: exit_distance,
                            partner_end: other_exit_distance.max(other_entry_distance),
                            kind: overlap_kind,
                        },
                    });
                } else {
                    panic!("both entry and exit should exist")
                }
            }


            if reply_needed {
                other_id.connect_overlaps(self.id, self.construction.path.clone(), false, world);
            }
        });
    }

    pub fn connect_to_transfer(&mut self, other_id: TransferLaneID, world: &mut World) {
        other_id.connect_transfer_to_normal(self.id, self.construction.path.clone(), world);
    }

    pub fn add_transfer_lane_interaction(&mut self, interaction: Interaction, _: &mut World) {
        let already_a_partner = self.connectivity.interactions.iter().any(|existing| {
            existing.partner_lane == interaction.partner_lane
        });
        if !already_a_partner {
            self.connectivity.interactions.push(interaction);
            super::pathfinding::on_connect(self);
        }
    }
}

use transport::pathfinding::trip::{TripResult, TripFate};

impl Unbuildable for Lane {
    fn disconnect(&mut self, other_id: UnbuildableID, world: &mut World) {
        let interaction_indices_to_remove = self.connectivity
            .interactions
            .iter()
            .enumerate()
            // TODO: ugly: untyped RawID shenanigans
            .filter_map(|(i, inter)| if inter.partner_lane.as_raw() == other_id.as_raw() {
                Some(i)
            } else {
                None
            })
            .collect::<Vec<_>>();

        let self_as_rough_location = self.id_as();
        self.microtraffic.cars.retain(
            |car| if let Some(hop_interaction) =
                car.next_hop_interaction
            {
                if interaction_indices_to_remove.contains(&(hop_interaction as usize)) {
                    car.trip.finish(
                        TripResult {
                            location_now: Some(self_as_rough_location),
                            fate: TripFate::HopDisconnected,
                        },
                        world,
                    );
                    false
                } else {
                    true
                }
            } else {
                true
            },
        );
        self.microtraffic.obstacles.retain(|&(_obstacle, from_id)| {
            // TODO: ugly: untyped RawID shenanigans
            from_id.as_raw() != other_id.as_raw()
        });
        for idx in interaction_indices_to_remove.into_iter().rev() {
            self.connectivity.interactions.remove(idx);
        }
        // TODO: untyped RawID shenanigans
        let other_as_lanelike = unsafe { LaneLikeID::from_raw(other_id.as_raw()) };
        super::pathfinding::on_disconnect(self, other_as_lanelike);
        other_id.on_confirm_disconnect(world);
    }

    fn unbuild(&mut self, report_to: MaterializedRealityID, world: &mut World) -> Fate {
        let mut disconnects_remaining = 0;
        for id in self.connectivity
            .interactions
            .iter()
            .map(|interaction| interaction.partner_lane)
            .unique()
        {
            // TODO: untyped RawID shenanigans
            let id_as_unbuildable = unsafe { UnbuildableID::from_raw(id.as_raw()) };
            id_as_unbuildable.disconnect(self.id_as(), world);
            disconnects_remaining += 1;
        }
        super::rendering::on_unbuild(self, world);
        MEMOIZED_BANDS_OUTLINES.with(|memoized_bands_outlines_cell| {
            let memoized_bands_outlines = unsafe { &mut *memoized_bands_outlines_cell.get() };
            memoized_bands_outlines.remove(&self.id_as())
        });
        if disconnects_remaining == 0 {
            self.finalize(report_to, world);
            Fate::Die
        } else {
            self.construction.disconnects_remaining = disconnects_remaining;
            self.construction.unbuilding_for = Some(report_to);
            Fate::Live
        }
    }

    fn on_confirm_disconnect(&mut self, world: &mut World) -> Fate {
        self.construction.disconnects_remaining -= 1;
        if self.construction.disconnects_remaining == 0 {
            self.finalize(
                self.construction.unbuilding_for.expect(
                    "should be unbuilding",
                ),
                world,
            );
            Fate::Die
        } else {
            Fate::Live
        }
    }
}

impl Lane {
    fn finalize(&self, report_to: MaterializedRealityID, world: &mut World) {
        report_to.on_lane_unbuilt(self.id_as(), world);

        for car in &self.microtraffic.cars {
            car.trip.finish(
                TripResult {
                    location_now: None,
                    fate: TripFate::LaneUnbuilt,
                },
                world,
            );
        }

        super::pathfinding::on_unbuild(self, world);
    }
}

use land_use::buildings::{Lot, BuildingID, BuildingSpawnerID, MIN_LANE_BUILDING_DISTANCE,
                          MIN_ROAD_LENGTH_TO_TOWN};
use rand::Rng;
use transport::pathfinding::PreciseLocation;

impl Lane {
    // TODO: this is a horrible hack
    pub fn find_lot(&mut self, requester: BuildingSpawnerID, world: &mut World) {
        const BUILDING_DISTANCE: f32 = 16.0;

        if let Some(location) = self.pathfinding.location {
            if !self.connectivity.on_intersection {
                let path = &self.construction.path;
                let mut offset = ::rand::thread_rng().next_f32() * path.length();
                let mut position = path.along(offset) +
                    (1.0 + ::rand::thread_rng().next_f32() * 1.0) * BUILDING_DISTANCE *
                        path.direction_along(offset).orthogonal();

                // hacky: spawn location for neighboring towns: only at road ends
                if offset > MIN_ROAD_LENGTH_TO_TOWN {
                    offset = path.length();
                    position = path.end() + 150.0 * path.end_direction()
                }

                let orientation = path.direction_along(offset);

                requester.found_lot(
                    Lot {
                        position,
                        orientation,
                        location: Some(PreciseLocation { location, offset: offset }),
                        adjacent_lane_position: path.along(offset),
                    },
                    world,
                );
            }
        }
    }

    pub fn try_reconnect_building(
        &mut self,
        building: BuildingID,
        lot_position: P2,
        world: &mut World,
    ) {
        if let Some(location) = self.pathfinding.location {
            if !self.connectivity.on_intersection {
                let path = &self.construction.path;
                let distance = path.distance_to(lot_position);

                if distance >= MIN_LANE_BUILDING_DISTANCE &&
                    distance <= 1.7 * MIN_LANE_BUILDING_DISTANCE
                {
                    if let Some(offset) = path.project_with_max_distance(
                        lot_position,
                        1.7 * MIN_LANE_BUILDING_DISTANCE,
                        0.5,
                    )
                    {
                        building.reconnect(
                            PreciseLocation { location, offset },
                            path.along(offset),
                            world,
                        );
                    }
                }
            }
        }
    }
}

impl TransferLane {
    pub fn start_connecting_and_report(
        &mut self,
        report_to: MaterializedRealityID,
        report_as: BuildableRef,
        world: &mut World,
    ) {
        Lane::global_broadcast(world).connect_to_transfer(self.id, world);
        report_to.on_lane_built(self.id_as(), report_as, world);
        super::rendering::on_build_transfer(self, world);
    }

    pub fn connect_transfer_to_normal(
        &mut self,
        other_id: LaneID,
        other_path: &CPath,
        world: &mut World,
    ) {
        let projections = (
            other_path.project(self.construction.path.start()),
            other_path.project(self.construction.path.end()),
        );
        if let (Some(lane_start_on_other_distance), Some(lane_end_on_other_distance)) =
            projections
        {
            if lane_start_on_other_distance < lane_end_on_other_distance &&
                lane_end_on_other_distance - lane_start_on_other_distance > 6.0
            {
                let lane_start_on_other = other_path.along(lane_start_on_other_distance);
                let lane_end_on_other = other_path.along(lane_end_on_other_distance);

                if lane_start_on_other.is_roughly_within(self.construction.path.start(), 3.0) &&
                    lane_end_on_other.is_roughly_within(self.construction.path.end(), 3.0)
                {
                    other_id.add_transfer_lane_interaction(
                        Interaction {
                            partner_lane: self.id_as(),
                            start: lane_start_on_other_distance,
                            partner_start: 0.0,
                            kind: InteractionKind::Overlap {
                                end: lane_start_on_other_distance + self.construction.length,
                                partner_end: self.construction.length,
                                kind: OverlapKind::Transfer,
                            },
                        },
                        world,
                    );

                    let mut distance_covered = 0.0;
                    let distance_map = self.construction
                        .path
                        .segments()
                        .iter()
                        .map(|segment| {
                            distance_covered += segment.length();
                            let segment_end_on_other_distance =
                                other_path.project(segment.end()).expect(
                                    "should contain transfer lane segment end",
                                );
                            (
                                distance_covered,
                                segment_end_on_other_distance - lane_start_on_other_distance,
                            )
                        })
                        .collect();

                    let other_is_right =
                        (lane_start_on_other - self.construction.path.start()).dot(
                            &self.construction.path.start_direction().orthogonal(),
                        ) > 0.0;

                    if other_is_right {
                        self.connectivity.right = Some((other_id, lane_start_on_other_distance));
                        self.connectivity.right_distance_map = distance_map;
                    } else {
                        self.connectivity.left = Some((other_id, lane_start_on_other_distance));
                        self.connectivity.left_distance_map = distance_map;
                    }
                }
            }
        }
    }
}

impl Unbuildable for TransferLane {
    fn disconnect(&mut self, other_id: UnbuildableID, world: &mut World) {
        self.connectivity.left = self.connectivity.left.and_then(
            // TODO: ugly: untyped RawID shenanigans
            |(left_id, left_start)| if left_id.as_raw() ==
                other_id.as_raw()
            {
                None
            } else {
                Some((left_id, left_start))
            },
        );
        self.connectivity.right = self.connectivity.right.and_then(
            // TODO: ugly: untyped RawID shenanigans
            |(right_id, right_start)| if right_id.as_raw() ==
                other_id.as_raw()
            {
                None
            } else {
                Some((right_id, right_start))
            },
        );
        other_id.on_confirm_disconnect(world);
    }

    fn unbuild(&mut self, report_to: MaterializedRealityID, world: &mut World) -> Fate {
        if let Some((left_id, _)) = self.connectivity.left {
            Into::<UnbuildableID>::into(left_id).disconnect(self.id_as(), world);
        }
        if let Some((right_id, _)) = self.connectivity.right {
            Into::<UnbuildableID>::into(right_id).disconnect(self.id_as(), world);
        }
        super::rendering::on_unbuild_transfer(self, world);
        if self.connectivity.left.is_none() && self.connectivity.right.is_none() {
            self.finalize(report_to, world);
            Fate::Die
        } else {
            self.construction.disconnects_remaining = self.connectivity
                .left
                .into_iter()
                .chain(self.connectivity.right)
                .count() as u8;
            self.construction.unbuilding_for = Some(report_to);
            Fate::Live
        }
    }

    fn on_confirm_disconnect(&mut self, world: &mut World) -> Fate {
        self.construction.disconnects_remaining -= 1;
        if self.construction.disconnects_remaining == 0 {
            self.finalize(
                self.construction.unbuilding_for.expect(
                    "should be unbuilding",
                ),
                world,
            );
            Fate::Die
        } else {
            Fate::Live
        }
    }
}

impl TransferLane {
    fn finalize(&self, report_to: MaterializedRealityID, world: &mut World) {
        report_to.on_lane_unbuilt(self.id_as(), world);

        for car in &self.microtraffic.cars {
            car.trip.finish(
                TripResult {
                    location_now: None,
                    fate: TripFate::LaneUnbuilt,
                },
                world,
            );
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
