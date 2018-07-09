extern crate open;

use kay::{ActorSystem, World, Networking};
use monet::glium::glutin::WindowBuilder;
use stagemaster::UserInterfaceID;
use std::net::SocketAddr;
use std::time::Instant;

pub fn ensure_crossplatform_proper_thread<F: Fn() -> () + Send + 'static>(callback: F) {
    // Makes sure that:
    // a) on Windows we use a dummy thread with manually set stack size
    // b) on Mac/Linux we use the main thread, because we have to create the UI there

    if cfg!(windows) {
        let dummy_thread = ::std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(callback)
            .unwrap();
        dummy_thread.join().unwrap();
    } else {
        callback();
    }
}

pub fn first_time_open_wiki_release_page() {
    let mut dir = ::std::env::temp_dir();
    dir.push("cb_seen_wiki.txt");
    if !::std::path::Path::new(&dir).exists() {
        let url = "https://github.com/citybound/citybound/wiki/TODO";
        if let Err(_err) = open::that(url) {
            println!("Please open {:?} in your browser!", url);
        };
        ::std::fs::File::create(dir).expect("should be able to create tmp file");
    }
}

use std::panic::{set_hook, PanicInfo};
use backtrace::Backtrace;
use std::fs::File;
use std::io::Write;

pub fn set_error_hook(ui_id: UserInterfaceID, mut world: World) {
    let callback: Box<FnMut(&PanicInfo)> = Box::new(move |panic_info| {
        let title = "SIMULATION BROKE :(";

        let message = match panic_info.payload().downcast_ref::<String>() {
            Some(string) => (string.clone()),
            None => match panic_info.payload().downcast_ref::<&'static str>() {
                Some(static_str) => (*static_str).to_string(),
                None => "Weird error type".to_string(),
            },
        };

        let backtrace = Backtrace::new();
        let location = format!(
            "at {}, line {}",
            panic_info.location().map(|l| l.file()).unwrap_or("unknown"),
            panic_info.location().map(|l| l.line()).unwrap_or(0)
        );

        let body = format!(
            "WHAT HAPPENED:\n{}\n\nWHERE IT HAPPENED:\n{}\n\nWHERE EXACTLY:\n{:?}",
            message, location, backtrace
        );

        let small_body = format!(
            "{}\n{}\nDETAILS IN cb_last_error.txt (AUTO-OPENED)",
            message, location
        );

        let report_guide = "HOW TO REPORT \
                            BUGS:\nhttps://github.\
                            com/citybound/citybound/blob/master/CONTRIBUTING.md#reporting-bugs";

        let mut error_file_path = ::std::env::temp_dir();
        error_file_path.push("cb_last_error.txt");

        println!(
            "{}\n\n{}\n\nALSO SEE {:?} (AUTO-OPENED)",
            title, body, error_file_path
        );

        {
            if let Ok(mut file) = File::create(&error_file_path) {
                let file_content = format!("{}\n\n{}\n\n{}", title, report_guide, body);
                let file_content = file_content.replace("\n", "\r\n");

                file.write_all(file_content.as_bytes())
                    .expect("Error writing error file, lol");
            };
        }

        open::that(error_file_path).expect("Couldn't open error file");

        ui_id.add_debug_text(
            title.to_owned().into(),
            small_body.into(),
            [1.0, 0.0, 0.0, 1.0],
            true,
            &mut world,
        );
        ui_id.on_panic(&mut world);
    });

    set_hook(unsafe { ::std::mem::transmute(callback) });
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
        "Version".to_owned().into(),
        ::ENV.version.to_owned().into(),
        [0.0, 0.0, 0.0, 1.0],
        true,
        world,
    );
}

pub fn print_instance_counts(system: &mut ActorSystem, user_interface: UserInterfaceID) {
    user_interface.add_debug_text(
        "Number of actors".to_owned().into(),
        system.get_instance_counts().into(),
        [0.0, 0.0, 0.0, 1.0],
        false,
        &mut system.world(),
    );
}

pub fn print_network_turn(system: &mut ActorSystem, user_interface: UserInterfaceID) {
    user_interface.add_debug_text(
        "Networking turn".to_owned().into(),
        system.networking_debug_all_n_turns().into(),
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
        let elapsed_ms = self.last_frame.elapsed().as_secs() as f32 * 1000.0
            + self.last_frame.elapsed().subsec_nanos() as f32 / 10.0E5;

        self.elapsed_ms_collected.push(elapsed_ms);

        if self.elapsed_ms_collected.len() > 10 {
            self.elapsed_ms_collected.remove(0);
        }

        self.last_frame = Instant::now();
    }

    pub fn print_fps(&self, user_interface: UserInterfaceID, world: &mut World) {
        let avg_elapsed_ms = self.elapsed_ms_collected.iter().sum::<f32>()
            / (self.elapsed_ms_collected.len() as f32);

        user_interface.add_debug_text(
            "Frame".to_owned().into(),
            format!("{:.1} FPS", 1000.0 * 1.0 / avg_elapsed_ms).into(),
            [0.0, 0.0, 0.0, 0.5],
            false,
            world,
        );
    }
}
