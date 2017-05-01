use kay::{ID, Recipient, Actor, Fate};
use kay::swarm::{Swarm, SubActor, CreateWith};
use ordered_float::OrderedFloat;

use super::Destination;

#[derive(SubActor, Compact, Clone)]
struct Trip {
    _id: Option<ID>,
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
use super::super::microtraffic::{AddCar, LaneCar, Obstacle};

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
            TellAsDestination {
                id,
                as_destination: None,
            } => {
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

impl Actor for TripCreator {}

#[derive(Copy, Clone)]
pub struct AddLaneForTrip(ID);

impl Recipient<AddLaneForTrip> for TripCreator {
    fn receive(&mut self, msg: &AddLaneForTrip) -> Fate {
        match *msg {
            AddLaneForTrip(lane_id) => {
                if let Some(source) = self.current_source_lane {
                    Swarm::<Trip>::all() <<
                    CreateWith(Trip {
                                   _id: None,
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

use super::super::lane::Lane;
use stagemaster::Event3d;

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
                if !self.connectivity.on_intersection {
                    TripCreator::id() << AddLaneForTrip(self.id());
                }
                Fate::Live
            }
            _ => Fate::Live,
        }
    }
}

pub fn setup() {
    Swarm::<Trip>::register_default();
    Swarm::<Trip>::handle::<TripResult>();
    Swarm::<Trip>::handle::<CreateWith<Trip, Start>>();
    Swarm::<Trip>::handle::<TellAsDestination>();

    TripCreator::register_with_state(TripCreator { current_source_lane: None });
    TripCreator::handle::<AddLaneForTrip>();

    Swarm::<Lane>::handle::<Event3d>();
}
