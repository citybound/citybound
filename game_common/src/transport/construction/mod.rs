use compact::CVec;
use kay::{ActorSystem, World, Fate, Actor, TypedID};
use descartes::{N, P2, Band, LinePath, ClosedLinePath, Segment, RoughEq, Intersect, WithUniqueOrthogonal};
use itertools::Itertools;
use ordered_float::OrderedFloat;

use super::lane::{Lane, LaneID, SwitchLane, SwitchLaneID};
use super::lane::connectivity::{Interaction, InteractionKind, OverlapKind};
use super::microtraffic::LaneLikeID;

use planning::Prototype;
use construction::{ConstructionID, Constructable, ConstructableID};
use super::transport_planning::{RoadPrototype, LanePrototype, SwitchLanePrototype,
IntersectionPrototype};

use style::dimensions::{LANE_CONNECTION_TOLERANCE, MAX_SWITCHING_LANE_DISTANCE,
MIN_SWITCHING_LANE_LENGTH};

impl RoadPrototype {
    pub fn construct(&self, report_to: ConstructionID, world: &mut World) -> CVec<ConstructableID> {
        match *self {
            RoadPrototype::Lane(LanePrototype(ref path, _)) => vec![
                LaneID::spawn_and_connect(path.clone(), false, CVec::new(), report_to, world)
                    .into(),
            ].into(),
            RoadPrototype::SwitchLane(SwitchLanePrototype(ref path)) => {
                vec![SwitchLaneID::spawn_and_connect(path.clone(), report_to, world).into()].into()
            }
            RoadPrototype::Intersection(IntersectionPrototype {
                ref connecting_lanes,
                ..
            }) => {
                let ids = connecting_lanes
                    .values()
                    .flat_map(|group| {
                        group
                            .iter()
                            .map(|&LanePrototype(ref path, ref timings)| {
                                LaneID::spawn_and_connect(
                                    path.clone(),
                                    true,
                                    timings.clone(),
                                    report_to,
                                    world,
                                )
                            }).collect::<Vec<_>>()
                    }).collect::<Vec<_>>();

                for id in &ids {
                    id.start_connecting_overlaps(
                        ids.iter().filter(|&other| other != id).cloned().collect(),
                        world,
                    )
                }

                ids.into_iter().map(|lane_id| lane_id.into()).collect()
            }
            RoadPrototype::PavedArea(_) => CVec::new(),
        }
    }
}

impl Constructable for Lane {
    fn morph(&mut self, _new_prototype: &Prototype, report_to: ConstructionID, world: &mut World) {
        report_to.action_done(self.id_as(), world);
    }
    fn destruct(&mut self, report_to: ConstructionID, world: &mut World) -> Fate {
        self.unbuild(report_to, world);
        Fate::Live
    }
}

impl Constructable for SwitchLane {
    fn morph(&mut self, _new_prototype: &Prototype, report_to: ConstructionID, world: &mut World) {
        report_to.action_done(self.id_as(), world);
    }
    fn destruct(&mut self, report_to: ConstructionID, world: &mut World) -> Fate {
        self.unbuild(report_to, world);
        Fate::Live
    }
}

#[derive(Compact, Clone)]
pub struct ConstructionInfo {
    pub length: f32,
    pub path: LinePath,
    pub progress: f32,
    unbuilding_for: Option<ConstructionID>,
    disconnects_remaining: u8,
}

impl ConstructionInfo {
    pub fn from_path(path: LinePath) -> Self {
        ConstructionInfo {
            length: path.length(),
            path,
            progress: 0.0,
            unbuilding_for: None,
            disconnects_remaining: 0,
        }
    }
}

pub trait Unbuildable {
    fn disconnect(&mut self, other_id: UnbuildableID, world: &mut World);
    fn unbuild(&mut self, report_to: ConstructionID, world: &mut World) -> Fate;
    fn on_confirm_disconnect(&mut self, world: &mut World) -> Fate;
}

use fnv::FnvHashMap;

// TODO: not thread safe for now
static mut MEMOIZED_BANDS_OUTLINES: Option<FnvHashMap<LaneLikeID, (Band, ClosedLinePath)>> = None;

