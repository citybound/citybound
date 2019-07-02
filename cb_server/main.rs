extern crate cb_simulation;
use cb_simulation::kay::TypedID;

#[macro_use]
extern crate rust_embed_flag;

extern crate ctrlc;

const VERSION: &str = include_str!("../.version");

mod init;
mod browser_ui_server;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    let (network_config, city_folder) = init::match_cmd_line_args(VERSION);

    init::print_start_message(VERSION, &network_config);

    let running = Arc::new(AtomicBool::new(true));
    let running_2 = running.clone();

    ctrlc::set_handler(move || {
        running_2.store(false, Ordering::SeqCst);
        println!("Stopping Citybound safely...");
    })
    .expect("Error setting Ctrl-C handler");

    let network_config_2 = network_config.clone();
    ::std::thread::spawn(move || {
        browser_ui_server::start_browser_ui_server(VERSION, network_config_2);
    });

    init::ensure_crossplatform_proper_thread(move || {
        let version_file_path = ::std::path::PathBuf::from(&city_folder).join("__cb_version.txt");
        let savegame_exists = if let Ok(version) = std::fs::read_to_string(&version_file_path) {
            println!("Loading from savegame {}...", &city_folder);
            if version != VERSION {
                println!("POTENTIALLY INCOMPATIBLE SAVEGAME!")
            }
            true
        } else {
            println!("Savegame folder {} not found, creating...", city_folder);
            std::fs::create_dir_all(&city_folder).expect("Couldn't create savegame folder.");
            ::std::fs::write(version_file_path, VERSION).expect("Could not write savegame version");
            false
        };

        let mut system = Box::new(cb_simulation::kay::ActorSystem::new_mmap_persisted(
            cb_simulation::kay::Networking::new(
                0,
                vec![network_config.bind_sim.clone(), "ws-client".to_owned()],
                network_config.batch_msg_bytes,
                network_config.ok_turn_dist,
                network_config.skip_ratio,
            ),
            &city_folder,
        ));
        init::set_error_hook();

        cb_simulation::setup_common(&mut system);
        system.networking_connect();

        let world = &mut system.world();

        let time = if savegame_exists {
            cb_simulation::cb_time::actors::TimeID::global_first(world)
        } else {
            cb_simulation::spawn_for_server(world)
        };
        println!(
            "Simulation running.\n(You can stop this process at any point and the savegame should \
             be fine)"
        );

        system.process_all_messages();

        let mut frame_counter = init::FrameCounter::new();
        let mut skip_turns = 0;

        while running.load(Ordering::SeqCst) {
            frame_counter.start_frame();

            system.process_all_messages();

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
