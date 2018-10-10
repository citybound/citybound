use kay::World;
use super::{HouseholdID, HouseholdCore};

pub trait HouseholdUI {
    fn on_household_ui_info(&mut self, id: HouseholdID, core: &HouseholdCore, _world: &mut World);
}

mod kay_auto;
pub use self::kay_auto::*;
