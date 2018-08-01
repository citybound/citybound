#![feature(proc_macro)]

#[macro_use]
extern crate stdweb;
use stdweb::js_export;

extern crate kay;
use kay::ActorSystem;

extern crate citybound_common;
use citybound_common::*;

use std::panic;

// TODO: not thread safe for now
static mut SYSTEM: *mut ActorSystem = 0 as *mut ActorSystem;

#[js_export]
pub fn start() {
    panic::set_hook(Box::new(|info| console!(error, info.to_string())));

    js!{ console.log("Before setup") }

    let mut system =
        kay::ActorSystem::new(kay::Networking::new(1, vec!["localhost:9999", "ws-client"], 50_000, 10, 1));
    setup_all(&mut system);

    system.networking_connect();

    let browser_ui_id = citybound_common::browser_ui::BrowserUIID::spawn(&mut system.world());

    system.process_all_messages();

    js!{ console.log("After setup") }

    let mut main_loop = MainLoop { browser_ui_id };

    unsafe { SYSTEM = Box::into_raw(Box::new(system)) };

    main_loop.frame();
}

#[derive(Copy, Clone)]
struct MainLoop {
    browser_ui_id: citybound_common::browser_ui::BrowserUIID,
}

impl MainLoop {
    fn frame(&mut self) {
        let system = unsafe { &mut *SYSTEM };
        let world = &mut system.world();

        system.process_all_messages();

        self.browser_ui_id.on_frame(world);
        system.process_all_messages();

        system.networking_send_and_receive();

        js!{
            window.cbclient.setState(oldState => update(oldState, {system: {networkingTurns: {"$set": 
                @{system.networking_debug_all_n_turns()}
            }}}));

            window.cbclient.onFrame();
        }

        system.process_all_messages();

        let maybe_sleep = system.networking_finish_turn();
        let mut next = self.clone();
        
        let next_step_in_ms = match maybe_sleep {
            None => {
                ::stdweb::web::window().request_animation_frame(move |_| next.frame());
            },
            Some(duration) => {
                let sleep_nanos = duration.subsec_nanos() as u64;
                let sleep_ms = (1000*1000*1000 * duration.as_secs() + sleep_nanos)/(1000 * 1000);

                ::stdweb::web::set_timeout(move || {
                    ::stdweb::web::window().request_animation_frame(move |_| next.frame());
                }, sleep_ms as u32);
            }
        };
    }
}

mod planning_browser;
pub use planning_browser::*;