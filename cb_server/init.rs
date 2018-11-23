extern crate open;
extern crate backtrace;
extern crate clap;

use std::time::{Instant, Duration};

pub fn print_start_message(version: &str, network_config: &NetworkConfig) {
    let my_host = format!(
        "{}:{}",
        match network_config.mode.as_str() {
            "local" => "localhost",
            "lan" => "<your LAN IP>",
            "internet" => "<your public IP>",
            _ => unreachable!(),
        },
        network_config.serve_host_port.split(':').nth(1).unwrap(),
    );

    println!("  {: ^41}  ", format!("Citybound {}", version.trim()));
    println!("  {: ^41}  ", "please connect with your browser");
    println!("╭───────────────────────────────────────────╮");
    println!("│ {: ^41} │", format!("http://{}", my_host));
    println!("╰───────────────────────────────────────────╯");
}

#[derive(Clone)]
pub struct NetworkConfig {
    pub mode: String,
    pub serve_host_port: String,
    pub bind_sim: String,
    pub batch_msg_bytes: usize,
    pub ok_turn_dist: usize,
    pub skip_ratio: usize,
}

pub fn match_cmd_line_args(version: &str) -> NetworkConfig {
    use self::clap::{Arg, App};
    let matches = App::new("citybound")
        .version(version.trim())
        .author("ae play (Anselm Eickhoff)")
        .about("The city is us.")
        .arg(
            Arg::with_name("mode")
                .long("mode")
                .value_name("local/lan/internet")
                .display_order(0)
                .possible_values(&["local", "lan", "internet"])
                .default_value("local")
                .help("Where to expose the simulation. Sets defaults other settings."),
        )
        .arg(
            Arg::with_name("bind")
                .long("bind")
                .value_name("host:port")
                .default_value_ifs(&[
                    ("mode", Some("local"), "localhost:1234"),
                    ("mode", Some("lan"), "0.0.0.0:1234"),
                    ("mode", Some("internet"), "0.0.0.0:1234"),
                ])
                .help("Address and port to serve the browser UI from"),
        )
        .arg(
            Arg::with_name("bind-sim")
                .long("bind-sim")
                .value_name("host:port")
                .default_value_ifs(&[
                    ("mode", Some("local"), "localhost:9999"),
                    ("mode", Some("lan"), "0.0.0.0:9999"),
                    ("mode", Some("internet"), "0.0.0.0:9999"),
                ])
                .help("Address and port to accept connections to the simulation from"),
        )
        .arg(
            Arg::with_name("batch-msg-b")
                .long("batch-msg-bytes")
                .value_name("n-bytes")
                .default_value("5000")
                .help("How many bytes of simulation messages to batch"),
        )
        .arg(
            Arg::with_name("ok-turn-dist")
                .long("ok-turn-dist")
                .value_name("n-turns")
                .default_value_ifs(&[
                    ("mode", Some("local"), "2"),
                    ("mode", Some("lan"), "10"),
                    ("mode", Some("internet"), "30"),
                ])
                .help("How many network turns client/server can be behind before skipping"),
        )
        .arg(
            Arg::with_name("skip-ratio")
                .long("skip-ratio")
                .value_name("n-turns")
                .default_value("5")
                .help("How many network turns to skip if server/client are ahead"),
        )
        .get_matches();

    NetworkConfig {
        serve_host_port: matches.value_of("bind").unwrap().to_owned(),
        bind_sim: matches.value_of("bind-sim").unwrap().to_owned(),
        mode: matches.value_of("mode").unwrap().to_owned(),
        batch_msg_bytes: matches.value_of("batch-msg-b").unwrap().parse().unwrap(),
        ok_turn_dist: matches.value_of("ok-turn-dist").unwrap().parse().unwrap(),
        skip_ratio: matches.value_of("skip-ratio").unwrap().parse().unwrap(),
    }
}

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

use std::panic::{set_hook, PanicInfo};
use self::backtrace::Backtrace;
use std::fs::File;
use std::io::Write;

pub fn set_error_hook() {
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
    });

    set_hook(unsafe { ::std::mem::transmute(callback) });
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

    pub fn sleep_if_faster_than(&self, fps: usize) {
        let ideal_frame_duration = Duration::from_millis((1000.0 / (fps as f32)) as u64);

        if let Some(pos_difference) = ideal_frame_duration.checked_sub(self.last_frame.elapsed()) {
            ::std::thread::sleep(pos_difference);
        }
    }
}

impl Default for FrameCounter {
    fn default() -> Self {
        Self::new()
    }
}
