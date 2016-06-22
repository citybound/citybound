#![allow(dead_code)]
extern crate world_record;
extern crate monet;
extern crate nalgebra;

use std::path::PathBuf;
use std::thread;
use std::sync::mpsc::channel;
use monet::glium::DisplayBuild;
use monet::glium::glutin;

mod models;
mod steps;
mod simulation;
mod renderer;
mod input;

fn main() {
    let (input_to_simulation, from_input) = channel::<Vec<input::InputCommand>>();
    let (renderer_to_simulation, from_renderer) = channel::<()>();
    let (to_renderer, from_simulation) = channel::<monet::Scene>();
    
    let input_step = move |past: &models::State, future: &mut models::State| {
        loop {match from_input.try_recv() {
            Ok(inputs) => for input in inputs {
                input::apply_input_command(input, past, future)
            },
            Err(_) => {break}
        }}
    };

    let renderer_listener = move |past: &models::State, future: &models::State| {
        match from_renderer.try_recv() {
            Ok(_) => {to_renderer.send(renderer::render(past, future)).unwrap();},
            Err(_) => {}
        };     
    };
    
    thread::Builder::new().name("simulation".to_string()).spawn(|| {
        let mut simulation = simulation::Simulation::<models::State>::new(
            PathBuf::from("savegames/dev"),
            vec! [Box::new(input_step), Box::new(steps::tick)],
            vec! [Box::new(renderer_listener)]
        );
    
       loop {
           let duration_to_sleep = simulation.step();
           thread::sleep(duration_to_sleep);
       }
    }).unwrap();
    
    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(512, 512)
        .with_multitouch()
        .with_vsync().build_glium().unwrap();

    let renderer = monet::Renderer::new(&window);
    let mut input_state = input::InputState::default();

    'main: loop {
        match input::interpret_events(window.poll_events(), &mut input_state) {
            input::InputResult::Exit => break 'main,
            input::InputResult::ContinueWithInputCommands(inputs) => {
                input_to_simulation.send(inputs).unwrap()
            }
        }

        renderer_to_simulation.send(()).unwrap();
        let scene = from_simulation.recv().unwrap();
        println!("rendering...");

        renderer.draw(scene);
    }
}