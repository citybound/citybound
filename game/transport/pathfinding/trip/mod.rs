use kay::{ID, World, ActorSystem, Fate};
use kay::swarm::{Swarm, SubActor};
use compact::CVec;
use ordered_float::OrderedFloat;
use core::simulation::Timestamp;

use super::Location;
use super::{RoughLocationID, LocationRequester, LocationRequesterID,
            MSG_LocationRequester_location_resolved};

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
        println!("Trip {:?} failed!", self.id());

        if let Some(listener) = self.listener {
            listener.trip_result(self.id, location, true, tick, world);
        }

        Fate::Die
    }

    pub fn succeed(&mut self, tick: Timestamp, world: &mut World) -> Fate {
        println!("Trip {:?} succeeded!", self.id());

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
                world.send(
                    source.node,
                    AddCar {
                        car: LaneCar {
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
                        from: None,
                        tick,
                    },
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
use super::super::microtraffic::{AddCar, LaneCar, Obstacle};

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
    current_source_lane: Option<ID>,
    trips_to_create: CVec<(ID, ID)>,
}

impl TripCreator {
    pub fn spawn(id: TripCreatorID, simulation: SimulationID, _: &mut World) -> TripCreator {
        TripCreator {
            id,
            simulation,
            current_source_lane: None,
            trips_to_create: CVec::new(),
        }
    }

    pub fn add_lane_for_trip(&mut self, lane_id: ID, world: &mut World) {
        if let Some(source_lane_id) = self.current_source_lane {
            self.trips_to_create.push((source_lane_id, lane_id));
            self.simulation.wake_up_in(Ticks(0), self.id.into(), world);
            self.current_source_lane = None;
        } else {
            self.current_source_lane = Some(lane_id);
        }
    }
}

impl Sleeper for TripCreator {
    fn wake(&mut self, current_tick: Timestamp, world: &mut World) {
        for (source, dest) in self.trips_to_create.clone() {
            TripID::spawn(
                RoughLocationID { _raw_id: source },
                RoughLocationID { _raw_id: dest },
                None,
                current_tick,
                world,
            );
        }
        self.current_source_lane = None;
        self.trips_to_create = CVec::new();
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add(Swarm::<Trip>::new(), |_| {});
    system.add(Swarm::<TripCreator>::new(), |_| {});

    auto_setup(system);

    system.extend(Swarm::<Lane>::subactors(|mut each_lane| {
        each_lane.on_random(move |event, lane, world| {
            match *event {
                Event3d::HoverStarted { .. } => {
                    lane.hovered = true;
                }
                Event3d::HoverStopped { .. } => {
                    lane.hovered = false;
                }
                Event3d::DragFinished { .. } => {
                    if !lane.connectivity.on_intersection {
                        // TODO: ugly/wrong
                        TripCreatorID::broadcast(world).add_lane_for_trip(lane.id(), world);
                    }
                }
                _ => {}
            };
            Fate::Live
        })
    }));
}

use super::super::lane::Lane;
use stagemaster::Event3d;

mod kay_auto;
pub use self::kay_auto::*;
