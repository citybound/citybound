#![feature(proc_macro)]
#![allow(dead_code)]
#![feature(plugin)]
#![feature(conservative_impl_trait)]
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
use game::lanes_and_cars::lane_thing_collector::LaneThingCollector;
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
        system.individual_id::<LaneThingCollector>(),
        system.individual_id::<CurrentPlan>(),
        system.broadcast_id::<RoadStrokeNodeInteractable>()
    ];
    let window = core::ui::setup_window_and_renderer(&mut system, renderables);

    let mut simulation_panicked = false;
    let mut last_frame = std::time::Instant::now();

    system.process_all_messages();

    loop {
        println!("----\n FRAME: {} ms", last_frame.elapsed().subsec_nanos() as f32 / 10.0E6);
        last_frame = std::time::Instant::now();
        if !core::ui::process_events(&window) {return}

        if simulation_panicked {
            system.clear_all_clearable_messages();
            system.process_all_messages();
        } else {
            let simulation_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                system.process_all_messages();
                
                Simulation::id() << Tick{dt: SECONDS_PER_TICK, current_tick: 0};

                system.process_all_messages();

                Renderer::id() << Control::Render;

                system.process_all_messages();
            }));
            simulation_panicked = simulation_result.is_err();
            if simulation_panicked {
                system.clear_all_clearable_messages();
                let msg = match simulation_result.unwrap_err().downcast::<String>() {
                    Ok(string) => (*string),
                    Err(any) => match any.downcast::<&'static str>() {
                        Ok(static_str) => (*static_str).to_string(),
                        Err(_) => "Weird error type".to_string()
                    }
                };
                println!("Simulation Panic!\n{:?}", msg);
            }
        }

        Renderer::id() << Control::Submit;

        system.process_all_messages();
    }
}