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

    let mut system = kay::ActorSystem::new(kay::Networking::new(
        1,
        vec!["localhost:9999", "ws-client"],
        50_000,
        10,
        1,
    ));
    setup_all(&mut system);

    system.networking_connect();

    let browser_ui_id = citybound_common::browser_ui::BrowserUIID::spawn(&mut system.world());

    system.process_all_messages();

    js!{ console.log("After setup") }

    let mut main_loop = MainLoop {
        browser_ui_id,
        skip_turns: 0,
    };

    unsafe { SYSTEM = Box::into_raw(Box::new(system)) };

    main_loop.frame();
}

#[derive(Copy, Clone)]
struct MainLoop {
    browser_ui_id: citybound_common::browser_ui::BrowserUIID,
    skip_turns: u32,
}

impl MainLoop {
    fn frame(&mut self) {
        let system = unsafe { &mut *SYSTEM };
        let world = &mut system.world();

        system.networking_send_and_receive();

        if self.skip_turns == 0 {
            system.process_all_messages();

            self.browser_ui_id.on_frame(world);
            system.process_all_messages();

            system.networking_send_and_receive();
            system.process_all_messages();
        }

        js!{
            window.cbclient.setState(oldState => update(oldState, {
                system: {networkingTurns: {"$set":
                    @{system.networking_debug_all_n_turns()}
                }
            }}));

            window.cbclient.onFrame();
        }

        let mut next = self.clone();

        if self.skip_turns > 0 {
            next.skip_turns -= 1;
        } else {
            let maybe_sleep = system.networking_finish_turn();
            next.skip_turns = if let Some(duration) = maybe_sleep {
                duration.subsec_millis()
            } else {
                0
            };
        }

        ::stdweb::web::window().request_animation_frame(move |_| next.frame());
    }
}

mod planning_browser;
pub use planning_browser::*;

mod debug;
pub use debug::*;
