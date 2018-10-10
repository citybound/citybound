use kay::{Actor, TypedID};
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use stdweb::js_export;
use SYSTEM;

#[cfg_attr(
    all(target_arch = "wasm32", target_os = "unknown"),
    js_export
)]
pub fn set_sim_speed(new_speed: u16) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    ::simulation::SimulationID::global_first(world).set_speed(new_speed, world);
}
