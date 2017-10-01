extern crate open;

use kay::{ActorSystem, World, Networking};
use monet::glium::glutin::WindowBuilder;
use stagemaster::UserInterfaceID;
use std::any::Any;
use std::net::SocketAddr;
use std::time::Instant;

pub fn first_time_open_wiki_release_page() {
    let mut dir = ::std::env::temp_dir();
    dir.push("cb_seen_wiki.txt");
    if !::std::path::Path::new(&dir).exists() {
        let url = "https://github.com/citybound/citybound/wiki/Road-&-Traffic-Prototype-1.2";
        if let Err(_err) = open::that(url) {
            println!("Please open {:?} in your browser!", url);
        };
        ::std::fs::File::create(dir).expect("should be able to create tmp file");
    }
}

pub fn create_init_callback() -> Box<Fn(Box<Any>, &mut World)> {
    Box::new(|error: Box<Any>, world: &mut ::kay::World| {
        let ui_id = UserInterfaceID::local_first(world);
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
    })
}

pub fn networking_from_env_args() -> Networking {
    println!("{:?}", ::std::env::args().collect::<Vec<_>>());

    if ::std::env::args().nth(1).is_none() {
        Networking::new(0, vec!["127.0.0.1:3500".parse().unwrap()])
    } else {
        let machine_id: u8 = ::std::env::args()
            .nth(1)
            .expect("expected machine_id")
            .parse()
            .unwrap();
        let network: Vec<SocketAddr> = ::std::env::args()
            .nth(2)
            .expect("expected network")
            .split(',')
            .map(|addr_str| addr_str.parse().unwrap())
            .collect();

        Networking::new(machine_id, network)
    }

}

pub fn build_window(machine_id: u8) -> WindowBuilder {
    WindowBuilder::new()
        .with_title(format!("Citybound (machine {})", machine_id))
        .with_dimensions(1920, 1080)
        .with_multitouch()
}

pub fn print_version(user_interface: UserInterfaceID, world: &mut World) {
    user_interface.add_debug_text(
        "Version".chars().collect(),
        ::ENV.version.chars().collect(),
        [0.0, 0.0, 0.0, 1.0],
        true,
        world,
    );
}

pub fn print_instance_counts(system: &mut ActorSystem, user_interface: UserInterfaceID) {
    user_interface.add_debug_text(
        "Number of actors".chars().collect(),
        system.get_instance_counts().as_str().chars().collect(),
        [0.0, 0.0, 0.0, 1.0],
        false,
        &mut system.world(),
    );
}

pub fn print_network_turn(system: &mut ActorSystem, user_interface: UserInterfaceID) {
    user_interface.add_debug_text(
        "Networking turn".chars().collect(),
        system
            .networking_debug_all_n_turns()
            .as_str()
            .chars()
            .collect(),
        [0.0, 0.0, 0.0, 1.0],
        false,
        &mut system.world(),
    );
}

pub struct FrameCounter {
    last_frame: Instant,
    elapsed_ms_collected: Vec<f32>,
}

impl FrameCounter {
    pub fn new() -> FrameCounter {
        FrameCounter {
            last_frame: Instant::now(),
            elapsed_ms_collected: Vec::new(),
        }
    }

    pub fn start_frame(&mut self) {
        let elapsed_ms = self.last_frame.elapsed().as_secs() as f32 * 1000.0 +
            self.last_frame.elapsed().subsec_nanos() as f32 / 10.0E5;

        self.elapsed_ms_collected.push(elapsed_ms);

        if self.elapsed_ms_collected.len() > 10 {
            self.elapsed_ms_collected.remove(0);
        }

        self.last_frame = Instant::now();
    }

    pub fn print_fps(&self, user_interface: UserInterfaceID, world: &mut World) {
        let avg_elapsed_ms = self.elapsed_ms_collected.iter().sum::<f32>() /
            (self.elapsed_ms_collected.len() as f32);

        user_interface.add_debug_text(
            "Frame".chars().collect(),
            format!("{:.1} FPS", 1000.0 * 1.0 / avg_elapsed_ms)
                .as_str()
                .chars()
                .collect(),
            [0.0, 0.0, 0.0, 0.5],
            false,
            world,
        );
    }
}