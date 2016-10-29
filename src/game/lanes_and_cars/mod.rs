pub mod ui;
use core::geometry::CPath;
use kay::{ID, CVec, CDict, Recipient, World, ActorSystem, InMemory, Compact};
use compass::{FiniteCurve, Path, Segment, P2, V2};
use core::simulation::{Simulation, Tick, AddSimulatable};
use ordered_float::OrderedFloat;
use ::std::f32::INFINITY;
use ::std::ops::{Deref, DerefMut};

#[derive(Copy, Clone)]
struct Obstacle {
    position: OrderedFloat<f32>,
    velocity: f32
}

impl Obstacle {
    fn far_away() -> Obstacle {Obstacle{position: OrderedFloat(INFINITY), velocity: INFINITY}}
}

#[derive(Copy, Clone)]
struct LaneCar {
    trip: ID,
    as_obstacle: Obstacle,
    acceleration: f32,
    max_velocity: f32
}

impl Deref for LaneCar {
    type Target = Obstacle;

    fn deref(&self) -> &Obstacle {&self.as_obstacle}
}

impl DerefMut for LaneCar {
    fn deref_mut(&mut self) -> &mut Obstacle {&mut self.as_obstacle}
}

#[derive(Copy, Clone)]
struct ParallelOverlap {
    partner_lane: ID,
    start: OrderedFloat<f32>,
    end: OrderedFloat<f32>,
    partner_start: OrderedFloat<f32>,
    partner_end: OrderedFloat<f32>
}

derive_compact!{
    pub struct Lane {
        length: f32,
        path: CPath,
        next_lanes_with_obstacles: CDict<ID, Obstacle>,
        previous_lanes: CVec<ID>,
        overlaps: CVec<ParallelOverlap>,
        overlap_obstacles: CVec<Obstacle>,
        cars: CVec<LaneCar>
    }
}

impl Lane {
    fn new(path: CPath, next_lane: Option<ID>, previous_lane: Option<ID>) -> Self {
        let length = path.length();
        let mut next_lanes_with_obstacles = CDict::new();
        let mut previous_lanes = CVec::new();
        if let Some(id) = next_lane {next_lanes_with_obstacles.insert(id, Obstacle::far_away());}
        if let Some(id) = previous_lane {previous_lanes.push(id);}
        Lane {
            length: length,
            path: path,
            next_lanes_with_obstacles: next_lanes_with_obstacles,
            previous_lanes: previous_lanes,
            overlaps: CVec::new(),
            overlap_obstacles: CVec::new(),
            cars: CVec::new()
        }
    }

    fn add_next_lane(&mut self, next_lane: ID) {
        self.next_lanes_with_obstacles.insert(next_lane, Obstacle::far_away());
    }
}

#[derive(Copy, Clone)]
struct AddCar(LaneCar);

#[derive(Copy, Clone)]
struct UpdateObstacleOnNextLane(ID, Obstacle);

#[derive(Copy, Clone)]
struct AddOverlapObstacle(Obstacle);

recipient!(Lane, (&mut self, world: &mut World, self_id: ID) {
    AddCar: &AddCar(car) => {
        self.cars.insert(0, car);
    },

    Tick: &Tick{dt} => {
        let first_obstacle_on_any_next_lane = self.next_lanes_with_obstacles.pairs.iter()
            .min_by_key(|&&(_id, first_obstacle)| {first_obstacle.position})
            .map_or(Obstacle::far_away(), |&(_id, first_obstacle)| {Obstacle{
                position: OrderedFloat(*first_obstacle.position + self.length),
                velocity: first_obstacle.velocity
            }});

        if self.cars.len() >= 2 {
            for c in 0..self.cars.len() {
                let next_car = if c + 1 < self.cars.len() {*self.cars[c + 1]} else {first_obstacle_on_any_next_lane};
                let car = &mut self.cars[c];
                let next_car_acceleration = intelligent_driver_acceleration(car, &next_car);

                // TODO: optimize, avoid nested loop
                let next_overlap_obstacle = self.overlap_obstacles.iter().find(|obstacle| obstacle.position > car.position);
                let next_overlap_obstacle_acceleration = match next_overlap_obstacle {
                    Some(obstacle) => intelligent_driver_acceleration(car, obstacle),
                    None => INFINITY
                };

                car.acceleration = next_car_acceleration.min(next_overlap_obstacle_acceleration);
            }
        }

        for car in &mut self.cars {
            *car.position += dt * car.velocity;
            car.velocity = car.max_velocity.min(car.velocity + dt * car.acceleration).max(0.0);
        }
        
        while self.cars.len() > 0 {
            let mut last_car = self.cars[self.cars.len() - 1];
            if *last_car.position > self.length {
                *last_car.position -= self.length;
                if let Some(next_lane) = self.next_lanes_with_obstacles.keys().next() {
                    world.send(next_lane, AddCar(last_car));
                }
                self.cars.pop();
            } else {break;}
        }

        let first_obstacle = match &self.cars.first() {
            &Some(ref car) => *car,
            &None => &first_obstacle_on_any_next_lane
        };
        for previous_lane in self.previous_lanes.iter() {
            world.send(*previous_lane, UpdateObstacleOnNextLane(self_id, *first_obstacle));
        }

        for overlap in self.overlaps.iter() {
            for car in self.cars.iter().filter(|car| car.position > overlap.start && car.position < overlap.end) {
                world.send(overlap.partner_lane, AddOverlapObstacle(Obstacle{
                    position: OrderedFloat(*car.position - *overlap.start + *overlap.partner_start),
                    velocity: car.velocity
                }));
            }
        }

        self.overlap_obstacles.clear();
    },

    UpdateObstacleOnNextLane: &UpdateObstacleOnNextLane(next_lane_id, first_obstacle) => {
        self.next_lanes_with_obstacles.insert(next_lane_id, first_obstacle);
    },

    AddOverlapObstacle: &AddOverlapObstacle(obstacle) => {
        self.overlap_obstacles.push(obstacle);
    }
});

