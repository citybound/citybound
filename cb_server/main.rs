extern crate cb_simulation;
use cb_simulation::*;

#[macro_use]
extern crate rust_embed_flag;

const VERSION: &str = include_str!("../.version");

mod init;
mod browser_ui_server;

fn main() {
    let network_config = init::match_cmd_line_args(VERSION);

    init::print_start_message(VERSION, &network_config);

    let network_config_2 = network_config.clone();
    ::std::thread::spawn(move || {
        browser_ui_server::start_browser_ui_server(VERSION, network_config_2);
    });

    init::ensure_crossplatform_proper_thread(move || {
        let mut system = Box::new(kay::ActorSystem::new(kay::Networking::new(
            0,
            vec![network_config.bind_sim.clone(), "ws-client".to_owned()],
            network_config.batch_msg_bytes,
            network_config.ok_turn_dist,
            network_config.skip_ratio,
        )));
        init::set_error_hook();

        setup_common(&mut system);
        system.networking_connect();

        let world = &mut system.world();

        let time = cb_simulation::spawn_for_server(world);

        system.process_all_messages();

        let mut frame_counter = init::FrameCounter::new();
        let mut skip_turns = 0;

        loop {
            frame_counter.start_frame();

            system.process_all_messages();

            if system.shutting_down {
                break;
            }

            if skip_turns == 0 {
                time.progress(world);
                system.process_all_messages();
            }

            system.networking_send_and_receive();
            system.process_all_messages();

            if skip_turns > 0 {
                skip_turns -= 1;
            } else {
                let maybe_should_skip = system.networking_finish_turn();
                if let Some(should_skip) = maybe_should_skip {
                    skip_turns = should_skip.min(100);
                }
            }

            frame_counter.sleep_if_faster_than(120);
        }
    });
}
