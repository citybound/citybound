#![allow(dead_code)]
#[macro_use]
extern crate kay;
extern crate monet;
extern crate nalgebra;
extern crate compass;

mod ui;
mod geometry;
mod type_ids;
mod simulation;

mod lanes_and_cars;

fn main() {
    let renderer = ui::setup_window_and_renderer();
    
    let mut system = kay::ActorSystem::new();
    simulation::setup(&mut system);
    ui::setup(&mut system);

    lanes_and_cars::setup(&mut system);
    lanes_and_cars::ui::setup(&mut system);

    system.process_messages();

    'main: loop {
        match ui::process_events(&renderer.window) {
            false => {return},
            true => {}
        }

        for _i in 0..1000 {
            system.process_messages();
        }

        system.world().send(kay::ID::individual(type_ids::Recipients::Simulation as usize), simulation::Tick);

        for _i in 0..1000 {
            system.process_messages();
        }

        system.world().send(kay::ID::individual(type_ids::Recipients::RenderManager as usize), ui::StartFrame);

        for _i in 0..1000 {
            system.process_messages();
        }

        ui::finish_frame(&mut system, &renderer);
    }
}