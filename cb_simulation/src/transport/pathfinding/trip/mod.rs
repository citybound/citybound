use kay::{World, ActorSystem, Fate, TypedID, Actor};
use compact::CVec;
use ordered_float::OrderedFloat;
use cb_time::units::Instant;

use transport::lane::LaneID;
use super::{PreciseLocation, RoughLocationID, LocationRequester, LocationRequesterID};

use itertools::Itertools;
use super::super::lane::Lane;

use cb_util::log::{debug, warn};
const LOG_T: &str = "Trips";

#[derive(Compact, Clone)]
pub struct Trip {
    id: TripID,
    rough_source: RoughLocationID,
    rough_destination: RoughLocationID,
    source: Option<PreciseLocation>,
    destination: Option<PreciseLocation>,
    listener: Option<TripListenerID>,
}

#[derive(Copy, Clone)]
pub struct TripResult {
    pub location_now: Option<RoughLocationID>,
    //pub instant: Instant,
    pub fate: TripFate,
}

#[derive(Copy, Clone, Debug)]
pub enum TripFate {
    Success(Instant),
    SourceOrDestinationNotResolvable,
    NoRoute,
    RouteForgotten,
    HopDisconnected,
    LaneUnbuilt,
    ForceStopped,
}

const DEBUG_FAILED_TRIPS_VISUALLY: bool = false;

impl Trip {
    pub fn spawn(
        id: TripID,
        rough_source: RoughLocationID,
        rough_destination: RoughLocationID,
        listener: Option<TripListenerID>,
        instant: Instant,
        world: &mut World,
    ) -> Self {
        rough_source.resolve_as_location(id.into(), rough_source, instant, world);

        if let Some(listener) = listener {
            listener.trip_created(id, world);
        }

        Trip {
            id,
            rough_source,
            rough_destination,
            listener,
            source: None,
            destination: None,
        }
    }

    pub fn finish(&mut self, result: TripResult, world: &mut World) -> Fate {
        match result.fate {
            TripFate::Success(_) | TripFate::ForceStopped => {}
            reason => {
                warn(
                    LOG_T,
                    format!(
                        "Trip {:?} failed! ({:?}) {:?} ({:?}) -> {:?} ({:?})",
                        self.id,
                        reason,
                        self.rough_source,
                        self.source,
                        self.rough_destination,
                        self.destination
                    ),
                    self.id(),
                    world,
                );
                if DEBUG_FAILED_TRIPS_VISUALLY {
                    FailedTripDebuggerID::spawn(self.rough_source, self.rough_destination, world);
                }
            }
        }

        if let Some(listener) = self.listener {
            listener.trip_result(
                self.id,
                result,
                self.rough_source,
                self.rough_destination,
                world,
            );
        }

        Fate::Die
    }
}

impl LocationRequester for Trip {
    fn location_resolved(
        &mut self,
        rough_location: RoughLocationID,
        location: Option<PreciseLocation>,
        instant: Instant,
        world: &mut World,
    ) {
        if let Some(precise) = location {
            if rough_location == self.rough_source {
                self.source = Some(precise);

                if self.rough_source == self.rough_destination {
                    self.destination = Some(precise);
                } else {
                    self.rough_destination.resolve_as_location(
                        self.id_as(),
                        self.rough_destination,
                        instant,
                        world,
                    );
                }
            } else if rough_location == self.rough_destination {
                self.destination = Some(precise);
            } else {
                unreachable!();
            }

            if let (Some(source), Some(destination)) = (self.source, self.destination) {
                // TODO: ugly: untyped RawID shenanigans
                let source_as_lane: LaneLikeID = LaneLikeID::from_raw(source.link.as_raw());
                source_as_lane.add_car(
                    LaneCar {
                        trip: self.id,
                        as_obstacle: Obstacle {
                            position: OrderedFloat(source.offset),
                            velocity: 0.0,
                            max_velocity: 8.0,
                        },
                        acceleration: 0.0,
                        destination,
                        next_hop_interaction: None,
                    },
                    None,
                    instant,
                    world,
                );
            }
        } else {
            debug(
                LOG_T,
                format!(
                    "{:?} is not a source/destination yet",
                    rough_location.as_raw()
                ),
                self.id(),
                world,
            );
            self.id.finish(
                TripResult {
                    location_now: Some(self.rough_source),
                    fate: TripFate::SourceOrDestinationNotResolvable,
                },
                world,
            );
        }
    }
}

