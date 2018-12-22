#![recursion_limit = "256"]

#[macro_use]
extern crate stdweb;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use stdweb::js_export;

#[macro_use]
extern crate serde_derive;

extern crate kay;
use kay::{ActorSystem, TypedID};

#[macro_use]
extern crate compact_macros;

extern crate cb_simulation;
use cb_simulation::*;

use std::panic;

pub mod planning_browser;
pub mod debug;
pub mod time_browser;
pub mod households_browser;
pub mod transport_browser;
pub mod land_use_browser;
pub mod vegetation_browser;
pub mod browser_utils;

// TODO: not thread safe for now
static mut SYSTEM: *mut ActorSystem = 0 as *mut ActorSystem;

#[cfg_attr(
    all(target_arch = "wasm32", target_os = "unknown"),
    js_export
)]
pub fn start() {
    panic::set_hook(Box::new(|info| console!(error, info.to_string())));

    js!{ console.log("Before setup") }

    let server_host = js!{
        return window.location.hostname;
    }
    .into_string()
    .unwrap();

    let mut network_settings = ::std::collections::HashMap::from(
        js!{
            return window.cbNetworkSettings;
        }
        .into_object()
        .unwrap(),
    );

    use stdweb::serde::Serde;
    use stdweb::unstable::TryFrom;

    let mut system = kay::ActorSystem::new(kay::Networking::new(
        1,
        vec![format!("{}:{}", server_host, 9999), "ws-client".to_owned()],
        u32::try_from(network_settings.remove("batchMessageBytes").unwrap()).unwrap() as usize,
        u32::try_from(network_settings.remove("acceptableTurnDistance").unwrap()).unwrap() as usize,
        u32::try_from(network_settings.remove("skipTurnsPerTurnAhead").unwrap()).unwrap() as usize,
    ));

    setup_common(&mut system);
    debug::setup(&mut system);
    browser_utils::auto_setup(&mut system);
    planning_browser::setup(&mut system);
    transport_browser::setup(&mut system);
    time_browser::setup(&mut system);
    land_use_browser::setup(&mut system);
    households_browser::setup(&mut system);
    vegetation_browser::setup(&mut system);

    js!{
        window.cbTypeIdMapping = @{Serde(system.get_actor_type_id_to_name_mapping())}
    }

    system.networking_connect();

    debug::spawn(&mut system.world());
    planning_browser::spawn(&mut system.world());
    transport_browser::spawn(&mut system.world());
    time_browser::spawn(&mut system.world());
    land_use_browser::spawn(&mut system.world());
    households_browser::spawn(&mut system.world());
    vegetation_browser::spawn(&mut system.world());

    system.process_all_messages();

    js!{ console.log("After setup") }

    let mut main_loop = MainLoop { skip_turns: 0 };

    unsafe { SYSTEM = Box::into_raw(Box::new(system)) };

    main_loop.frame();
}

#[derive(Copy, Clone)]
struct MainLoop {
    skip_turns: usize,
}

impl MainLoop {
    fn frame(&mut self) {
        let system = unsafe { &mut *SYSTEM };
        let world = &mut system.world();

        system.networking_send_and_receive();

        if self.skip_turns == 0 {
            system.process_all_messages();

            browser_utils::FrameListenerID::local_broadcast(world).on_frame(world);
            system.process_all_messages();

            system.networking_send_and_receive();
            system.process_all_messages();
        }

        use ::stdweb::serde::Serde;

        js!{
            window.cbReactApp.boundSetState(oldState => update(oldState, {
                system: {
                    networkingTurns: {"$set": @{Serde(system.networking_debug_all_n_turns())}},
                    queueLengths: {"$set": @{Serde(system.get_queue_lengths())}},
                    messageStats: {"$set": @{Serde(system.get_message_statistics())}},
                }
            }));

            window.cbReactApp.onFrame();
        }

        if self.skip_turns == 0 {
            system.reset_message_statistics();
        }

        let mut next = self.clone();

        if self.skip_turns > 0 {
            next.skip_turns -= 1;
        } else {
            let maybe_should_skip = system.networking_finish_turn();
            if let Some(should_skip) = maybe_should_skip {
                next.skip_turns = should_skip
            }
        }

        ::stdweb::web::window().request_animation_frame(move |_| next.frame());
    }
}

use stdweb::serde::Serde;

#[cfg_attr(
    all(target_arch = "wasm32", target_os = "unknown"),
    js_export
)]
pub fn point_in_area(point: Serde<descartes::P2>, area: Serde<descartes::Area>) -> bool {
    use ::descartes::PointContainer;
    area.0.contains(point.0)
}

#[cfg_attr(
    all(target_arch = "wasm32", target_os = "unknown"),
    js_export
)]
pub fn point_close_to_path(
    point: Serde<descartes::P2>,
    path: Serde<descartes::LinePath>,
    max_distance_right: Serde<descartes::N>,
    max_distance_left: Serde<descartes::N>,
) -> Serde<Option<(descartes::P2, descartes::P2, descartes::V2)>> {
    use ::descartes::WithUniqueOrthogonal;
    Serde(
        path.0
            .project(point.0)
            .and_then(|(distance, projected_point)| {
                let direction = path.0.direction_along(distance);
                let orth_distance = (point.0 - projected_point).dot(&direction.orthogonal_right());
                if orth_distance >= -max_distance_left.0 && orth_distance <= max_distance_right.0 {
                    Some((point.0, projected_point, direction))
                } else {
                    None
                }
            }),
    )
}
