#![allow(dead_code)]
#[macro_use]
extern crate kay;
extern crate monet;
extern crate nalgebra;
extern crate compass;

use monet::glium::DisplayBuild;
use monet::glium::glutin;
use kay::{ID, Known, Message, Recipient, World, CVec, ActorSystem, Swarm, Inbox, MemChunker, Compact};

#[path = "../resources/car.rs"]
mod car;

#[derive(Copy, Clone)]
struct LaneCar {
    trip: ID,
    position: f32
}

derive_compact!{
    struct Lane {
        length: f32,
        y_position: f32,
        next: Option<ID>,
        previous: Option<ID>,
        cars: CVec<LaneCar>
    }
}
impl Known for Lane {fn type_id() -> usize {13}}

#[derive(Copy, Clone)]
struct AddCar(LaneCar);

impl Message for AddCar {}
impl Known for AddCar {fn type_id() -> usize {42}}

#[derive(Copy, Clone)]
struct Tick;
impl Message for Tick {}
impl Known for Tick {fn type_id() -> usize {43}}

#[derive(Copy, Clone)]
struct Render{scene_id: ID}
impl Message for Render {}
impl Known for Render {fn type_id() -> usize {44}}

#[derive(Copy, Clone)]
struct RenderedCar{x: f32, y: f32, z: f32}
impl Message for RenderedCar {}
impl Known for RenderedCar {fn type_id() -> usize {44}}

impl Recipient<RenderedCar> for monet::Scene {
    fn receive(&mut self, car: &RenderedCar, _world: &mut World) {
        let instances = &mut self.swarms.get_mut("cars").unwrap().instances;
        instances.push(monet::WorldPosition{world_position: [car.x, car.y, car.z]});
    }
}

impl Recipient<AddCar> for Lane {
    fn receive(&mut self, message: &AddCar, _world: &mut World) {
        self.cars.insert(0, message.0);
    }
}

impl Recipient<Tick> for Lane {
    fn receive(&mut self, _message: &Tick, world: &mut World) {
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
}

impl Recipient<Render> for Lane {
    fn receive(&mut self, render: &Render, world: &mut World) {
        for car in &self.cars {
            world.send(render.scene_id, RenderedCar{x: car.position, y: self.y_position, z: 0.0})
        }
    }
}

fn main() {
    
    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(512, 512)
        .with_multitouch()
        .with_vsync().build_glium().unwrap();

    let renderer = monet::Renderer::new(&window);

    let mut system = ActorSystem::new();

    system.add_swarm::<Lane>(Swarm::new(MemChunker::new("lane_actors", 512 * 64), 10));
    system.add_inbox::<AddCar, Lane>(Inbox::new(MemChunker::new("add_car", 512), 4));
    system.add_inbox::<Tick, Lane>(Inbox::new(MemChunker::new("tick", 512), 4));
    system.add_inbox::<Render, Lane>(Inbox::new(MemChunker::new("render", 512), 4));

    {
        let mut scene = monet::Scene::new();
        scene.swarms.insert("cars", monet::Swarm::new(car::create(), Vec::new()));
        scene.eye.position *= 30.0;

        system.add_individual(scene, 111);
        system.add_individual_inbox::<RenderedCar, monet::Scene>(Inbox::new(MemChunker::new("rendered_car", 512 * 8), 4), 111);
    }

    let mut world = system.world();

    let mut actor1 = world.create(Lane {
        length: 2500.0,
        y_position: 0.0,
        previous: None,
        next: None,
        cars: CVec::new()
    });

    let mut actor2 = world.create(Lane {
        length: 1000.0,
        y_position: 10.0,
        previous: None,
        next: None,
        cars: CVec::new()
    });

    let actor3 = world.create(Lane {
        length: 100.0,
        y_position: 20.0,
        previous: None,
        next: Some(actor1.id),
        cars: CVec::new()
    });

    actor1.next = Some(actor2.id);
    actor2.next = Some(actor3.id);

    let (actor1_id, actor2_id, actor3_id) = (actor1.id, actor2.id, actor3.id);

    world.start(actor1);
    world.start(actor2);
    world.start(actor3);

    for i in 0..500 {
        world.send(actor1_id, AddCar(LaneCar{position: 2500.0 - (i as f32 * 5.0), trip: ID::invalid()}));
    }

    'main: loop {
        for event in window.poll_events() {}

        {
            let scene = system.get_individual_mut::<monet::Scene>(111);
            scene.swarms.get_mut("cars").unwrap().instances.clear();
        }
        
        world.send(actor1_id, Tick);
        world.send(actor2_id, Tick);
        world.send(actor3_id, Tick);

        world.send(actor1_id, Render{scene_id: ID::individual(111)});
        world.send(actor2_id, Render{scene_id: ID::individual(111)});
        world.send(actor3_id, Render{scene_id: ID::individual(111)});

        for _i in 0..1000 {
            system.process_messages();
        }

        let scene = system.get_individual::<monet::Scene>(111);
        renderer.draw(scene);

    }
}