use kay::{ID, ActorSystem, Fate};
use kay::swarm::{Swarm, SubActor, CreateWith};
use ordered_float::OrderedFloat;

use super::Destination;

#[derive(SubActor, Compact, Clone)]
struct Trip {
    _id: Option<ID>,
    source: ID,
    rough_destination: ID,
    destination: Option<Destination>,
    listener: Option<ID>,
}

#[derive(Copy, Clone)]
pub struct Start;
use super::QueryAsDestination;


pub fn setup(system: &mut ActorSystem) {
    system.add(Swarm::<Trip>::new(),
               Swarm::<Trip>::subactors(|mut each_trip| {
        each_trip.on_create_with(|_: &Start, trip, world| {
            world.send(trip.rough_destination,
                       QueryAsDestination {
                           rough_destination: trip.rough_destination,
                           requester: trip.id(),
                       });
            Fate::Live
        });

        each_trip.on(|&TellAsDestination {
                           as_destination: maybe_destination,
                           rough_destination,
                       },
                      trip,
                      world| {
            if let Some(as_destination) = maybe_destination {
                trip.destination = Some(as_destination);
                world.send(trip.source,
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
                           });
                Fate::Live
            } else {
                println!("{:?} is not a destination yet", rough_destination);
                Fate::Die
            }
        });

        each_trip.on(|control, trip, world| {
            match *control {
                TripControl::Fail { location } => {
                    println!("Trip {:?} failed!", trip.id());
                    if let Some(listener) = trip.listener {
                        world.send(listener, TripResult::Failure { id: trip.id(), location })
                    }
                }
                TripControl::Succeed => {
                    println!("Trip {:?} succeeded!", trip.id());
                    if let Some(listener) = trip.listener {
                        world.send(listener, TripResult::Success { id: trip.id() })
                    }
                }
            }


            Fate::Die
        })
    }));

    system.add(TripCreator { current_source_lane: None },
               |mut the_creator| {
        let trips_id = the_creator.world().id::<Swarm<Trip>>();

        the_creator.on(move |&AddLaneForTrip(lane_id), tc, world| {
            if let Some(source) = tc.current_source_lane {
                world.send(trips_id,
                           CreateWith(Trip {
                                          _id: None,
                                          source: source,
                                          rough_destination: lane_id,
                                          destination: None,
                                          listener: None,
                                      },
                                      Start));
                tc.current_source_lane = None;
            } else {
                tc.current_source_lane = Some(lane_id);
            }
            Fate::Live
        })
    });

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

use super::TellAsDestination;
use super::super::microtraffic::{AddCar, LaneCar, Obstacle};

#[derive(Copy, Clone)]
pub enum TripControl {
    Succeed,
    Fail { location: ID },
}

#[derive(Copy, Clone)]
pub enum TripResult {
    Success { id: ID },
    Failure { id: ID, location: ID },
}

pub struct TripCreator {
    current_source_lane: Option<ID>,
}

#[derive(Copy, Clone)]
pub struct AddLaneForTrip(ID);

use super::super::lane::Lane;
use stagemaster::Event3d;
