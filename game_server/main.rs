extern crate citybound_common;
use citybound_common::*;

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

fn main() {
    util::init::ensure_crossplatform_proper_thread(|| {
        util::init::first_time_open_wiki_release_page();

        let mut system = Box::new(kay::ActorSystem::new(kay::Networking::new(
            0,
            vec!["localhost:9999", "ws-client"],
            30,
            1,
        )));

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

        loop {
            frame_counter.start_frame();

            user_interface.process_events(world);

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

            let maybe_sleep = system.networking_finish_turn();

            if let Some(duration) = maybe_sleep {
                ::std::thread::sleep(duration);
            }
        }
    });
}
