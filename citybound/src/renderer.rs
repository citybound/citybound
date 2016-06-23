#[path = "../resources/car.rs"]
mod car;

extern crate nalgebra;
use nalgebra::{Vector3, Point3};
use ::monet::WorldPosition;

// coordinate system:
// Z = UP
// Y = NORTH
// X = EAST

pub fn render (past: &::models::State, future: &::models::State) -> ::monet::Scene {
    println!("creating renderer state...");
    let mut scene = ::monet::Scene::new();

    let n_cars_sqrt = 400;
    let mut positions = Vec::<WorldPosition>::with_capacity(n_cars_sqrt * n_cars_sqrt);
    for i in 0..(n_cars_sqrt * n_cars_sqrt) {
        positions.push(WorldPosition{world_position: [
            (i / n_cars_sqrt) as f32 * 5.0,
            (i % n_cars_sqrt) as f32 * 5.0,
            5.0 * (past.core.header.time as f32 * 0.01).sin() * ((i/n_cars_sqrt) as f32).sin()
        ]})
    }

    let car_swarm = ::monet::Swarm::new(car::create(), positions);

    scene.swarms.insert("car", car_swarm);
    scene.debug_text = format!("Simulation frame: {}", past.core.header.ticks);

    let eye = past.ui_state.eye;
    scene.eye.target = Point3::new(eye.target.x, eye.target.y, 0.0);
    scene.eye.up = Vector3::<f32>::z();
    scene.eye.position = scene.eye.target + eye.distance * Vector3::new(
        eye.inclination.cos() * -eye.azimuth.sin(),
        eye.inclination.cos() * -eye.azimuth.cos(),
        eye.inclination.sin()
    );
    scene
}