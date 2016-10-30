pub mod ui;
mod intelligent_acceleration;
use self::intelligent_acceleration::intelligent_acceleration;
use core::geometry::CPath;
use kay::{ID, CVec, Recipient, World, ActorSystem, InMemory, Compact};
use compass::{FiniteCurve, Path, Segment, P2, V2};
use core::simulation::{Simulation, Tick, AddSimulatable};
use ordered_float::OrderedFloat;
use itertools::Itertools;
use ::std::f32::INFINITY;
use ::std::ops::{Deref, DerefMut};

derive_compact!{
    pub struct Lane {
        length: f32,
        path: CPath,
        interactions: CVec<Interaction>,
        interaction_obstacles: CVec<Obstacle>,
        cars: CVec<LaneCar>
    }
}

impl Lane {
    fn new(path: CPath) -> Self {
        Lane {
            length: path.length(),
            path: path,
            interactions: CVec::new(),
            interaction_obstacles: CVec::new(),
            cars: CVec::new()
        }
    }

    fn add_next_lane(&mut self, next_lane: ID) {
        self.interactions.push(Interaction{
            partner_lane: next_lane,
            kind: Next{partner_start: 0.0}
        });
    }

    fn add_previous_lane(&mut self, previous_lane: ID, previous_lane_length: f32) {
        self.interactions.push(Interaction{
            partner_lane: previous_lane,
            kind: Previous{start: 0.0, partner_length: previous_lane_length}
        });
    } 
}

#[derive(Copy, Clone)]
pub struct Obstacle {
    position: OrderedFloat<f32>,
    velocity: f32
}

impl Obstacle {
    fn far_away() -> Obstacle {Obstacle{position: OrderedFloat(INFINITY), velocity: INFINITY}}
    fn offset_by(&self, delta: f32) -> Obstacle {
        Obstacle{
            position: OrderedFloat(*self.position + delta),
            .. *self
        }
    } 
}

#[derive(Copy, Clone)]
pub struct LaneCar {
    trip: ID,
    as_obstacle: Obstacle,
    acceleration: f32,
    max_velocity: f32
}

impl LaneCar {
    fn offset_by(&self, delta: f32) -> LaneCar {
        LaneCar{
            as_obstacle: self.as_obstacle.offset_by(delta),
            .. *self
        }
    }
}

impl Deref for LaneCar {
    type Target = Obstacle;

    fn deref(&self) -> &Obstacle {&self.as_obstacle}
}

impl DerefMut for LaneCar {
    fn deref_mut(&mut self) -> &mut Obstacle {&mut self.as_obstacle}
}

#[derive(Copy, Clone)]
struct Interaction {
    partner_lane: ID,
    kind: InteractionKind
}

#[derive(Copy, Clone)]
enum InteractionKind{
    Overlap{
        start: f32,
        end: f32,
        partner_start: f32,
        partner_end: f32,
        kind: OverlapKind
    },
    Next{
        partner_start: f32
    },
    Previous{
        start: f32,
        partner_length: f32
    }
}
use self::InteractionKind::{Overlap, Next, Previous};

#[derive(Copy, Clone)]
enum OverlapKind{Parallel, Conflicting}
use self::OverlapKind::{Parallel, Conflicting};

// MESSAGES

#[derive(Copy, Clone)]
struct AddCar(LaneCar);

#[derive(Copy, Clone)]
struct AddInteractionObstacle(Obstacle);

recipient!(Lane, (&mut self, world: &mut World, self_id: ID) {
    AddCar: &AddCar(car) => {
        self.cars.insert(0, car);
    },

    AddInteractionObstacle: &AddInteractionObstacle(obstacle) => {
        self.interaction_obstacles.push(obstacle);
    },

    Tick: &Tick{dt} => {
        for c in 0..self.cars.len() {
            let next_obstacle = self.cars.get(c + 1).map_or(Obstacle::far_away(), |car| car.as_obstacle);
            let car = &mut self.cars[c];
            let next_obstacle_acceleration = intelligent_acceleration(car, &next_obstacle);

            // TODO: optimize, avoid nested loop
            let next_overlap_obstacle_acceleration = self.interaction_obstacles.iter()
                .find(|obstacle| obstacle.position > car.position)
                .map(|obstacle| intelligent_acceleration(car, obstacle));

            car.acceleration = next_obstacle_acceleration.min(next_overlap_obstacle_acceleration.unwrap_or(INFINITY));
        }

        for car in &mut self.cars {
            *car.position += dt * car.velocity;
            car.velocity = (car.velocity + dt * car.acceleration).min(car.max_velocity).max(0.0);
        }
        
        loop {
            let should_pop = self.cars.iter().rev().find(|car| *car.position > self.length).map(|car_over_end| {
                if let Some(next_overlap) = self.interactions.iter().find(|overlap| match overlap.kind {Next{..} => true, _ => false}) {
                    world.send(next_overlap.partner_lane, AddCar(car_over_end.offset_by(-self.length)));
                };
                car_over_end
            }).is_some();
            if should_pop {self.cars.pop();} else {break;}
        }

        for interaction in self.interactions.iter() {
            let mut cars = self.cars.iter();
            let mut send_obstacle = |obstacle: Obstacle| world.send(interaction.partner_lane, AddInteractionObstacle(obstacle));
            
            match interaction.kind {
                Overlap{start, end, partner_start, kind, ..} => {
                    let in_overlap = |car: &&LaneCar| *car.position > start && *car.position < end;
                    match kind {
                        Parallel => cars.filter(in_overlap).map(|car|
                            car.as_obstacle.offset_by(-start + partner_start)
                        ).foreach(send_obstacle),
                        Conflicting => if cars.find(in_overlap).is_some() {
                            (send_obstacle)(Obstacle{position: OrderedFloat(partner_start), velocity: 0.0})
                        }
                    }
                }
                Previous{start, partner_length} =>
                    if let Some(next_car) = cars.find(|car| *car.position > start) {
                        (send_obstacle)(next_car.as_obstacle.offset_by(-start + partner_length))
                    },
                Next{..} => {
                    //TODO: for looking backwards for merging lanes?
                }
            };
        }

        self.interaction_obstacles.clear();
    }
});

