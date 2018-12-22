use kay::{World, ActorSystem, Actor, TypedID};
use stdweb::serde::Serde;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use stdweb::js_export;
use SYSTEM;
use browser_utils::{FrameListener, FrameListenerID};

#[cfg_attr(
    all(target_arch = "wasm32", target_os = "unknown"),
    js_export
)]
pub fn set_sim_speed(new_speed: u16) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    ::time::TimeID::global_first(world).set_speed(new_speed, world);
}

#[derive(Compact, Clone)]
pub struct BrowserTimeUI {
    id: BrowserTimeUIID,
}

impl BrowserTimeUI {
    pub fn spawn(id: BrowserTimeUIID, _: &mut World) -> BrowserTimeUI {
        BrowserTimeUI { id }
    }
}

impl FrameListener for BrowserTimeUI {
    fn on_frame(&mut self, world: &mut World) {
        ::time::TimeID::global_first(world).get_info(self.id_as(), world);
    }
}

use time::ui::{TimeUI, TimeUIID};

impl TimeUI for BrowserTimeUI {
    fn on_time_info(&mut self, current_instant: ::time::Instant, speed: u16, _world: &mut World) {
        js! {
            window.cbReactApp.boundSetState(oldState => update(oldState, {
                time: {
                    ticks: {"$set": @{current_instant.ticks() as u32}},
                    time: {"$set": @{
                        Serde(::time::TimeOfDay::from(current_instant).hours_minutes())
                    }},
                    speed: {"$set": @{speed}}
                }
            }))
        }
    }
}

mod kay_auto;
pub use self::kay_auto::*;

pub fn setup(system: &mut ActorSystem) {
    system.register::<BrowserTimeUI>();
    auto_setup(system);
}

pub fn spawn(world: &mut World) {
    BrowserTimeUIID::spawn(world);
}
