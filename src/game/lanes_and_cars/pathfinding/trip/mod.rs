use kay::{ID, World, ActorSystem, Fate};
use kay::swarm::{Swarm, SubActor};
use ordered_float::OrderedFloat;
use core::simulation::Timestamp;

use super::Destination;
use super::{RoughDestinationID, AsDestinationRequester, AsDestinationRequesterID,
            MSG_AsDestinationRequester_tell_as_destination};

#[derive(Compact, Clone)]
pub struct Trip {
    id: TripID,
    rough_source: RoughDestinationID,
    rough_destination: RoughDestinationID,
    source: Option<Destination>,
    destination: Option<Destination>,
    listener: Option<TripListenerID>,
}

impl Trip {
    pub fn spawn(
        id: TripID,
        rough_source: RoughDestinationID,
        rough_destination: RoughDestinationID,
        listener: Option<TripListenerID>,
        tick: Timestamp,
        world: &mut World,
    ) -> Self {

        rough_source.query_as_destination(id.into(), rough_source, tick, world);
        rough_destination.query_as_destination(id.into(), rough_destination, tick, world);

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
        location: RoughDestinationID,
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

impl AsDestinationRequester for Trip {
    fn tell_as_destination(
        &mut self,
        rough_destination: RoughDestinationID,
        as_destination: Option<Destination>,
        tick: Timestamp,
        world: &mut World,
    ) {
        if let Some(precise) = as_destination {
            if rough_destination == self.rough_source {
                self.source = Some(precise);
            } else if rough_destination == self.rough_destination {
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
                rough_destination._raw_id
            );
            self.id.fail_at(self.rough_source, tick, world);
        }
    }
}

use core::simulation::{Simulation, SleeperID, WakeUpIn, MSG_Sleeper_wake};
use core::simulation::DurationTicks;

pub fn setup(system: &mut ActorSystem) {
    auto_setup(system);

    system.add(
        TripCreator {
            current_source_lane: None,
            current_destination_lane: None,
        },
        |mut the_creator| {
            the_creator.on(move |&AddLaneForTrip(lane_id), tc, world| {
                if tc.current_source_lane.is_some() {
                    tc.current_destination_lane = Some(lane_id)
                } else {
                    tc.current_source_lane = Some(lane_id);
                }
                let sim_id = world.id::<Simulation>();
                let tc_id = world.id::<TripCreator>();
                world.send(
                    sim_id,
                    WakeUpIn(DurationTicks::new(0), SleeperID { _raw_id: tc_id }),
                );
                Fate::Live
            });

            the_creator.on(move |&MSG_Sleeper_wake(current_tick), tc, world| {
                TripID::spawn(
                    RoughDestinationID {
                        _raw_id: tc.current_source_lane.expect(
                            "Should already have source lane",
                        ),
                    },
                    RoughDestinationID {
                        _raw_id: tc.current_destination_lane.expect(
                            "Should already have destination lane",
                        ),
                    },
                    None,
                    current_tick,
                    world,
                );
                tc.current_source_lane = None;
                tc.current_destination_lane = None;
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

pub trait TripListener {
    fn trip_created(&mut self, trip: TripID, world: &mut World);
    fn trip_result(
        &mut self,
        trip: TripID,
        location: RoughDestinationID,
        failed: bool,
        tick: Timestamp,
        world: &mut World,
    );
}

pub struct TripCreator {
    current_source_lane: Option<ID>,
    current_destination_lane: Option<ID>,
}

#[derive(Copy, Clone)]
pub struct AddLaneForTrip(ID);

use super::super::lane::Lane;
use stagemaster::Event3d;

mod kay_auto;
pub use self::kay_auto::*;
