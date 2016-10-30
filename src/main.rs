#![feature(core_intrinsics)]
#![allow(dead_code)]
extern crate nalgebra;
#[macro_use]
pub extern crate glium;
extern crate glium_text;
extern crate ordered_float;
extern crate itertools;

#[macro_use]
mod kay;
mod compass;
mod monet;

mod core;
mod game;

const SECONDS_PER_TICK : f32 = 1.0 / 20.0;

fn main() {    
    let mut system = kay::ActorSystem::new();
    core::simulation::setup(&mut system);

    game::setup(&mut system);
    game::setup_ui(&mut system);

    let window = core::ui::setup_window_and_renderer(&mut system);

    system.process_all_messages();

    'main: loop {
        match core::ui::process_events(&window) {
            false => {return},
            true => {}
        }

        system.process_all_messages();

        system.world().send_to_individual::<_, core::simulation::Simulation>(core::simulation::Tick{dt: SECONDS_PER_TICK});

        system.process_all_messages();

        system.world().send_to_individual::<_, monet::Renderer>(monet::Render);

        system.process_all_messages();

        system.world().send_to_individual::<_, monet::Renderer>(monet::Submit);
    }
}