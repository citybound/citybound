#![feature(custom_derive, conservative_impl_trait)]
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![allow(dead_code)]
// Enable this for memory tracking with Instruments/MacOS
// and for much better stacktraces for memory issues
// ![feature(alloc_system)]
// extern crate alloc_system;

extern crate ordered_float;
extern crate itertools;
extern crate random;
extern crate fnv;
extern crate roaring;
extern crate open;

extern crate compact;
#[macro_use]
extern crate compact_macros;
extern crate kay;
#[macro_use]
extern crate kay_macros;
extern crate monet;
extern crate descartes;
extern crate stagemaster;
#[macro_use]
extern crate imgui;
#[macro_use]
extern crate serde_derive;
extern crate serde;

use stagemaster::environment::Environment;

pub const ENV: &'static Environment = &Environment {
    name: "Citybound",
    author: "ae play",
    version: "0.1.3",
};

mod core;
mod transport;
mod economy;

use compact::CVec;
use monet::{RendererID, RenderableID};
use monet::glium::{DisplayBuild, glutin};
use core::simulation::SimulatableID;
use stagemaster::UserInterfaceID;
use transport::lane::{Lane, TransferLane};
use transport::rendering::lane_thing_collector::ThingCollectorID;
use transport::planning::current_plan::CurrentPlanID;
use economy::households::family::FamilyID;
use economy::households::tasks::TaskEndSchedulerID;
use economy::buildings::Building;
use kay::swarm::Swarm;
use std::any::Any;

fn main() {
    let mut dir = ::std::env::temp_dir();
    dir.push("cb_seen_wiki.txt");
    if !::std::path::Path::new(&dir).exists() {
        let url = "https://github.com/citybound/citybound/wiki/Road-&-Traffic-Prototype-1.2";
        if let Err(_err) = open::that(url) {
            println!("Please open {:?} in your browser!", url);
        };
        ::std::fs::File::create(dir).expect("should be able to create tmp file");
    }

    let mut system = kay::ActorSystem::new(Box::new(|error: Box<Any>, world| {
        // TODO: ugly/wrong
        let ui_id = UserInterfaceID::broadcast(world);
        let message = match error.downcast::<String>() {
            Ok(string) => (*string),
            Err(any) => {
                match any.downcast::<&'static str>() {
                    Ok(static_str) => (*static_str).to_string(),
                    Err(_) => "Weird error type".to_string(),
                }
            }
        };
        println!("Simulation Panic!\n{:?}", message);
        ui_id.add_debug_text(
            "SIMULATION PANIC".chars().collect(),
            message.as_str().chars().collect(),
            [1.0, 0.0, 0.0, 1.0],
            true,
            world,
        );
        ui_id.on_panic(world);
    }));

    let simulatables = vec![
        system.id::<Swarm<Lane>>().broadcast(),
        system.id::<Swarm<TransferLane>>().broadcast(),
    ].into_iter()
        .map(|id| SimulatableID { _raw_id: id })
        .chain(vec![
            FamilyID::broadcast(&mut system.world()).into(),
            // TODO: ugly/wrong
            TaskEndSchedulerID::broadcast(&mut system.world())
                .into(),
        ])
        .collect();
    let simulation = core::simulation::setup(&mut system, simulatables);

    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(1024, 512)
        .with_multitouch()
        .with_vsync()
        .build_glium()
        .unwrap();

    let renderables: CVec<_> = vec![
        system.id::<Swarm<Lane>>().broadcast(),
        system.id::<Swarm<TransferLane>>().broadcast(),
        system.id::<Swarm<Building>>().broadcast(),
    ].into_iter()
        .map(|id| RenderableID { _raw_id: id })
        .chain(vec![
            ThingCollectorID::broadcast(&mut system.world()).into(),
            // TODO: ugly/wrong
            CurrentPlanID::broadcast(&mut system.world()).into(),
        ])
        .collect();
    let (user_interface, renderer) = stagemaster::setup(&mut system, renderables, *ENV, window);

    transport::setup(&mut system, user_interface, renderer);
    economy::setup(&mut system, user_interface, simulation);

    let mut last_frame = std::time::Instant::now();

    user_interface.add_debug_text(
        "Version".chars().collect(),
        ENV.version.chars().collect(),
        [0.0, 0.0, 0.0, 1.0],
        true,
        &mut system.world(),
    );

    system.process_all_messages();

    let mut elapsed_ms_collected = Vec::<f32>::new();

    loop {
        let elapsed_ms = last_frame.elapsed().as_secs() as f32 * 1000.0 +
            last_frame.elapsed().subsec_nanos() as f32 / 10.0E5;
        elapsed_ms_collected.push(elapsed_ms);
        if elapsed_ms_collected.len() > 10 {
            elapsed_ms_collected.remove(0);
        }
        let avg_elapsed_ms = elapsed_ms_collected.iter().sum::<f32>() /
            (elapsed_ms_collected.len() as f32);
        user_interface.add_debug_text(
            "Frame".chars().collect(),
            format!("{:.1} FPS", 1000.0 * 1.0 / avg_elapsed_ms)
                .as_str()
                .chars()
                .collect(),
            [0.0, 0.0, 0.0, 0.5],
            false,
            &mut system.world(),
        );
        last_frame = std::time::Instant::now();

        let subactor_counts = system.get_subactor_counts();
        user_interface.add_debug_text(
            "Number of actors".chars().collect(),
            subactor_counts.as_str().chars().collect(),
            [0.0, 0.0, 0.0, 1.0],
            false,
            &mut system.world(),
        );

        user_interface.process_events(&mut system.world());

        system.process_all_messages();

        simulation.do_tick(&mut system.world());

        system.process_all_messages();

        renderer.render(&mut system.world());

        system.process_all_messages();

        user_interface.start_frame(&mut system.world());

        system.process_all_messages();
    }
}
