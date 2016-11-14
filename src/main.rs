#![feature(proc_macro)]
#![allow(dead_code)]
#![feature(plugin)]
#![plugin(clippy)]
#![allow(no_effect, unnecessary_operation)]
extern crate ordered_float;
extern crate itertools;

extern crate kay;
#[macro_use]
extern crate kay_macros;
extern crate monet;
extern crate descartes;

mod core;
mod game;

use monet::{Renderer, Control};
use core::simulation::{Simulation, Tick};
use game::lanes_and_cars::{Lane, TransferLane};
use game::lanes_and_cars::planning::{CurrentPlan, RoadStrokeNodeInteractable};
use kay::Individual;

const SECONDS_PER_TICK : f32 = 1.0 / 20.0;

fn main() {    
    let mut system = kay::ActorSystem::new();
    unsafe {
        kay::THE_SYSTEM = &mut system as *mut kay::ActorSystem;
    }

    game::setup(&mut system);
    game::setup_ui(&mut system);

    let simulatables = vec![
        system.broadcast_id::<Lane>(),
        system.broadcast_id::<TransferLane>()
    ];
    core::simulation::setup(&mut system, simulatables);

    let renderables = vec![
        system.broadcast_id::<Lane>(),
        system.broadcast_id::<TransferLane>(),
        system.individual_id::<CurrentPlan>(),
        system.broadcast_id::<RoadStrokeNodeInteractable>()
    ];
    let window = core::ui::setup_window_and_renderer(&mut system, renderables);

    system.process_all_messages();

    loop {
        if !core::ui::process_events(&window) {return}

        system.process_all_messages();

        Simulation::id() << Tick{dt: SECONDS_PER_TICK};

        system.process_all_messages();

        Renderer::id() << Control::Render;

        system.process_all_messages();

        Renderer::id() << Control::Submit;
    }
}