fn intelligent_driver_acceleration(car: &LaneCar, obstacle: &Obstacle) -> f32 {
    // http://en.wikipedia.org/wiki/Intelligent_driver_model

    let car_length = 4.0;
    let acceleration = 5.0;
	let comfortable_breaking_deceleration : f32 = 4.0;
	let max_deceleration : f32 = 14.0;
	let desired_velocity = car.max_velocity;
	let safe_time_headway = 1.0;
	let acceleration_exponent = 4.0;
	let minimum_spacing = 10.0;

	let net_distance = *obstacle.position - *car.position - car_length;
	let velocity_difference = car.velocity - obstacle.velocity;

	let s_star = minimum_spacing + 0.0f32.max(car.velocity * safe_time_headway
		+ (car.velocity * velocity_difference / (2.0 * (acceleration * comfortable_breaking_deceleration).sqrt())));

    let result_acceleration = (-max_deceleration).max(acceleration * (
		1.0
		- (car.velocity / desired_velocity).powf(acceleration_exponent)
		- (s_star / net_distance).powf(2.0)
	));

	result_acceleration
}

pub fn setup(system: &mut ActorSystem) {
    system.add_swarm::<Lane>(InMemory("lane_actors", 512 * 64, 10));
    system.add_inbox::<AddCar, Lane>(InMemory("add_car", 512, 4));
    system.add_inbox::<UpdateObstacleOnNextLane, Lane>(InMemory("update_obstacle_on_next_lane", 512, 4));
    system.add_inbox::<AddOverlapObstacle, Lane>(InMemory("add_overlap_obstacle", 512, 4));
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
        ]),
        None,
        None
    ));

    let mut lane3 = world.create(Lane::new(
        CPath::new(vec![
            Segment::arc_with_direction(P2::new(0.0, 100.0), V2::new(-1.0, 0.0), P2::new(0.0, 0.0))
        ]),
        Some(lane1.id),
        None
    ));

    let lane2 = world.create(Lane::new(
        CPath::new(vec![
            Segment::line(P2::new(300.0, 100.0), P2::new(0.0, 100.0))
        ]),
        Some(lane3.id),
        Some(lane1.id)
    ));

    let mut overlapping_lane = world.create(Lane::new(
        CPath::new(vec![
            Segment::line(P2::new(0.0, -10.0), P2::new(300.0, 10.0))
        ]),
        None,
        None
    ));

    lane1.add_next_lane(lane2.id);
    lane1.previous_lanes.push(lane3.id);
    lane3.previous_lanes.push(lane2.id);

    lane1.overlaps.push(ParallelOverlap{
        partner_lane: overlapping_lane.id,
        start: OrderedFloat(100.0),
        end: OrderedFloat(200.0),
        partner_start: OrderedFloat(100.0),
        partner_end: OrderedFloat(200.0),
    });

    overlapping_lane.overlaps.push(ParallelOverlap{
        partner_lane: lane1.id,
        start: OrderedFloat(100.0),
        end: OrderedFloat(200.0),
        partner_start: OrderedFloat(100.0),
        partner_end: OrderedFloat(200.0),
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
            position: OrderedFloat(80.0),
            velocity: 0.0,
        },
        trip: ID::invalid(),
        acceleration: 1.0,
        max_velocity: 10.0
    }));
}