use cb_time::actors::{TimeID, Sleeper, SleeperID};
use cb_time::units::Ticks;
use super::super::microtraffic::{LaneLikeID, LaneCar, Obstacle};

pub trait TripListener {
    fn trip_created(&mut self, trip: TripID, world: &mut World);
    fn trip_result(
        &mut self,
        trip: TripID,
        result: TripResult,
        rough_source: RoughLocationID,
        rough_destination: RoughLocationID,
        world: &mut World,
    );
}

#[derive(Compact, Clone)]
pub struct TripCreator {
    id: TripCreatorID,
    time: TimeID,
    lanes: CVec<LaneID>,
}

impl TripCreator {
    pub fn spawn(id: TripCreatorID, time: TimeID, _: &mut World) -> TripCreator {
        TripCreator {
            id,
            time,
            lanes: CVec::new(),
        }
    }

    pub fn add_lane_for_trip(&mut self, lane_id: LaneID, world: &mut World) {
        self.lanes.push(lane_id);

        if self.lanes.len() > 1 {
            self.time.wake_up_in(Ticks(50), self.id_as(), world);
        }
    }
}

use rand::Rng;

impl Sleeper for TripCreator {
    fn wake(&mut self, current_instant: Instant, world: &mut World) {
        ::rand::thread_rng().shuffle(&mut self.lanes);

        for mut pair in &self.lanes.iter().chunks(2) {
            if let (Some(source), Some(dest)) = (pair.next(), pair.next()) {
                TripID::spawn(
                    (*source).into(),
                    (*dest).into(),
                    None,
                    current_instant,
                    world,
                );
            }
        }

        self.lanes = CVec::new();
    }
}

pub const DEBUG_MANUALLY_SPAWN_CARS: bool = false;

impl Lane {
    pub fn manually_spawn_car_add_lane(&self, world: &mut World) {
        if !self.connectivity.on_intersection {
            // TODO: ugly/wrong
            TripCreatorID::local_first(world).add_lane_for_trip(self.id, world);
        }
    }
}

use super::{PositionRequester, PositionRequesterID};
use descartes::{P2};

#[derive(Compact, Clone)]
pub struct FailedTripDebugger {
    id: FailedTripDebuggerID,
    rough_source: RoughLocationID,
    source_position: Option<P2>,
    rough_destination: RoughLocationID,
    destination_position: Option<P2>,
}

impl FailedTripDebugger {
    pub fn spawn(
        id: FailedTripDebuggerID,
        rough_source: RoughLocationID,
        rough_destination: RoughLocationID,
        world: &mut World,
    ) -> Self {
        rough_source.resolve_as_position(id.into(), rough_source, world);
        rough_destination.resolve_as_position(id.into(), rough_destination, world);
        FailedTripDebugger {
            id,
            rough_source,
            source_position: None,
            rough_destination,
            destination_position: None,
        }
    }

    pub fn done(&mut self, _: &mut World) -> ::kay::Fate {
        ::kay::Fate::Die
    }
}

impl PositionRequester for FailedTripDebugger {
    fn position_resolved(
        &mut self,
        rough_location: RoughLocationID,
        position: P2,
        world: &mut World,
    ) {
        if rough_location == self.rough_source {
            self.source_position = Some(position);
        } else {
            self.destination_position = Some(position);
        }

        if let (Some(_source_position), Some(_destination_position)) =
            (self.source_position, self.destination_position)
        {
            // TODO: render this again
            self.id.done(world);
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Trip>();
    system.register::<TripCreator>();
    system.register::<FailedTripDebugger>();
    auto_setup(system);
}

pub fn spawn(world: &mut World, time: TimeID) {
    TripCreatorID::spawn(time, world);
}

mod kay_auto;
pub use self::kay_auto::*;
