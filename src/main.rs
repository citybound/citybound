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
mod game;

use monet::{RendererID, RenderableID};
use monet::glium::{DisplayBuild, glutin};
use core::simulation::{Simulation, DoTick};
use stagemaster::{ProcessEvents, StartFrame, UserInterface, AddDebugText, OnPanic};
use game::lanes_and_cars::lane::{Lane, TransferLane};
use game::lanes_and_cars::rendering::{LaneAsphalt, LaneMarker, TransferLaneMarkerGaps};
use game::lanes_and_cars::rendering::lane_thing_collector::ThingCollector;
use game::lanes_and_cars::planning::current_plan::CurrentPlan;
use game::economy::buildings::Building;
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
        let ui_id = world.id::<UserInterface>();
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
        world.send(
            ui_id,
            AddDebugText {
                key: "SIMULATION PANIC".chars().collect(),
                text: message.as_str().chars().collect(),
                color: [1.0, 0.0, 0.0, 1.0],
                persistent: true,
            },
        );
        world.send(ui_id, OnPanic);
    }));

    game::setup(&mut system);
    game::setup_ui(&mut system);

    let simulatables = vec![
        system.id::<Swarm<Lane>>().broadcast(),
        system.id::<Swarm<TransferLane>>().broadcast(),
    ];
    core::simulation::setup(&mut system, simulatables);

    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(1024, 512)
        .with_multitouch()
        .with_vsync()
        .build_glium()
        .unwrap();

    let renderables: Vec<_> = vec![
        system.id::<Swarm<Lane>>().broadcast(),
        system.id::<Swarm<TransferLane>>().broadcast(),
        system.id::<ThingCollector<LaneAsphalt>>(),
        system.id::<ThingCollector<LaneMarker>>(),
        system.id::<ThingCollector<TransferLaneMarkerGaps>>(),
        system.id::<CurrentPlan>(),
        system.id::<Swarm<Building>>().broadcast(),
    ].into_iter()
        .map(|id| RenderableID { _raw_id: id })
        .collect();
    stagemaster::setup(&mut system, renderables, ENV, &window);

    let mut last_frame = std::time::Instant::now();

    let ui_id = system.id::<UserInterface>();
    let sim_id = system.id::<Simulation>();
    // TODO: ugly/wrong
    let renderer_id = RendererID::broadcast(&mut system.world());

    system.send(
        ui_id,
        AddDebugText {
            key: "Version".chars().collect(),
            text: ENV.version.chars().collect(),
            color: [0.0, 0.0, 0.0, 1.0],
            persistent: true,
        },
    );

    system.process_all_messages();

    loop {
        system.send(
            ui_id,
            AddDebugText {
                key: "Frame".chars().collect(),
                text: format!(
                    "{:.2} ms",
                    last_frame.elapsed().as_secs() as f32 * 1000.0 +
                        last_frame.elapsed().subsec_nanos() as f32 / 10.0E5
                ).as_str()
                    .chars()
                    .collect(),
                color: [0.0, 0.0, 0.0, 0.5],
                persistent: false,
            },
        );
        last_frame = std::time::Instant::now();

        system.send(ui_id, ProcessEvents);

        system.process_all_messages();

        system.send(sim_id, DoTick);

        system.process_all_messages();

        renderer_id.render(&mut system.world());

        system.process_all_messages();

        system.send(ui_id, StartFrame);

        system.process_all_messages();
    }
}
