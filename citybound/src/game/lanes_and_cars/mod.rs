pub mod ui;
use core::geometry::CPath;
use kay::{ID, CVec, CDict, Recipient, World, ActorSystem, InMemory, Compact};
use compass::{FiniteCurve, Path, Segment, P2, V2};
use core::simulation::{Simulation, Tick, AddSimulatable};
use ordered_float::OrderedFloat;

#[derive(Copy, Clone)]
struct LaneCar {
    trip: ID,
    position: f32,
    velocity: f32,
    acceleration: f32,
    max_velocity: f32
}

#[derive(Copy, Clone)]
struct ObstacleOnNextLane {
    position: OrderedFloat<f32>,
    velocity: f32
}

impl ObstacleOnNextLane {
    fn far_away() -> ObstacleOnNextLane {
        ObstacleOnNextLane{
            position: OrderedFloat(::std::f32::INFINITY),
            velocity: ::std::f32::INFINITY
        }
    }
}

derive_compact!{
    pub struct Lane {
        length: f32,
        path: CPath,
        next_lanes_with_obstacles: CDict<ID, ObstacleOnNextLane>,
        previous_lanes: CVec<ID>,
        cars: CVec<LaneCar>
    }
}

impl Lane {
    fn new(path: CPath, next_lane: Option<ID>, previous_lane: Option<ID>) -> Self {
        let length = path.length();
        let mut next_lanes_with_obstacles = CDict::new();
        let mut previous_lanes = CVec::new();
        if let Some(id) = next_lane {next_lanes_with_obstacles.insert(id, ObstacleOnNextLane::far_away());}
        if let Some(id) = previous_lane {previous_lanes.push(id);}
        Lane {
            length: length,
            path: path,
            next_lanes_with_obstacles: next_lanes_with_obstacles,
            previous_lanes: previous_lanes,
            cars: CVec::new()
        }
    }

    fn add_next_lane(&mut self, next_lane: ID) {
        self.next_lanes_with_obstacles.insert(next_lane, ObstacleOnNextLane::far_away());
    }
}

#[derive(Copy, Clone)]
struct AddCar(LaneCar);

#[derive(Copy, Clone)]
struct UpdateObstacleOnNextLane(ID, ObstacleOnNextLane);

recipient!(Lane, (&mut self, world: &mut World, self_id: ID) {
    AddCar: &AddCar(car) => {
        self.cars.insert(0, car);
    },

    Tick: &Tick{dt} => {
        let first_obstacle_on_any_next_lane = self.next_lanes_with_obstacles.pairs.iter()
            .min_by_key(|&&(_id, first_obstacle)| {first_obstacle.position})
            .map_or(ObstacleOnNextLane::far_away(), |&(_id, first_obstacle)| {ObstacleOnNextLane{
                position: OrderedFloat(first_obstacle.position.as_ref() + self.length),
                velocity: first_obstacle.velocity
            }});

        if self.cars.len() >= 2 {
            for c in 0..(self.cars.len() - 1) {
                let next_car = self.cars[c + 1];
                let car = &mut self.cars[c];
                car.acceleration = intelligent_driver_acceleration(
                    car.position, car.velocity, car.max_velocity,
                    next_car.position, next_car.velocity
                );
            }
        }

        if let Some(last_car) = self.cars.last_mut() {
            last_car.acceleration = intelligent_driver_acceleration(
                last_car.position, last_car.velocity, last_car.max_velocity,
                *first_obstacle_on_any_next_lane.position.as_ref(), first_obstacle_on_any_next_lane.velocity
            )
        }

        for car in &mut self.cars {
            car.position += dt * car.velocity;
            car.velocity = car.max_velocity.min(car.velocity + dt * car.acceleration).max(0.0);
        }
        
        while self.cars.len() > 0 {
            let mut last_car = self.cars[self.cars.len() - 1];
            if last_car.position > self.length {
                last_car.position -= self.length;
                let next_lane = self.next_lanes_with_obstacles.keys().next().unwrap();
                world.send(next_lane, AddCar(last_car));
                self.cars.pop();
            } else {break;}
        }

        let first_obstacle = match self.cars.first() {
            Some(car) => ObstacleOnNextLane{
                position: OrderedFloat(car.position),
                velocity: car.velocity
            },
            None => first_obstacle_on_any_next_lane
        };
        for previous_lane in self.previous_lanes.iter() {
            world.send(*previous_lane, UpdateObstacleOnNextLane(self_id, first_obstacle));
        }
    },

    UpdateObstacleOnNextLane: &UpdateObstacleOnNextLane(next_lane_id, first_obstacle) => {
        self.next_lanes_with_obstacles.insert(next_lane_id, first_obstacle);
    }
});