impl Lane {
    pub fn spawn_and_connect(
        id: LaneID,
        path: &LinePath,
        on_intersection: bool,
        timings: &CVec<bool>,
        report_to: ConstructionID,
        world: &mut World,
    ) -> Lane {
        Lane::global_broadcast(world).connect(
            id,
            path.start(),
            path.end(),
            path.length(),
            true,
            world,
        );
        if !on_intersection {
            SwitchLane::global_broadcast(world).connect_switch_to_normal(id, path.clone(), world);
        }
        report_to.action_done(id.into(), world);
        Lane::spawn(id, path, on_intersection, timings, world)
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

        if other_start.rough_eq_by(self.construction.path.end(), LANE_CONNECTION_TOLERANCE) {
            connected = true;

            let already_a_partner =
                self.connectivity
                    .interactions
                    .iter()
                    .any(|interaction| match *interaction {
                        Interaction {
                            partner_lane,
                            kind: InteractionKind::Next { .. },
                            ..
                        } => partner_lane == other_id.into(),
                        _ => false,
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

        if other_end.rough_eq_by(self.construction.path.start(), LANE_CONNECTION_TOLERANCE) {
            connected = true;

            let already_a_partner =
                self.connectivity
                    .interactions
                    .iter()
                    .any(|interaction| match *interaction {
                        Interaction {
                            partner_lane,
                            kind: InteractionKind::Previous { .. },
                            ..
                        } => partner_lane == other_id.into(),
                        _ => false,
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
        other_path: &LinePath,
        reply_needed: bool,
        world: &mut World,
    ) {
        let &(ref lane_band, ref lane_outline) = unsafe {
            MEMOIZED_BANDS_OUTLINES
                .get_or_insert_with(FnvHashMap::default)
                .entry(self.id_as())
                .or_insert_with(|| {
                    let band = Band::new(self.construction.path.clone(), 4.5);
                    let outline = band.outline();
                    (band, outline)
                }) as &(Band, ClosedLinePath)
        };

        let &(ref other_band, ref other_outline) = unsafe {
            MEMOIZED_BANDS_OUTLINES
                .get_or_insert_with(FnvHashMap::default)
                .entry(other_id.into())
                .or_insert_with(|| {
                    let band = Band::new(other_path.clone(), 4.5);
                    let outline = band.outline();
                    (band, outline)
                }) as &(Band, ClosedLinePath)
        };

        let intersections = (lane_outline, other_outline).intersect();
        if intersections.len() >= 2 {
            if let ::itertools::MinMaxResult::MinMax(
                (entry_intersection, entry_distance),
                (exit_intersection, exit_distance),
            ) = intersections
                .iter()
                .map(|intersection| {
                    (
                        intersection,
                        lane_band.outline_distance_to_path_distance(intersection.along_a),
                    )
                }).minmax_by_key(|&(_, distance)| OrderedFloat(distance))
            {
                let other_entry_distance =
                    other_band.outline_distance_to_path_distance(entry_intersection.along_b);
                let other_exit_distance =
                    other_band.outline_distance_to_path_distance(exit_intersection.along_b);

                let overlap_kind = if other_path
                    .direction_along(other_entry_distance)
                    .rough_eq_by(self.construction.path.direction_along(entry_distance), 0.1)
                    || other_path
                        .direction_along(other_exit_distance)
                        .rough_eq_by(self.construction.path.direction_along(exit_distance), 0.1)
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
    }

    pub fn connect_to_switch(&mut self, other_id: SwitchLaneID, world: &mut World) {
        other_id.connect_switch_to_normal(self.id, self.construction.path.clone(), world);
    }

    pub fn add_switch_lane_interaction(&mut self, interaction: Interaction, _: &mut World) {
        let already_a_partner = self
            .connectivity
            .interactions
            .iter()
            .any(|existing| existing.partner_lane == interaction.partner_lane);
        if !already_a_partner {
            self.connectivity.interactions.push(interaction);
            super::pathfinding::on_connect(self);
        }
    }
}

use transport::pathfinding::trip::{TripResult, TripFate};

impl Unbuildable for Lane {
    fn disconnect(&mut self, other_id: UnbuildableID, world: &mut World) {
        // TODO: ugly: untyped RawID shenanigans
        let interaction_indices_to_remove = self
            .connectivity
            .interactions
            .iter()
            .enumerate()
            .filter_map(|(i, inter)| {
                if inter.partner_lane.as_raw() == other_id.as_raw() {
                    Some(i)
                } else {
                    None
                }
            }).collect::<Vec<_>>();

        let self_as_rough_location = self.id_as();
        self.microtraffic.cars.retain(|car| {
            if let Some(hop_interaction) = car.next_hop_interaction {
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
            }
        });
        self.microtraffic.obstacles.retain(|&(_obstacle, from_id)| {
            // TODO: ugly: untyped RawID shenanigans
            from_id.as_raw() != other_id.as_raw()
        });
        for idx in interaction_indices_to_remove.into_iter().rev() {
            self.connectivity.interactions.remove(idx);
        }
        // TODO: untyped RawID shenanigans
        let other_as_lanelike = LaneLikeID::from_raw(other_id.as_raw());
        super::pathfinding::on_disconnect(self, other_as_lanelike);
        other_id.on_confirm_disconnect(world);
    }

    fn unbuild(&mut self, report_to: ConstructionID, world: &mut World) -> Fate {
        let mut disconnects_remaining = 0;
        for id in self
            .connectivity
            .interactions
            .iter()
            .map(|interaction| interaction.partner_lane)
            .unique()
        {
            // TODO: untyped RawID shenanigans
            let id_as_unbuildable = UnbuildableID::from_raw(id.as_raw());
            id_as_unbuildable.disconnect(self.id_as(), world);
            disconnects_remaining += 1;
        }
        super::rendering::on_unbuild(self, world);
        unsafe {
            MEMOIZED_BANDS_OUTLINES
                .get_or_insert_with(FnvHashMap::default)
                .remove(&self.id_as());
        }
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
                self.construction
                    .unbuilding_for
                    .expect("should be unbuilding"),
                world,
            );
            Fate::Die
        } else {
            Fate::Live
        }
    }
}

impl Lane {
    fn finalize(&self, report_to: ConstructionID, world: &mut World) {
        report_to.action_done(self.id_as(), world);

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

use land_use::buildings::{BuildingID};
use style::dimensions::LANE_DISTANCE;
use transport::pathfinding::PreciseLocation;

impl Lane {
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

                println!(
                    "Building {:?} lane {:?} distance: {}",
                    building, self.id, distance
                );

                if distance <= 3.0 * LANE_DISTANCE {
                    if let Some((offset, projected_point)) =
                        path.project_with_max_distance(lot_position, 0.5, 3.0 * LANE_DISTANCE)
                    {
                        println!("Projected: {}", offset);
                        building.reconnect(
                            PreciseLocation { location, offset },
                            projected_point,
                            world,
                        );
                    }
                }
            }
        }
    }
}

impl SwitchLane {
    pub fn spawn_and_connect(
        id: SwitchLaneID,
        path: &LinePath,
        report_to: ConstructionID,
        world: &mut World,
    ) -> SwitchLane {
        Lane::global_broadcast(world).connect_to_switch(id, world);

        let lane = SwitchLane::spawn(id, path, world);
        super::rendering::on_build_switch(&lane, world);

        report_to.action_done(id.into(), world);

        lane
    }

    pub fn connect_switch_to_normal(
        &mut self,
        other_id: LaneID,
        other_path: &LinePath,
        world: &mut World,
    ) {
        let projections = (
            other_path.project_with_max_distance(
                self.construction.path.start(),
                MAX_SWITCHING_LANE_DISTANCE,
                MAX_SWITCHING_LANE_DISTANCE,
            ),
            other_path.project_with_max_distance(
                self.construction.path.end(),
                MAX_SWITCHING_LANE_DISTANCE,
                MAX_SWITCHING_LANE_DISTANCE,
            ),
        );
        if let (
            Some((lane_start_on_other_distance, lane_start_on_other)),
            Some((lane_end_on_other_distance, lane_end_on_other)),
        ) = projections
        {
            if lane_start_on_other_distance < lane_end_on_other_distance
                && lane_end_on_other_distance - lane_start_on_other_distance
                    > MIN_SWITCHING_LANE_LENGTH
                && lane_start_on_other
                    .rough_eq_by(self.construction.path.start(), MAX_SWITCHING_LANE_DISTANCE)
                && lane_end_on_other.rough_eq_by(self.construction.path.end(), 3.0)
            {
                other_id.add_switch_lane_interaction(
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
                let distance_map = self
                    .construction
                    .path
                    .segments()
                    .map(|segment| {
                        distance_covered += segment.length();
                        let segment_end_on_other_distance = other_path
                            .project_with_tolerance(segment.end(), MAX_SWITCHING_LANE_DISTANCE)
                            .expect("should contain switch lane segment end")
                            .0;
                        (
                            distance_covered,
                            segment_end_on_other_distance - lane_start_on_other_distance,
                        )
                    }).collect();

                let other_is_right = (lane_start_on_other - self.construction.path.start())
                    .dot(&self.construction.path.start_direction().orthogonal_right())
                    > 0.0;

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

impl Unbuildable for SwitchLane {
    fn disconnect(&mut self, other_id: UnbuildableID, world: &mut World) {
        self.connectivity.left = self.connectivity.left.and_then(
            // TODO: ugly: untyped RawID shenanigans
            |(left_id, left_start)| {
                if left_id.as_raw() == other_id.as_raw() {
                    None
                } else {
                    Some((left_id, left_start))
                }
            },
        );
        self.connectivity.right = self.connectivity.right.and_then(
            // TODO: ugly: untyped RawID shenanigans
            |(right_id, right_start)| {
                if right_id.as_raw() == other_id.as_raw() {
                    None
                } else {
                    Some((right_id, right_start))
                }
            },
        );
        other_id.on_confirm_disconnect(world);
    }

    fn unbuild(&mut self, report_to: ConstructionID, world: &mut World) -> Fate {
        if let Some((left_id, _)) = self.connectivity.left {
            Into::<UnbuildableID>::into(left_id).disconnect(self.id_as(), world);
        }
        if let Some((right_id, _)) = self.connectivity.right {
            Into::<UnbuildableID>::into(right_id).disconnect(self.id_as(), world);
        }
        super::rendering::on_unbuild_switch(self, world);
        if self.connectivity.left.is_none() && self.connectivity.right.is_none() {
            self.finalize(report_to, world);
            Fate::Die
        } else {
            self.construction.disconnects_remaining = self
                .connectivity
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
                self.construction
                    .unbuilding_for
                    .expect("should be unbuilding"),
                world,
            );
            Fate::Die
        } else {
            Fate::Live
        }
    }
}

impl SwitchLane {
    fn finalize(&self, report_to: ConstructionID, world: &mut World) {
        report_to.action_done(self.id_as(), world);

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
