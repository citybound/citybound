use kay::{World, ActorSystem, Fate};
use compact::CVec;
use ordered_float::OrderedFloat;
use core::simulation::Timestamp;

use transport::lane::LaneID;
use super::Location;
use super::{RoughLocationID, LocationRequester, LocationRequesterID,
            MSG_LocationRequester_location_resolved};

use itertools::Itertools;

#[derive(Compact, Clone)]
pub struct Trip {
    id: TripID,
    rough_source: RoughLocationID,
    rough_destination: RoughLocationID,
    source: Option<Location>,
    destination: Option<Location>,
    listener: Option<TripListenerID>,
}

impl Trip {
    pub fn spawn(
        id: TripID,
        rough_source: RoughLocationID,
        rough_destination: RoughLocationID,
        listener: Option<TripListenerID>,
        tick: Timestamp,
        world: &mut World,
    ) -> Self {
        rough_source.resolve_as_location(id.into(), rough_source, tick, world);

        if let Some(listener) = listener {
            listener.trip_created(id, world);
        }

        Trip {
            id: id,
            rough_source,
            rough_destination,
            listener,
            source: None,
            destination: None,
        }
    }

    pub fn fail_at(
        &mut self,
        location: RoughLocationID,
        tick: Timestamp,
        world: &mut World,
    ) -> Fate {
        println!("Trip {:?} failed!", self.id);

        if let Some(listener) = self.listener {
            listener.trip_result(self.id, location, true, tick, world);
        }

        Fate::Die
    }

    pub fn succeed(&mut self, tick: Timestamp, world: &mut World) -> Fate {
        println!("Trip {:?} succeeded!", self.id);

        if let Some(listener) = self.listener {
            listener.trip_result(self.id, self.rough_destination, false, tick, world);
        }

        Fate::Die
    }
}

impl LocationRequester for Trip {
    fn location_resolved(
        &mut self,
        rough_location: RoughLocationID,
        location: Option<Location>,
        tick: Timestamp,
        world: &mut World,
    ) {
        if let Some(precise) = location {
            if rough_location == self.rough_source {
                self.source = Some(precise);

                if self.rough_source == self.rough_destination {
                    self.destination = Some(precise);
                } else {
                    self.rough_destination.resolve_as_location(
                        self.id.into(),
                        self.rough_destination,
                        tick,
                        world,
                    );
                }
            } else if rough_location == self.rough_destination {
                self.destination = Some(precise);
            } else {
                unreachable!();
            }

            if let (Some(source), Some(destination)) = (self.source, self.destination) {
                // TODO: ugly: untyped ID shenanigans
                let source_as_lane: LaneLikeID = LaneLikeID { _raw_id: source.node._raw_id };
                source_as_lane.add_car(
                    LaneCar {
                        trip: self.id,
                        as_obstacle: Obstacle {
                            position: OrderedFloat(-1.0),
                            velocity: 0.0,
                            max_velocity: 15.0,
                        },
                        acceleration: 0.0,
                        destination: destination,
                        next_hop_interaction: 0,
                    },
                    None,
                    tick,
                    world,
                );
            }
        } else {
            println!(
                "{:?} is not a source/destination yet",
                rough_location._raw_id
            );
            self.id.fail_at(self.rough_source, tick, world);
        }
    }
}

use core::simulation::{SimulationID, Sleeper, SleeperID, MSG_Sleeper_wake};
use core::simulation::Ticks;
use super::super::microtraffic::{LaneLikeID, LaneCar, Obstacle};

pub trait TripListener {
    fn trip_created(&mut self, trip: TripID, world: &mut World);
    fn trip_result(
        &mut self,
        trip: TripID,
        location: RoughLocationID,
        failed: bool,
        tick: Timestamp,
        world: &mut World,
    );
}

#[derive(Compact, Clone)]
pub struct TripCreator {
    id: TripCreatorID,
    simulation: SimulationID,
    lanes: CVec<LaneID>,
}

impl TripCreator {
    pub fn spawn(id: TripCreatorID, simulation: SimulationID, _: &mut World) -> TripCreator {
        TripCreator { id, simulation, lanes: CVec::new() }
    }

    pub fn add_lane_for_trip(&mut self, lane_id: LaneID, world: &mut World) {
        self.lanes.push(lane_id);

        if self.lanes.len() > 1 {
            self.simulation.wake_up_in(Ticks(50), self.id.into(), world);
        }
    }
}

use rand::Rng;

impl Sleeper for TripCreator {
    fn wake(&mut self, current_tick: Timestamp, world: &mut World) {
        ::rand::thread_rng().shuffle(&mut self.lanes);

        for mut pair in &self.lanes.iter().chunks(2) {
            if let (Some(source), Some(dest)) = (pair.next(), pair.next()) {
                TripID::spawn((*source).into(), (*dest).into(), None, current_tick, world);
            }
        }

        self.lanes = CVec::new();
    }
}

use stagemaster::{Interactable3d, Interactable3dID, MSG_Interactable3d_on_event};

impl Interactable3d for Lane {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        match event {
            Event3d::HoverStarted { .. } => {
                self.hovered = true;
            }
            Event3d::HoverStopped { .. } => {
                self.hovered = false;
            }
            Event3d::DragFinished { .. } => {
                if !self.connectivity.on_intersection {
                    // TODO: ugly/wrong
                    TripCreatorID::local_first(world).add_lane_for_trip(self.id, world);
                }
            }
            _ => {}
        };
    }
}

pub fn setup(system: &mut ActorSystem, simulation: SimulationID) {
    system.register::<Trip>();
    system.register::<TripCreator>();
    auto_setup(system);

    TripCreatorID::spawn(simulation, &mut system.world());
}

use super::super::lane::Lane;
use stagemaster::Event3d;

mod kay_auto;
pub use self::kay_auto::*;
