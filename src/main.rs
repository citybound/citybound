#![allow(dead_code)]
extern crate ordered_float;
extern crate itertools;

#[macro_use]
extern crate kay;
extern crate monet;
extern crate descartes;

mod core;
mod game;

use monet::{Renderer, Control};
use core::simulation::{Simulation, Tick};
use game::lanes_and_cars::{Lane, TransferLane};

const SECONDS_PER_TICK : f32 = 1.0 / 20.0;

fn main() {    
    let mut system = kay::ActorSystem::new();

    game::setup(&mut system);
    game::setup_ui(&mut system);

    let simulatables = vec![
        system.broadcast_id::<Lane>(),
        system.broadcast_id::<TransferLane>()
    ];
    core::simulation::setup(&mut system, simulatables);

    let renderables = vec![
        system.broadcast_id::<Lane>(),
        system.broadcast_id::<TransferLane>()
    ];
    let window = core::ui::setup_window_and_renderer(&mut system, renderables);

    system.process_all_messages();

    'main: loop {
        match core::ui::process_events(&window, &mut system.world()) {
            false => {return},
            true => {}
        }

        system.process_all_messages();

        system.world().send_to_individual::<Simulation, _>(Tick{dt: SECONDS_PER_TICK});

        system.process_all_messages();

        system.world().send_to_individual::<Renderer, _>(Control::Render);

        system.process_all_messages();

        system.world().send_to_individual::<Renderer, _>(Control::Submit);
    }
}