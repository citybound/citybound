use compact::CVec;
use kay::{ActorSystem, World, Fate, Actor, TypedID};
use descartes::{N, P2, Band, LinePath, ClosedLinePath, Segment,
RoughEq, Intersect, WithUniqueOrthogonal};
use itertools::Itertools;
use ordered_float::OrderedFloat;

use super::lane::{Lane, LaneID, SwitchLane, SwitchLaneID};
use super::lane::connectivity::Interaction;
use super::microtraffic::LaneLikeID;

use cb_planning::Prototype;
use cb_planning::construction::{Constructable, ConstructableID};
use planning::{CBConstructionID, CBPrototypeKind};
use super::transport_planning::{RoadPrototype, LanePrototype, SwitchLanePrototype,
IntersectionPrototype};

use cb_util::log::debug;
const LOG_T: &str = "Transport Construction";

use dimensions::{LANE_CONNECTION_TOLERANCE, MAX_SWITCHING_LANE_DISTANCE,
MIN_SWITCHING_LANE_LENGTH};

impl RoadPrototype {
    pub fn construct(&self, report_to: CBConstructionID, world: &mut World) -> CVec<ConstructableID<CBPrototypeKind>> {
        match *self {
            RoadPrototype::Lane(LanePrototype(ref path, _)) => {
                vec![
                    LaneID::spawn_and_connect(path.clone(), false, CVec::new(), report_to, world)
                        .into(),
                ]
                .into()
            }
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
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();

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

impl Constructable<CBPrototypeKind> for Lane {
    fn morph(&mut self, _new_prototype: &Prototype<CBPrototypeKind>, report_to: CBConstructionID, world: &mut World) {
        report_to.action_done(self.id_as(), world);
    }
    fn destruct(&mut self, report_to: CBConstructionID, world: &mut World) -> Fate {
        self.unbuild(report_to, world);
        Fate::Live
    }
}

impl Constructable<CBPrototypeKind> for SwitchLane {
    fn morph(&mut self, _new_prototype: &Prototype<CBPrototypeKind>, report_to: CBConstructionID, world: &mut World) {
        report_to.action_done(self.id_as(), world);
    }
    fn destruct(&mut self, report_to: CBConstructionID, world: &mut World) -> Fate {
        self.unbuild(report_to, world);
        Fate::Live
    }
}

#[derive(Compact, Clone)]
pub struct ConstructionInfo {
    pub length: f32,
    pub path: LinePath,
    pub progress: f32,
    unbuilding_for: Option<CBConstructionID>,
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

use fnv::FnvHashMap;

// TODO: not thread safe for now
static mut MEMOIZED_BANDS_OUTLINES: Option<FnvHashMap<LaneLikeID, (Band, ClosedLinePath)>> = None;

impl Lane {
    pub fn spawn_and_connect(
        id: LaneID,
        path: &LinePath,
        on_intersection: bool,
        timings: &CVec<bool>,
        report_to: CBConstructionID,
        world: &mut World,
    ) -> Lane {
        LaneID::global_broadcast(world).connect(
            id,
            path.start(),
            path.end(),
            path.length(),
            true,
            world,
        );
        if !on_intersection {
            SwitchLaneID::global_broadcast(world).connect_switch_to_normal(id, path.clone(), world);
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
                        Interaction::Next { next, .. } => next == other_id,
                        _ => false,
                    });
            if !already_a_partner {
                self.connectivity.interactions.push(Interaction::Next {
                    next: other_id,
                    green: false,
                });
            }

            ::transport::pathfinding::Link::on_connect(self);
        }

        if other_end.rough_eq_by(self.construction.path.start(), LANE_CONNECTION_TOLERANCE) {
            connected = true;

            let already_a_partner =
                self.connectivity
                    .interactions
                    .iter()
                    .any(|interaction| match *interaction {
                        Interaction::Previous { previous, .. } => previous == other_id,
                        _ => false,
                    });
            if !already_a_partner {
                self.connectivity.interactions.push(Interaction::Previous {
                    previous: other_id,
                    previous_length: other_length,
                });
            }

            ::transport::pathfinding::Link::on_connect(self);
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
                })
                .minmax_by_key(|&(_, distance)| OrderedFloat(distance))
            {
                let other_entry_distance =
                    other_band.outline_distance_to_path_distance(entry_intersection.along_b);
                let other_exit_distance =
                    other_band.outline_distance_to_path_distance(exit_intersection.along_b);

                let can_weave = other_path
                    .direction_along(other_entry_distance)
                    .rough_eq_by(self.construction.path.direction_along(entry_distance), 0.1)
                    || other_path
                        .direction_along(other_exit_distance)
                        .rough_eq_by(self.construction.path.direction_along(exit_distance), 0.1);

                self.connectivity
                    .interactions
                    .push(Interaction::Conflicting {
                        conflicting: other_id,
                        start: entry_distance,
                        conflicting_start: other_entry_distance.min(other_exit_distance),
                        end: exit_distance,
                        conflicting_end: other_exit_distance.max(other_entry_distance),
                        can_weave,
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
            .any(|existing| existing.direct_partner() == interaction.direct_partner());
        if !already_a_partner {
            self.connectivity.interactions.push(interaction);
            ::transport::pathfinding::Link::on_connect(self);
        }
    }
}

use transport::pathfinding::trip::{TripResult, TripFate};

impl Lane {
    pub fn disconnect(&mut self, other_id: LaneID, world: &mut World) {
        self.connectivity
            .interactions
            .retain(|interaction| interaction.direct_lane_partner() != Some(other_id));

        self.microtraffic.obstacles.drain();

        let self_as_rough_location = self.id_as();

        for car in self.microtraffic.cars.drain() {
            car.trip.finish(
                TripResult {
                    location_now: Some(self_as_rough_location),
                    fate: TripFate::HopDisconnected,
                },
                world,
            );
        }

        ::transport::pathfinding::Link::on_disconnect(self);
        other_id.on_confirm_disconnect(world);
    }

    pub fn disconnect_switch(&mut self, other_id: SwitchLaneID, world: &mut World) {
        self.connectivity
            .interactions
            .retain(|interaction| interaction.direct_switch_partner() != Some(other_id));

        self.microtraffic.obstacles.drain();

        let self_as_rough_location = self.id_as();

        for car in self.microtraffic.cars.drain() {
            car.trip.finish(
                TripResult {
                    location_now: Some(self_as_rough_location),
                    fate: TripFate::HopDisconnected,
                },
                world,
            );
        }

        ::transport::pathfinding::Link::on_disconnect(self);
        other_id.on_confirm_disconnect(world);
    }

    pub fn unbuild(&mut self, report_to: CBConstructionID, world: &mut World) -> Fate {
        let mut disconnects_remaining = 0;

        for lane in self
            .connectivity
            .interactions
            .iter()
            .filter_map(|interaction| interaction.direct_lane_partner())
            .unique()
        {
            lane.disconnect(self.id, world);
            disconnects_remaining += 1;
        }

        for switch_lane in self
            .connectivity
            .interactions
            .iter()
            .filter_map(|interaction| interaction.direct_switch_partner())
            .unique()
        {
            switch_lane.disconnect(self.id, world);
            disconnects_remaining += 1;
        }

        super::ui::on_unbuild(self, world);
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

    pub fn on_confirm_disconnect(&mut self, world: &mut World) -> Fate {
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
    fn finalize(&self, report_to: CBConstructionID, world: &mut World) {
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

        ::transport::pathfinding::road_pathfinding::on_unbuild(self, world);
    }
}

use land_use::buildings::{BuildingID};
use dimensions::LANE_DISTANCE;
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

                debug(
                    LOG_T,
                    format!(
                        "Building {:?} lane {:?} distance: {}",
                        building, self.id, distance
                    ),
                    self.id(),
                    world,
                );

                if distance <= 3.0 * LANE_DISTANCE {
                    if let Some((offset, projected_point)) =
                        path.project_with_max_distance(lot_position, 0.5, 3.0 * LANE_DISTANCE)
                    {
                        debug(LOG_T, format!("Projected: {}", offset), self.id(), world);
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
        report_to: CBConstructionID,
        world: &mut World,
    ) -> SwitchLane {
        LaneID::global_broadcast(world).connect_to_switch(id, world);

        let lane = SwitchLane::spawn(id, path, world);
        super::ui::on_build_switch(&lane, world);

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
            Some((start_on_other_distance, start_on_other)),
            Some((end_on_other_distance, end_on_other)),
        ) = projections
        {
            if start_on_other_distance < end_on_other_distance
                && end_on_other_distance - start_on_other_distance > MIN_SWITCHING_LANE_LENGTH
                && start_on_other
                    .rough_eq_by(self.construction.path.start(), MAX_SWITCHING_LANE_DISTANCE)
                && end_on_other.rough_eq_by(self.construction.path.end(), 3.0)
            {
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
                            segment_end_on_other_distance - start_on_other_distance,
                        )
                    })
                    .collect();

                let other_is_right = (start_on_other - self.construction.path.start())
                    .dot(&self.construction.path.start_direction().orthogonal_right())
                    > 0.0;

                if other_is_right {
                    self.connectivity.right =
                        Some((other_id, start_on_other_distance, end_on_other_distance));
                    self.connectivity.right_distance_map = distance_map;
                } else {
                    self.connectivity.left =
                        Some((other_id, start_on_other_distance, end_on_other_distance));
                    self.connectivity.left_distance_map = distance_map;
                }

                if let (
                    Some((left, left_start_on_other_distance, left_end_on_other_distance)),
                    Some((right, right_start_on_other_distance, right_end_on_other_distance)),
                ) = (self.connectivity.left, self.connectivity.right)
                {
                    left.add_switch_lane_interaction(
                        Interaction::Switch {
                            via: self.id,
                            to: right,
                            start: left_start_on_other_distance,
                            end: left_end_on_other_distance,
                            is_left: false,
                        },
                        world,
                    );

                    right.add_switch_lane_interaction(
                        Interaction::Switch {
                            via: self.id,
                            to: left,
                            start: right_start_on_other_distance,
                            end: right_end_on_other_distance,
                            is_left: true,
                        },
                        world,
                    );
                }
            }
        }
    }
}

impl SwitchLane {
    pub fn disconnect(&mut self, other: LaneID, world: &mut World) {
        self.connectivity.left = self
            .connectivity
            .left
            .filter(|(left_id, ..)| *left_id != other);
        self.connectivity.right = self
            .connectivity
            .right
            .filter(|(right_id, ..)| *right_id != other);
        other.on_confirm_disconnect(world);
    }

    pub fn unbuild(&mut self, report_to: CBConstructionID, world: &mut World) -> Fate {
        self.construction.disconnects_remaining = 0;

        if let (Some((left_id, ..)), Some((right_id, ..))) =
            (self.connectivity.left, self.connectivity.right)
        {
            left_id.disconnect_switch(self.id, world);
            right_id.disconnect_switch(self.id, world);
            self.construction.disconnects_remaining = 2;
        }

        super::ui::on_unbuild_switch(self, world);
        if self.construction.disconnects_remaining == 0 {
            self.finalize(report_to, world);
            Fate::Die
        } else {
            self.construction.unbuilding_for = Some(report_to);
            Fate::Live
        }
    }

    pub fn on_confirm_disconnect(&mut self, world: &mut World) -> Fate {
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
    fn finalize(&self, report_to: CBConstructionID, world: &mut World) {
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
