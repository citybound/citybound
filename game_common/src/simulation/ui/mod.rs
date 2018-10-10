use kay::World;
use super::{Simulation, SimulationID};

pub trait SimulationUI {
    fn on_simulation_info(
        &mut self,
        current_instant: ::simulation::Instant,
        speed: u16,
        _world: &mut World,
    );
}

impl Simulation {
    pub fn get_info(&mut self, requester: SimulationUIID, world: &mut World) {
        requester.on_simulation_info(self.current_instant, self.speed, world);
    }

    pub fn set_speed(&mut self, speed: u16, _world: &mut World) {
        self.speed = speed as u16;
    }
}

pub mod kay_auto;
pub use self::kay_auto::*;