pub fn setup(system: &mut ActorSystem) {
    system.add_swarm::<Lane>(InMemory("lane_actors", 512 * 64, 10));
    system.add_inbox::<AddCar, Lane>(InMemory("add_car", 512, 4));
    system.add_inbox::<AddInteractionObstacle, Lane>(InMemory("add_interaction_obstacle", 512, 4));
    system.add_inbox::<Tick, Lane>(InMemory("tick", 512, 4));

    system.world().send_to_individual::<_, Simulation>(AddSimulatable(system.broadcast_id::<Lane>()));

    setup_scenario(system);
}

fn setup_scenario(system: &mut ActorSystem) {
    let mut world = system.world();

    let mut lane1 = world.create(Lane::new(
        CPath::new(vec![
            Segment::line(P2::new(0.0, 0.0), P2::new(300.0, 0.0)),
            Segment::arc_with_direction(P2::new(300.0, 0.0), V2::new(1.0, 0.0), P2::new(300.0, 100.0))
        ])
    ));

    let mut lane3 = world.create(Lane::new(
        CPath::new(vec![
            Segment::arc_with_direction(P2::new(0.0, 100.0), V2::new(-1.0, 0.0), P2::new(0.0, 0.0))
        ])
    ));

    let mut lane2 = world.create(Lane::new(
        CPath::new(vec![
            Segment::line(P2::new(300.0, 100.0), P2::new(0.0, 100.0))
        ])
    ));

    let mut overlapping_lane = world.create(Lane::new(
        CPath::new(vec![
            Segment::line(P2::new(300.0, 10.0), P2::new(0.0, -10.0))
        ])
    ));

    lane1.add_next_lane(lane2.id); lane2.add_previous_lane(lane1.id, lane1.length);
    lane2.add_next_lane(lane3.id); lane3.add_previous_lane(lane2.id, lane2.length);
    lane3.add_next_lane(lane1.id); lane1.add_previous_lane(lane3.id, lane3.length);

    lane1.interactions.push(Interaction{
        partner_lane: overlapping_lane.id,
        kind: Overlap{
            kind: Conflicting,
            start: 100.0,
            end: 200.0,
            partner_start: 100.0,
            partner_end: 200.0
        }
    });

    overlapping_lane.interactions.push(Interaction{
        partner_lane: lane1.id,
        kind: Overlap{
            kind: Conflicting,
            start: 100.0,
            end: 200.0,
            partner_start: 100.0,
            partner_end: 200.0
        }
    });

    let lane1_id = lane1.id;
    let lane2_id = lane2.id;
    let overlapping_lane_id = overlapping_lane.id;

    world.start(lane1);
    world.start(lane2);
    world.start(lane3);
    world.start(overlapping_lane);

    let n_cars = 10;
    for i in 0..n_cars {
        world.send(lane1_id, AddCar(LaneCar{
            as_obstacle: Obstacle {
                position: OrderedFloat(n_cars as f32 * 5.0 - (i as f32 * 5.0)),
                velocity: 0.0,
            },
            trip: ID::invalid(),
            acceleration: 1.0,
            max_velocity: 22.0
        }));
    }

    world.send(lane2_id, AddCar(LaneCar{
        as_obstacle: Obstacle {
            position: OrderedFloat(5.0),
            velocity: 0.0,
        },
        trip: ID::invalid(),
        acceleration: 1.0,
        max_velocity: 0.0
    }));

    world.send(overlapping_lane_id, AddCar(LaneCar{
        as_obstacle: Obstacle {
            position: OrderedFloat(60.0),
            velocity: 0.0,
        },
        trip: ID::invalid(),
        acceleration: 1.0,
        max_velocity: 10.0
    }));
}