#![feature(custom_derive, plugin, conservative_impl_trait)]
#![plugin(clippy)]
#![allow(dead_code)]
#![allow(no_effect, unnecessary_operation)]
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

use stagemaster::environment::Environment;

pub const ENV: Environment = Environment {
    name: "Citybound",
    author: "ae play",
    version: "0.1.2",
};

mod core;
mod game;

use monet::{Renderer, Control};
use monet::glium::{DisplayBuild, glutin};
use core::simulation::{Simulation, Tick};
use stagemaster::{ProcessEvents, StartFrame, UserInterface, AddDebugText};
use game::lanes_and_cars::lane::{Lane, TransferLane};
use game::lanes_and_cars::rendering::{LaneAsphalt, LaneMarker, TransferLaneMarkerGaps};
use game::lanes_and_cars::rendering::lane_thing_collector::ThingCollector;
use game::lanes_and_cars::planning::current_plan::CurrentPlan;
use kay::Actor;
use kay::swarm::Swarm;
use std::any::Any;

const SECONDS_PER_TICK: f32 = 1.0 / 20.0;

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

    let mut system = kay::ActorSystem::create_the_system(Box::new(|error: Box<Any>| {
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
        UserInterface::id() <<
        AddDebugText {
            key: "SIMULATION PANIC".chars().collect(),
            text: message.as_str().chars().collect(),
            color: [1.0, 0.0, 0.0, 1.0],
            persistent: true,
        };
    }));

    game::setup();
    game::setup_ui();

    let simulatables = vec![Swarm::<Lane>::all(), Swarm::<TransferLane>::all()];
    core::simulation::setup(simulatables);

    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(1024, 512)
        .with_multitouch()
        .with_vsync()
        .build_glium()
        .unwrap();

    let renderables = vec![Swarm::<Lane>::all(),
                           Swarm::<TransferLane>::all(),
                           ThingCollector::<LaneAsphalt>::id(),
                           ThingCollector::<LaneMarker>::id(),
                           ThingCollector::<TransferLaneMarkerGaps>::id(),
                           CurrentPlan::id()];
    stagemaster::setup(renderables, &ENV, &window);

    let mut last_frame = std::time::Instant::now();

    UserInterface::id() <<
    AddDebugText {
        key: "Version".chars().collect(),
        text: ENV.version.chars().collect(),
        color: [1.0, 1.0, 1.0, 1.0],
        persistent: true,
    };

    system.process_all_messages();

    loop {
        UserInterface::id() <<
        AddDebugText {
            key: "Frame".chars().collect(),
            text: format!("{:.2} ms",
                          last_frame.elapsed().as_secs() as f32 * 1000.0 +
                          last_frame.elapsed().subsec_nanos() as f32 / 10.0E5)
                .as_str()
                .chars()
                .collect(),
            color: [1.0, 1.0, 1.0, 0.5],
            persistent: false,
        };
        last_frame = std::time::Instant::now();

        UserInterface::id() << ProcessEvents;

        system.process_all_messages();

        Simulation::id() <<
        Tick {
            dt: SECONDS_PER_TICK,
            current_tick: 0,
        };

        system.process_all_messages();

        Renderer::id() << Control::Render;

        system.process_all_messages();

        UserInterface::id() << StartFrame;

        system.process_all_messages();
    }
}
