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
    ::simulation::SimulationID::global_first(world).set_speed(new_speed, world);
}

#[derive(Compact, Clone)]
pub struct BrowserSimulationUI {
    id: BrowserSimulationUIID,
}

impl BrowserSimulationUI {
    pub fn spawn(id: BrowserSimulationUIID, _: &mut World) -> BrowserSimulationUI {
        BrowserSimulationUI { id }
    }
}

impl FrameListener for BrowserSimulationUI {
    fn on_frame(&mut self, world: &mut World) {
        ::simulation::SimulationID::global_first(world).get_info(self.id_as(), world);
    }
}

use simulation::ui::{SimulationUI, SimulationUIID};

impl SimulationUI for BrowserSimulationUI {
    fn on_simulation_info(
        &mut self,
        current_instant: ::simulation::Instant,
        speed: u16,
        _world: &mut World,
    ) {
        js! {
            window.cbReactApp.setState(oldState => update(oldState, {
                simulation: {
                    ticks: {"$set": @{current_instant.ticks() as u32}},
                    time: {"$set": @{
                        Serde(::simulation::TimeOfDay::from(current_instant).hours_minutes())
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
    system.register::<BrowserSimulationUI>();
    auto_setup(system);
}

pub fn spawn(world: &mut World) {
    BrowserSimulationUIID::spawn(world);
}
