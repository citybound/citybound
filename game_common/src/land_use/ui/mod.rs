use kay::World;
use compact::CVec;
use super::buildings::{BuildingID, BuildingStyle};
use economy::households::HouseholdID;
use super::zone_planning::Lot;

pub trait LandUseUI {
    fn on_building_constructed(
        &mut self,
        id: BuildingID,
        lot: &Lot,
        style: BuildingStyle,
        _world: &mut World,
    );

    fn on_building_destructed(&mut self, id: BuildingID, _world: &mut World);

    fn on_building_ui_info(
        &mut self,
        id: BuildingID,
        style: BuildingStyle,
        households: &CVec<HouseholdID>,
        _world: &mut World,
    );
}

mod kay_auto;
pub use self::kay_auto::*;
