#![feature(custom_derive, iter_rfold)]
#![allow(dead_code)]
// Enable this for memory tracking with Instruments/MacOS
// and for much better stacktraces for memory issues
#![feature(alloc_system)]
extern crate alloc_system;

extern crate ordered_float;
extern crate itertools;
extern crate rand;
extern crate fnv;
extern crate roaring;
extern crate backtrace;
extern crate uuid;

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
extern crate tungstenite;
extern crate rmpv;

use stagemaster::environment::Environment;

pub const ENV: &Environment = &Environment {
    name: "Citybound",
    author: "ae play",
    version: "0.4.0",
};

mod util;
mod simulation;
mod transport;
mod planning;
mod construction;
mod economy;
mod land_use;
mod ui_layers;
mod render_layers;
mod style;
mod browser_ui;

use kay::Actor;
use compact::CVec;
use monet::Grouper;
use transport::lane::{Lane, SwitchLane};
use transport::rendering::LaneRenderer;
use economy::households::family::Family;
use economy::households::grocery_shop::GroceryShop;
use economy::households::grain_farm::GrainFarm;
use economy::households::cow_farm::CowFarm;
use economy::households::mill::Mill;
use economy::households::bakery::Bakery;
use economy::households::neighboring_town_trade::NeighboringTownTrade;
use economy::households::tasks::TaskEndScheduler;
use land_use::buildings::rendering::BuildingRenderer;
use planning::PlanManager;
use construction::Construction;

fn setup_all(system: &mut kay::ActorSystem) {
    for setup_fn in &[
        stagemaster::setup,
        simulation::setup,
        ui_layers::setup,
        planning::setup,
        construction::setup,
        transport::setup,
        economy::setup,
        land_use::setup,
        browser_ui::setup,
    ] {
        setup_fn(system)
    }
}

fn main() {
    util::init::ensure_crossplatform_proper_thread(|| {
        util::init::first_time_open_wiki_release_page();

        let mut system = Box::new(kay::ActorSystem::new(util::init::networking_from_env_args()));

        setup_all(&mut system);

        let world = &mut system.world();

        system.networking_connect();

        let simulatables = vec![
            Lane::local_broadcast(world).into(),
            SwitchLane::local_broadcast(world).into(),
            Family::local_broadcast(world).into(),
            GroceryShop::local_broadcast(world).into(),
            GrainFarm::local_broadcast(world).into(),
            CowFarm::local_broadcast(world).into(),
            Mill::local_broadcast(world).into(),
            Bakery::local_broadcast(world).into(),
            NeighboringTownTrade::local_broadcast(world).into(),
            TaskEndScheduler::local_first(world).into(),
            Construction::global_first(world).into(),
        ];
        let simulation = simulation::spawn(world, simulatables);

        let renderables: CVec<_> = vec![
            LaneRenderer::global_broadcast(world).into(),
            Grouper::global_broadcast(world).into(),
            BuildingRenderer::global_broadcast(world).into(),
            PlanManager::global_first(world).into(),
        ].into();

        let machine_id = system.networking_machine_id();

        let (user_interface, renderer) = stagemaster::spawn(
            world,
            renderables,
            *ENV,
            util::init::build_window(machine_id.0),
            style::colors::GRASS,
        );

        simulation.add_to_ui(user_interface, world);
        ui_layers::spawn(world, user_interface);

        util::init::set_error_hook(user_interface, system.world());

        let plan_manager = planning::spawn(world, user_interface);
        construction::spawn(world);
        transport::spawn(world, simulation);
        economy::spawn(world, simulation, plan_manager);
        land_use::spawn(world, user_interface);

        util::init::print_version(user_interface, world);

        system.process_all_messages();

        let mut frame_counter = util::init::FrameCounter::new();

        let browser_ui = browser_ui::spawn(world);

        loop {
            frame_counter.start_frame();

            user_interface.process_events(world);
            browser_ui.process_messages(world);

            system.process_all_messages();

            if system.shutting_down {
                break;
            }

            simulation.progress(world);

            system.process_all_messages();

            renderer.prepare_render(world);

            system.process_all_messages();

            renderer.render(world);

            system.process_all_messages();

            system.networking_send_and_receive();

            frame_counter.print_fps(user_interface, world);

            util::init::print_instance_counts(&mut system, user_interface);
            util::init::print_network_turn(&mut system, user_interface);

            system.process_all_messages();

            user_interface.start_frame(world);

            system.process_all_messages();

            system.networking_finish_turn();
        }
    });
}
