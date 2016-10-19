pub mod ui;
use geometry::CPath;
use kay::{ID, CVec, Known, Recipient, Message, World, ActorSystem, InMemory, Compact};
use compass::{FiniteCurve, Path, Segment, P2, V2};
use simulation::Tick;

#[derive(Copy, Clone)]
struct LaneCar {
    trip: ID,
    position: f32
}

derive_compact!{
    struct Lane {
        length: f32,
        path: CPath,
        next: Option<ID>,
        cars: CVec<LaneCar>
    }
}

impl Lane {
    fn new(path: CPath, next: Option<ID>) -> Self {
        let length = path.length();
        Lane {
            length: length,
            path: path,
            next: next,
            cars: CVec::new()
        }
    }
}

impl Known for Lane {fn type_id() -> usize {::type_ids::Recipients::Lane as usize}}

#[derive(Copy, Clone)]
struct AddCar(LaneCar);
message!(AddCar, ::type_ids::Messages::AddCar);

recipient!(Lane, (&mut self, world: &mut World, self_id: ID) {
    AddCar: &AddCar(car) => {
        self.cars.insert(0, car);
    },

    Tick: _ => {
        for car in &mut self.cars {
            car.position += 1.25;
        }
        while self.cars.len() > 0 {
            let mut last_car = self.cars[self.cars.len() - 1];
            if last_car.position > self.length {
                last_car.position -= self.length;
                world.send(self.next.unwrap(), AddCar(last_car));
                self.cars.pop();
            } else {break;}
        }
    }
});

pub fn setup(system: &mut ActorSystem) {
    system.add_swarm::<Lane>(InMemory("lane_actors", 512 * 64, 10));
    system.add_inbox::<AddCar, Lane>(InMemory("add_car", 512, 4));
    system.add_inbox::<Tick, Lane>(InMemory("tick", 512, 4));

    system.world().send(ID::individual(::type_ids::Recipients::Simulation as usize), ::simulation::AddSimulatable(ID::broadcast::<Lane>()));

    setup_scenario(system);
}

fn setup_scenario(system: &mut ActorSystem) {
    let mut world = system.world();

    let mut actor1 = world.create(Lane::new(
        CPath::new(vec![
            Segment::line(P2::new(0.0, 0.0), P2::new(300.0, 0.0)),
            Segment::arc_with_direction(P2::new(300.0, 0.0), V2::new(1.0, 0.0), P2::new(300.0, 100.0))
        ]),
        None
    ));

    let actor3 = world.create(Lane::new(
        CPath::new(vec![
            Segment::arc_with_direction(P2::new(0.0, 100.0), V2::new(-1.0, 0.0), P2::new(0.0, 0.0))
        ]),
        Some(actor1.id)
    ));

    let actor2 = world.create(Lane::new(
        CPath::new(vec![
            Segment::line(P2::new(300.0, 100.0), P2::new(0.0, 100.0))
        ]),
        Some(actor3.id)
    ));

    actor1.next = Some(actor2.id);

    let actor1_id = actor1.id;

    world.start(actor1);
    world.start(actor2);
    world.start(actor3);

    let n_cars = 10;
    for i in 0..n_cars {
        world.send(actor1_id, AddCar(LaneCar{position: n_cars as f32 * 5.0 - (i as f32 * 5.0), trip: ID::invalid()}));
    }
}