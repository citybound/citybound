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

pub const ENV: &'static Environment = &Environment {
    name: "Citybound",
    author: "ae play",
    version: "0.2.0",
};

mod core;
mod transport;
mod planning;
mod economy;

use compact::CVec;
use monet::GrouperID;
use transport::lane::{LaneID, TransferLaneID};
use transport::rendering::LaneRendererID;
use planning::plan_manager::PlanManagerID;
use economy::households::family::FamilyID;
use economy::households::crop_farm::CropFarmID;
use economy::households::tasks::TaskEndSchedulerID;
use economy::buildings::BuildingSpawnerID;
use economy::buildings::rendering::BuildingRendererID;

fn main() {
    core::init::ensure_crossplatform_proper_thread(|| {
        core::init::first_time_open_wiki_release_page();

        let mut system = Box::new(kay::ActorSystem::new(
            core::init::networking_from_env_args(),
        ));

        let world = &mut system.world();

        system.networking_connect();

        let simulatables = vec![
            LaneID::local_broadcast(world).into(),
            TransferLaneID::local_broadcast(world).into(),
            FamilyID::local_broadcast(world).into(),
            CropFarmID::local_broadcast(world).into(),
            TaskEndSchedulerID::local_first(world).into(),
            BuildingSpawnerID::local_first(world).into(),
        ];
        let simulation = core::simulation::setup(&mut system, simulatables);

        let renderables: CVec<_> = vec![
            LaneRendererID::global_broadcast(world).into(),
            GrouperID::global_broadcast(world).into(),
            PlanManagerID::global_broadcast(world).into(),
            BuildingRendererID::global_broadcast(&mut system.world())
                .into(),
        ].into();

        let machine_id = system.networking_machine_id();

        let (user_interface, renderer) = stagemaster::setup(
            &mut system,
            renderables,
            *ENV,
            core::init::build_window(machine_id),
            (0.6, 0.75, 0.4, 1.0)
        );

        simulation.add_to_ui(user_interface, world);

        core::init::set_error_hook(user_interface, system.world());

        let materialized_reality = planning::setup(&mut system, user_interface, renderer);
        transport::setup(&mut system, simulation);
        economy::setup(
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
