extern crate open;
use std::time::{Instant, Duration};

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
use backtrace::Backtrace;
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

    pub fn print_fps(&self) {
        let avg_elapsed_ms = self.elapsed_ms_collected.iter().sum::<f32>()
            / (self.elapsed_ms_collected.len() as f32);

        println!("Could achieve {:.1} FPS", 1000.0 * 1.0 / avg_elapsed_ms)
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
