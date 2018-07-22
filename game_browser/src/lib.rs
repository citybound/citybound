#![feature(proc_macro)]

#[macro_use]
extern crate stdweb;
use stdweb::js_export;

extern crate kay;
use kay::ActorSystem;

extern crate citybound_common;
use citybound_common::*;

use std::panic;

#[js_export]
pub fn start() {
    panic::set_hook(Box::new(|info| console!(error, info.to_string())));

    js!{ console.log("Before setup") }

    let mut system =
        kay::ActorSystem::new(kay::Networking::new(1, vec!["localhost:9999", "ws-client"]));
    setup_all(&mut system);

    system.networking_connect();

    let browser_ui_id = citybound_common::browser_ui::BrowserUIID::spawn(&mut system.world());

    system.process_all_messages();

    js!{ console.log("After setup") }

    let main_loop = Rc::new(RefCell::new(MainLoop {
        system,
        browser_ui_id,
    }));

    main_loop.borrow_mut().frame(main_loop.clone());
}

use std::cell::RefCell;
use std::rc::Rc;

struct MainLoop {
    system: ActorSystem,
    browser_ui_id: citybound_common::browser_ui::BrowserUIID,
}

impl MainLoop {
    fn frame(&mut self, rc: Rc<RefCell<Self>>) {
        let system = &mut self.system;
        let world = &mut system.world();

        system.networking_send_and_receive();
        system.process_all_messages();

        self.browser_ui_id.on_frame(world);
        system.process_all_messages();

        system.process_all_messages();
        system.process_all_messages();
        system.process_all_messages();
        system.process_all_messages();
        system.process_all_messages();
        system.process_all_messages();
        system.process_all_messages();

        system.networking_finish_turn();

        ::stdweb::web::window().request_animation_frame(move |_dt| {
            let next_rc = rc.clone();
            rc.borrow_mut().frame(next_rc);
        });

        // stdweb::web::set_timeout(
        //     move || {
        //         let next_rc = rc.clone();
        //         rc.borrow_mut().frame(next_rc);
        //     },
        //     100,
        // );
    }
}
