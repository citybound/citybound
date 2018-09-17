use kay::Actor;
use stdweb::serde::Serde;
use stdweb::js_export;
use SYSTEM;

#[js_export]
pub fn set_sim_speed(new_speed: u16) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    ::simulation::Simulation::global_first(world).set_speed(new_speed, world);
}
