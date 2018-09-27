use kay::Actor;
use stdweb::serde::Serde;
use stdweb::js_export;
use SYSTEM;

#[js_export]
pub fn get_building_info(building_id: Serde<::land_use::buildings::BuildingID>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    building_id.0.get_ui_info(
        ::citybound_common::browser_ui::BrowserUI::local_first(world),
        world,
    );
}

#[js_export]
pub fn get_household_info(household_id: Serde<::economy::households::HouseholdID>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    household_id.0.get_ui_info(
        ::citybound_common::browser_ui::BrowserUI::local_first(world),
        world,
    );
}
