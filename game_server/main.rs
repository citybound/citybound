extern crate citybound_common;
use citybound_common::*;

use kay::Actor;
use transport::lane::{Lane, SwitchLane};
use economy::households::family::Family;
use economy::households::grocery_shop::GroceryShop;
use economy::households::grain_farm::GrainFarm;
use economy::households::cow_farm::CowFarm;
use economy::households::mill::Mill;
use economy::households::bakery::Bakery;
use economy::households::neighboring_town_trade::NeighboringTownTrade;
use economy::households::tasks::TaskEndScheduler;
use construction::Construction;

fn main() {
    util::init::ensure_crossplatform_proper_thread(|| {
        util::init::first_time_open_wiki_release_page();

        let mut system = Box::new(kay::ActorSystem::new(kay::Networking::new(
            0,
            vec!["127.0.0.1:9999", "ws-client"],
            3_000,
            2,
            5,
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
        util::init::set_error_hook();

        let plan_manager = planning::spawn(world);
        construction::spawn(world);
        transport::spawn(world, simulation);
        economy::spawn(world, simulation, plan_manager);
        system.process_all_messages();

        let mut frame_counter = util::init::FrameCounter::new();
        let mut skip_turns = 0;

        loop {
            frame_counter.start_frame();

            system.process_all_messages();

            if system.shutting_down {
                break;
            }

            if skip_turns == 0 {
                simulation.progress(world);

                system.process_all_messages();
            }

            system.networking_send_and_receive();

            system.process_all_messages();

            if skip_turns > 0 {
                skip_turns -= 1;
            //println!("Skipping! {} left", skip_turns);
            } else {
                let maybe_should_skip = system.networking_finish_turn();
                if let Some(should_skip) = maybe_should_skip {
                    skip_turns = should_skip.min(100);
                }
            }

            //frame_counter.print_fps();
            frame_counter.sleep_if_faster_than(120);
        }
    });
}
