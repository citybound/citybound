use kay::{ID, World, ActorSystem, Fate};
use kay::swarm::{Swarm, SubActor, CreateWith};
use ordered_float::OrderedFloat;
use core::simulation::Timestamp;

use super::Destination;
use super::{RoughDestinationID, AsDestinationRequesterID,
            MSG_AsDestinationRequester_tell_as_destination};

#[derive(SubActor, Compact, Clone)]
pub struct Trip {
    _id: Option<ID>,
    source: ID,
    rough_destination: RoughDestinationID,
    destination: Option<Destination>,
    listener: Option<TripListenerID>,
}

impl Trip {
    pub fn new(
        source: ID,
        rough_destination: RoughDestinationID,
        listener: Option<TripListenerID>,
    ) -> Self {
        Trip {
            _id: None,
            source,
            rough_destination,
            listener,
            destination: None,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Start(pub Timestamp);

pub fn setup(system: &mut ActorSystem) {
    system.add(
        Swarm::<Trip>::new(),
        Swarm::<Trip>::subactors(|mut each_trip| {
            each_trip.on_create_with(|&Start(tick), trip, world| {
                trip.rough_destination.query_as_destination(
                    AsDestinationRequesterID { _raw_id: trip.id() },
                    trip.rough_destination._raw_id,
                    Some(tick),
                    world,
                );

                if let Some(listener) = trip.listener {
                    listener.trip_created(trip.id(), world);
                }
                Fate::Live
            });

            each_trip.on(
                |&MSG_AsDestinationRequester_tell_as_destination(rough_destination,
                                                                 maybe_destination,
                                                                 tick),
                 trip,
                 world| {
                    if let Some(as_destination) = maybe_destination {
                        trip.destination = Some(as_destination);
                        world.send(
                            trip.source,
                            AddCar {
                                car: LaneCar {
                                    trip: trip.id(),
                                    as_obstacle: Obstacle {
                                        position: OrderedFloat(-1.0),
                                        velocity: 0.0,
                                        max_velocity: 15.0,
                                    },
                                    acceleration: 0.0,
                                    destination: as_destination,
                                    next_hop_interaction: 0,
                                },
                                from: None,
                            },
                        );
                        Fate::Live
                    } else {
                        println!("{:?} is not a destination yet", rough_destination._raw_id);
                        if let Some(listener) = trip.listener {
                            listener.trip_result(
                                trip.id(),
                                RoughDestinationID { _raw_id: trip.source },
                                true,
                                tick.expect("Should have a tick"),
                                world,
                            );
                        }
                        Fate::Die
                    }
                },
            );

            each_trip.on(|control, trip, world| {
                match *control {
                    TripControl::Fail { location, tick } => {
                        println!("Trip {:?} failed!", trip.id());
                        if let Some(listener) = trip.listener {
                            listener.trip_result(trip.id(), location, true, tick, world);
                        }
                    }
                    TripControl::Succeed { tick } => {
                        println!("Trip {:?} succeeded!", trip.id());
                        if let Some(listener) = trip.listener {
                            listener.trip_result(
                                trip.id(),
                                trip.rough_destination,
                                false,
                                tick,
                                world,
                            );
                        }
                    }
                }


                Fate::Die
            })
        }),
    );

    system.add(
        TripCreator { current_source_lane: None },
        |mut the_creator| {
            let trips_id = the_creator.world().id::<Swarm<Trip>>();

            the_creator.on(move |&AddLaneForTrip(lane_id), tc, world| {
                if let Some(source) = tc.current_source_lane {
                    world.send(
                        trips_id,
                        CreateWith(
                            Trip {
                                _id: None,
                                source: source,
                                rough_destination: RoughDestinationID { _raw_id: lane_id },
                                destination: None,
                                listener: None,
                            },
                            Start,
                        ),
                    );
                    tc.current_source_lane = None;
                } else {
                    tc.current_source_lane = Some(lane_id);
                }
                Fate::Live
            })
        },
    );

    system.extend(Swarm::<Lane>::subactors(|mut each_lane| {
        let creator_id = each_lane.world().id::<TripCreator>();

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
                        world.send(creator_id, AddLaneForTrip(lane.id()));
                    }
                }
                _ => {}
            };
            Fate::Live
        })
    }));
}

use super::super::microtraffic::{AddCar, LaneCar, Obstacle};

#[derive(Copy, Clone)]
pub enum TripControl {
    Succeed { tick: Timestamp },
    Fail {
        location: RoughDestinationID,
        tick: Timestamp,
    },
}

pub trait TripListener {
    fn trip_created(&mut self, trip: ID, world: &mut World);
    fn trip_result(
        &mut self,
        trip: ID,
        location: RoughDestinationID,
        failed: bool,
        tick: Timestamp,
        world: &mut World,
    );
}

pub struct TripCreator {
    current_source_lane: Option<ID>,
}

#[derive(Copy, Clone)]
pub struct AddLaneForTrip(ID);

use super::super::lane::Lane;
use stagemaster::Event3d;

mod kay_auto;
pub use self::kay_auto::*;
