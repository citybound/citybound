#![allow(dead_code)]
extern crate world_record;
extern crate monet;

use std::path::PathBuf;
use std::thread;
use std::sync::mpsc::channel;

mod models;
mod steps;
mod simulation;

fn main() {
    let (to_simulation, from_renderer) = channel::<()>();
    let (to_renderer, from_simulation) = channel::<String>();
    
    let renderer_listener = move |past: &models::State, future: &models::State| {
        match from_renderer.try_recv() {
            Ok(_) => {
                println!("creating renderer state...");
                to_renderer.send(
                   format!("Simulation frame: {}", past.core.header.ticks)
                ).unwrap();
            },
            Err(_) => {}
        };
        
    };
    
    thread::Builder::new().name("simulation".to_string()).spawn(|| {
        let mut simulation = simulation::Simulation::<models::State>::new(
            PathBuf::from("savegames/dev"),
            vec! [Box::new(steps::tick)],
            vec! [Box::new(renderer_listener)]
        );
    
       loop {
           let duration_to_sleep = simulation.step();
           thread::sleep(duration_to_sleep);
       }
    }).unwrap();
    
    monet::main_loop(to_simulation, from_simulation);
}