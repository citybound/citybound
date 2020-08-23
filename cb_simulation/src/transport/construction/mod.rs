use compact::CVec;
use kay::{ActorSystem, World, Fate, Actor, TypedID};
use descartes::{N, P2, LinePath, RoughEq};
use itertools::Itertools;

use super::lane::{CarLane, CarLaneID, CarSwitchLane, CarSwitchLaneID, Sidewalk, SidewalkID};
use super::lane::connectivity::{SidewalkInteraction, CarLaneInteraction};

use cb_planning::Prototype;
use cb_planning::construction::{Constructable, ConstructableID};
use planning::{CBConstructionID, CBPrototypeKind};
use super::{
    microtraffic::ObstacleContainerID,
    transport_planning::{
        RoadPrototype, LanePrototype, SwitchLanePrototype, IntersectionPrototype, LaneType,
    },
};

use cb_util::log::debug;
const LOG_T: &str = "Transport Construction";

use dimensions::{LANE_CONNECTION_TOLERANCE};

impl RoadPrototype {
    pub fn construct(
        &self,
        report_to: CBConstructionID,
        world: &mut World,
    ) -> CVec<ConstructableID<CBPrototypeKind>> {
        match *self {
            RoadPrototype::Lane(LanePrototype(ref path, _, LaneType::CarLane)) => {
                vec![CarLaneID::spawn_and_connect(
                    path.clone(),
                    false,
                    CVec::new(),
                    report_to,
                    world,
                )
                .into()]
                .into()
            }
            RoadPrototype::Lane(LanePrototype(ref path, _, LaneType::Sidewalk)) => {
                vec![SidewalkID::spawn_and_connect(
                    path.clone(),
                    false,
                    CVec::new(),
                    report_to,
                    world,
                )
                .into()]
                .into()
            }
            RoadPrototype::SwitchLane(SwitchLanePrototype(ref path)) => {
                vec![CarSwitchLaneID::spawn_and_connect(path.clone(), report_to, world).into()]
                    .into()
            }
            RoadPrototype::Intersection(IntersectionPrototype {
                ref connecting_lanes,
                ..
            }) => {
                let car_lane_ids = connecting_lanes
                    .values()
                    .flat_map(|group| {
                        group
                            .iter()
                            .filter_map(|&LanePrototype(ref path, ref timings, ref lane_type)| {
                                match lane_type {
                                    LaneType::CarLane => Some(CarLaneID::spawn_and_connect(
                                        path.clone(),
                                        true,
                                        timings.clone(),
                                        report_to,
                                        world,
                                    )),
                                    _ => None,
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<CarLaneID>>();

                let sidewalk_ids = connecting_lanes
                    .values()
                    .flat_map(|group| {
                        group
                            .iter()
                            .filter_map(|&LanePrototype(ref path, ref timings, ref lane_type)| {
                                match lane_type {
                                    LaneType::Sidewalk => Some(SidewalkID::spawn_and_connect(
                                        path.clone(),
                                        true,
                                        timings.clone(),
                                        report_to,
                                        world,
                                    )),
                                    _ => None,
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<SidewalkID>>();

                let as_obstacle_containers = car_lane_ids
                    .into_iter()
                    .map(Into::<ObstacleContainerID>::into)
                    .chain(
                        sidewalk_ids
                            .into_iter()
                            .map(Into::<ObstacleContainerID>::into),
                    )
                    .collect::<Vec<_>>();

                let as_constructables = car_lane_ids
                    .into_iter()
                    .map(Into::<ConstructableID<CBPrototypeKind>>::into)
                    .chain(
                        sidewalk_ids
                            .into_iter()
                            .map(Into::<ConstructableID<CBPrototypeKind>>::into),
                    )
                    .collect::<CVec<_>>();

                for id in &as_obstacle_containers {
                    id.start_connecting_overlaps(
                        as_obstacle_containers
                            .iter()
                            .filter(|&other| other != id)
                            .cloned()
                            .collect(),
                        world,
                    )
                }

                as_constructables
            }
            RoadPrototype::PavedArea(_) => CVec::new(),
        }
    }
}

impl Constructable<CBPrototypeKind> for CarLane {
    fn morph(
        &mut self,
        _new_prototype: &Prototype<CBPrototypeKind>,
        report_to: CBConstructionID,
        world: &mut World,
    ) {
        report_to.action_done(self.id_as(), world);
    }
    fn destruct(&mut self, report_to: CBConstructionID, world: &mut World) -> Fate {
        self.unbuild(report_to, world);
        Fate::Live
    }
}

impl Constructable<CBPrototypeKind> for CarSwitchLane {
    fn morph(
        &mut self,
        _new_prototype: &Prototype<CBPrototypeKind>,
        report_to: CBConstructionID,
        world: &mut World,
    ) {
        report_to.action_done(self.id_as(), world);
    }
    fn destruct(&mut self, report_to: CBConstructionID, world: &mut World) -> Fate {
        self.unbuild(report_to, world);
        Fate::Live
    }
}

impl Constructable<CBPrototypeKind> for Sidewalk {
    fn morph(
        &mut self,
        _new_prototype: &Prototype<CBPrototypeKind>,
        report_to: CBConstructionID,
        world: &mut World,
    ) {
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
    pub unbuilding_for: Option<CBConstructionID>,
    pub disconnects_remaining: u8,
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

// use fnv::FnvHashMap;

// TODO: not thread safe for now
// static mut MEMOIZED_BANDS_OUTLINES: Option<FnvHashMap<CarLaneLikeID, (Band, ClosedLinePath)>> =
// None;

impl CarLane {
    pub fn spawn_and_connect(
        id: CarLaneID,
        path: &LinePath,
        on_intersection: bool,
        timings: &CVec<bool>,
        report_to: CBConstructionID,
        world: &mut World,
    ) -> CarLane {
        CarLaneID::global_broadcast(world).connect(
            id,
            path.start(),
            path.end(),
            path.length(),
            true,
            world,
        );
        if !on_intersection {
            CarSwitchLaneID::global_broadcast(world).connect_switch_to_normal(
                id,
                path.clone(),
                world,
            );
        }
        report_to.action_done(id.into(), world);
        CarLane::spawn(id, path, on_intersection, timings, world)
    }

    pub fn connect(
        &mut self,
        other_id: CarLaneID,
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
                        CarLaneInteraction::Next { next, .. } => next == other_id,
                        _ => false,
                    });
            if !already_a_partner {
                self.connectivity
                    .interactions
                    .push(CarLaneInteraction::Next {
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
                        CarLaneInteraction::Previous { previous, .. } => previous == other_id,
                        _ => false,
                    });
            if !already_a_partner {
                self.connectivity
                    .interactions
                    .push(CarLaneInteraction::Previous {
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

    pub fn connect_to_switch(&mut self, other_id: CarSwitchLaneID, world: &mut World) {
        other_id.connect_switch_to_normal(self.id, self.construction.path.clone(), world);
    }

    pub fn add_switch_lane_interaction(&mut self, interaction: CarLaneInteraction, _: &mut World) {
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

impl CarLane {
    pub fn disconnect_switch(&mut self, other_id: CarSwitchLaneID, world: &mut World) {
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
        Into::<ObstacleContainerID>::into(other_id).on_confirm_disconnect(world);
    }

    pub fn unbuild(&mut self, report_to: CBConstructionID, world: &mut World) -> Fate {
        let mut disconnects_remaining = 0;

        for lane in self
            .connectivity
            .interactions
            .iter()
            .filter_map(CarLaneInteraction::direct_lane_partner)
            .unique()
        {
            lane.disconnect(self.id_as(), world);
            disconnects_remaining += 1;
        }

        for switch_lane in self
            .connectivity
            .interactions
            .iter()
            .filter_map(CarLaneInteraction::direct_switch_partner)
            .unique()
        {
            Into::<ObstacleContainerID>::into(switch_lane).disconnect(self.id_as(), world);
            disconnects_remaining += 1;
        }

        super::ui::on_unbuild(self, world);
        // unsafe {
        //     MEMOIZED_BANDS_OUTLINES
        //         .get_or_insert_with(FnvHashMap::default)
        //         .remove(&self.id_as());
        // }

        if disconnects_remaining == 0 {
            self.finalize(report_to, world);
            Fate::Die
        } else {
            self.construction.disconnects_remaining = disconnects_remaining;
            self.construction.unbuilding_for = Some(report_to);
            Fate::Live
        }
    }
}

impl CarLane {
    pub fn finalize(&self, report_to: CBConstructionID, world: &mut World) {
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

impl CarLane {
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

impl CarSwitchLane {
    pub fn spawn_and_connect(
        id: CarSwitchLaneID,
        path: &LinePath,
        report_to: CBConstructionID,
        world: &mut World,
    ) -> CarSwitchLane {
        CarLaneID::global_broadcast(world).connect_to_switch(id, world);

        let lane = CarSwitchLane::spawn(id, path, world);
        super::ui::on_build_switch(&lane, world);

        report_to.action_done(id.into(), world);

        lane
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

    pub fn finalize(&self, report_to: CBConstructionID, world: &mut World) {
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

impl Sidewalk {
    pub fn spawn_and_connect(
        id: SidewalkID,
        path: &LinePath,
        on_intersection: bool,
        timings: &CVec<bool>,
        report_to: CBConstructionID,
        world: &mut World,
    ) -> Sidewalk {
        SidewalkID::global_broadcast(world).connect(
            id,
            path.start(),
            path.end(),
            path.length(),
            true,
            world,
        );
        report_to.action_done(id.into(), world);
        Sidewalk::spawn(id, path, on_intersection, timings, world)
    }

    pub fn connect(
        &mut self,
        other_id: SidewalkID,
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
                        SidewalkInteraction::Next { next, .. } => next == other_id,
                        _ => false,
                    });
            if !already_a_partner {
                self.connectivity
                    .interactions
                    .push(SidewalkInteraction::Next {
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
                        SidewalkInteraction::Previous { previous, .. } => previous == other_id,
                        _ => false,
                    });
            if !already_a_partner {
                self.connectivity
                    .interactions
                    .push(SidewalkInteraction::Previous { previous: other_id });
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
}

impl Sidewalk {
    pub fn finalize(&self, report_to: CBConstructionID, world: &mut World) {
        report_to.action_done(self.id_as(), world);

        for pedestrian in &self.microtraffic.pedestrians {
            pedestrian.trip.finish(
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

pub fn setup(system: &mut ActorSystem) {
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
