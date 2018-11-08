use kay::World;
use super::{Time, TimeID};

pub trait TimeUI {
    fn on_time_info(&mut self, current_instant: ::time::Instant, speed: u16, _world: &mut World);
}

impl Time {
    pub fn get_info(&mut self, requester: TimeUIID, world: &mut World) {
        requester.on_time_info(self.current_instant, self.speed, world);
    }

    pub fn set_speed(&mut self, speed: u16, _world: &mut World) {
        self.speed = speed as u16;
    }
}

pub mod kay_auto;
pub use self::kay_auto::*;
