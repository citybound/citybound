use kay::{ID, Recipient, Individual, Actor, ActorSystem, Swarm, CreateWith, Fate};
use ordered_float::OrderedFloat;

use super::Destination;

#[derive(Actor, Compact, Clone)]
struct Trip {
    _id: ID,
    source: ID,
    rough_destination: ID,
    destination: Option<Destination>,
}

#[derive(Copy, Clone)]
pub struct Start;
use super::QueryAsDestination;

impl Recipient<Start> for Trip {
    fn receive(&mut self, _msg: &Start) -> Fate {
        self.rough_destination << QueryAsDestination { requester: self.id() };
        Fate::Live
    }
}

use super::TellAsDestination;
use super::super::{AddCar, LaneCar, Obstacle};

impl Recipient<TellAsDestination> for Trip {
    fn receive(&mut self, msg: &TellAsDestination) -> Fate {
        match *msg {
            TellAsDestination { as_destination: Some(as_destination), .. } => {
                self.destination = Some(as_destination);
                self.source <<
                AddCar {
                    car: LaneCar {
                        trip: self.id(),
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
                };
                Fate::Live
            }
            TellAsDestination { id, as_destination: None } => {
                println!("{:?} is not a destination yet", id);
                Fate::Die
            }
        }
    }
}

#[derive(Copy, Clone)]
pub enum TripResult {
    Success,
    Failure,
}

impl Recipient<TripResult> for Trip {
    fn receive(&mut self, msg: &TripResult) -> Fate {
        match *msg {
            TripResult::Failure => {
                println!("Trip {:?} failed!", self.id());
                Fate::Die
            }
            TripResult::Success => {
                println!("Trip {:?} succeeded!", self.id());
                Fate::Die
            }
        }
    }
}

pub struct TripCreator {
    current_source_lane: Option<ID>,
}

impl Individual for TripCreator {}

#[derive(Copy, Clone)]
pub struct AddLaneForTrip(ID);

impl Recipient<AddLaneForTrip> for TripCreator {
    fn receive(&mut self, msg: &AddLaneForTrip) -> Fate {
        match *msg {
            AddLaneForTrip(lane_id) => {
                if let Some(source) = self.current_source_lane {
                    Swarm::<Trip>::all() <<
                    CreateWith(Trip {
                                   _id: ID::invalid(),
                                   source: source,
                                   rough_destination: lane_id,
                                   destination: None,
                               },
                               Start);
                    self.current_source_lane = None;
                } else {
                    self.current_source_lane = Some(lane_id);
                }
                Fate::Live
            }
        }
    }
}

use super::super::Lane;
use ::core::ui::Event3d;

impl Recipient<Event3d> for Lane {
    fn receive(&mut self, msg: &Event3d) -> Fate {
        match *msg {
            Event3d::HoverStarted { .. } => {
                self.hovered = true;
                Fate::Live
            }
            Event3d::HoverStopped { .. } => {
                self.hovered = false;
                Fate::Live
            }
            Event3d::DragFinished { .. } => {
                if !self.on_intersection {
                    TripCreator::id() << AddLaneForTrip(self.id());
                }
                Fate::Live
            }
            _ => Fate::Live,
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(Swarm::<Trip>::new());
    system.add_inbox::<TripResult, Swarm<Trip>>();
    system.add_inbox::<CreateWith<Trip, Start>, Swarm<Trip>>();
    system.add_inbox::<TellAsDestination, Swarm<Trip>>();

    system.add_individual(TripCreator { current_source_lane: None });
    system.add_inbox::<AddLaneForTrip, TripCreator>();

    system.add_inbox::<Event3d, Swarm<Lane>>();
}