fn intelligent_driver_acceleration(car_position: f32, car_velocity: f32, car_max_velocity: f32, obstacle_position: f32, obstacle_velocity: f32) -> f32 {
    // http://en.wikipedia.org/wiki/Intelligent_driver_model

    let car_length = 4.0;
    let acceleration = 14.0;
	let comfortable_breaking_deceleration : f32 = 15.0;
	let max_deceleration : f32 = 45.0;
	let desired_velocity = car_max_velocity;
	let safe_time_headway = 1.0;
	let acceleration_exponent = 4.0;
	let minimum_spacing = 10.0;

	let net_distance = obstacle_position - car_position - car_length;
	let velocity_difference = car_velocity - obstacle_velocity;

	let s_star = minimum_spacing + 0.0f32.max(car_velocity * safe_time_headway
		+ (car_velocity * velocity_difference / (2.0 * (acceleration * comfortable_breaking_deceleration).sqrt())));

    let result_acceleration = (-max_deceleration).max(acceleration * (
		1.0
		- (car_velocity / desired_velocity).powf(acceleration_exponent)
		- (s_star / net_distance).powf(2.0)
	));

	result_acceleration
}

pub fn setup(system: &mut ActorSystem) {
    system.add_swarm::<Lane>(InMemory("lane_actors", 512 * 64, 10));
    system.add_inbox::<AddCar, Lane>(InMemory("add_car", 512, 4));
    system.add_inbox::<UpdateObstacleOnNextLane, Lane>(InMemory("update_obstacle_on_next_lane", 512, 4));
    system.add_inbox::<Tick, Lane>(InMemory("tick", 512, 4));

    system.world().send_to_individual::<_, Simulation>(AddSimulatable(system.broadcast_id::<Lane>()));

    setup_scenario(system);
}

fn setup_scenario(system: &mut ActorSystem) {
    let mut world = system.world();

    let mut actor1 = world.create(Lane::new(
        CPath::new(vec![
            Segment::line(P2::new(0.0, 0.0), P2::new(300.0, 0.0)),
            Segment::arc_with_direction(P2::new(300.0, 0.0), V2::new(1.0, 0.0), P2::new(300.0, 100.0))
        ]),
        None,
        None
    ));

    let mut actor3 = world.create(Lane::new(
        CPath::new(vec![
            Segment::arc_with_direction(P2::new(0.0, 100.0), V2::new(-1.0, 0.0), P2::new(0.0, 0.0))
        ]),
        Some(actor1.id),
        None
    ));

    let actor2 = world.create(Lane::new(
        CPath::new(vec![
            Segment::line(P2::new(300.0, 100.0), P2::new(0.0, 100.0))
        ]),
        Some(actor3.id),
        Some(actor1.id)
    ));

    actor1.add_next_lane(actor2.id);
    actor1.previous_lanes.push(actor3.id);
    actor3.previous_lanes.push(actor2.id);

    let actor1_id = actor1.id;
    let actor2_id = actor2.id;

    world.start(actor1);
    world.start(actor2);
    world.start(actor3);

    let n_cars = 10;
    for i in 0..n_cars {
        world.send(actor1_id, AddCar(LaneCar{
            position: n_cars as f32 * 5.0 - (i as f32 * 5.0),
            trip: ID::invalid(),
            velocity: 0.0,
            acceleration: 1.0,
            max_velocity: 22.0
        }));
    }

    world.send(actor2_id, AddCar(LaneCar{
        position: 5.0,
        trip: ID::invalid(),
        velocity: 0.0,
        acceleration: 1.0,
        max_velocity: 3.0
    }));
}