#[path = "../resources/car.rs"]
mod car;

extern crate nalgebra;
use nalgebra::{Vector3, Point3};

// coordinate system:
// Z = UP
// Y = NORTH
// X = EAST

pub fn render (past: &::models::State, future: &::models::State) -> ::monet::Scene {
    println!("creating renderer state...");
    let mut scene = ::monet::Scene::new();
    scene.things.insert("car", car::create());
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