#![feature(custom_derive, conservative_impl_trait)]
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![allow(dead_code)]
// Enable this for memory tracking with Instruments/MacOS
// and for much better stacktraces for memory issues
//#![feature(alloc_system)]
//extern crate alloc_system;

extern crate ordered_float;
extern crate itertools;
extern crate rand;
extern crate fnv;
extern crate roaring;
extern crate backtrace;

extern crate compact;
#[macro_use]
extern crate compact_macros;
extern crate kay;
extern crate monet;
extern crate descartes;
extern crate stagemaster;
#[macro_use]
extern crate imgui;
#[macro_use]
extern crate serde_derive;

use stagemaster::environment::Environment;

pub const ENV: &Environment = &Environment {
    name: "Citybound",
    author: "ae play",
    version: "0.3.0",
};

mod core;
mod transport;
mod planning;
mod economy;
mod land_use;
mod ui_layers;
mod style;

use kay::Actor;
use compact::CVec;
use monet::Grouper;
use transport::lane::{Lane, TransferLane};
use transport::rendering::LaneRenderer;
use planning::plan_manager::PlanManager;
use economy::households::family::Family;
use economy::households::grocery_shop::GroceryShop;
use economy::households::grain_farm::GrainFarm;
use economy::households::cow_farm::CowFarm;
use economy::households::mill::Mill;
use economy::households::bakery::Bakery;
use economy::households::neighboring_town_trade::NeighboringTownTrade;
use economy::households::tasks::TaskEndScheduler;
use land_use::buildings::BuildingSpawner;
use land_use::buildings::rendering::BuildingRenderer;

fn main() {
    core::init::ensure_crossplatform_proper_thread(|| {
        core::init::first_time_open_wiki_release_page();

        let mut system = Box::new(kay::ActorSystem::new(
            core::init::networking_from_env_args(),
        ));

        let world = &mut system.world();

        system.networking_connect();

        let simulatables = vec![
            Lane::local_broadcast(world).into(),
            TransferLane::local_broadcast(world).into(),
            Family::local_broadcast(world).into(),
            GroceryShop::local_broadcast(world).into(),
            GrainFarm::local_broadcast(world).into(),
            CowFarm::local_broadcast(world).into(),
            Mill::local_broadcast(world).into(),
            Bakery::local_broadcast(world).into(),
            NeighboringTownTrade::local_broadcast(world).into(),
            TaskEndScheduler::local_first(world).into(),
            BuildingSpawner::local_first(world).into(),
        ];
        let simulation = core::simulation::setup(&mut system, simulatables);

        let renderables: CVec<_> = vec![
            LaneRenderer::global_broadcast(world).into(),
            Grouper::global_broadcast(world).into(),
            PlanManager::global_broadcast(world).into(),
            BuildingRenderer::global_broadcast(&mut system.world())
                .into(),
        ].into();

        let machine_id = system.networking_machine_id();

        let (user_interface, renderer) = stagemaster::setup(
            &mut system,
            renderables,
            *ENV,
            core::init::build_window(machine_id),
            style::colors::GRASS,
        );

        simulation.add_to_ui(user_interface, world);
        ui_layers::setup(&mut system, user_interface);

        core::init::set_error_hook(user_interface, system.world());

        let materialized_reality = planning::setup(&mut system, user_interface, renderer);
        transport::setup(&mut system, simulation);
        economy::setup(&mut system);
        land_use::setup(
            &mut system,
            user_interface,
            simulation,
            materialized_reality,
        );

        core::init::print_version(user_interface, world);

        system.process_all_messages();

        let mut frame_counter = core::init::FrameCounter::new();

        loop {
            frame_counter.start_frame();

            user_interface.process_events(world);

            system.process_all_messages();

            if system.shutting_down {
                break;
            }

            simulation.progress(world);

            system.process_all_messages();

            renderer.render(world);

            system.process_all_messages();

            system.networking_send_and_receive();

            frame_counter.print_fps(user_interface, world);

            core::init::print_instance_counts(&mut system, user_interface);
            core::init::print_network_turn(&mut system, user_interface);

            system.process_all_messages();

            user_interface.start_frame(world);

            system.process_all_messages();

            system.networking_finish_turn();
        }
    });
}
