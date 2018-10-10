use kay::TypedID;
use stdweb::serde::Serde;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use stdweb::js_export;
use SYSTEM;

#[cfg_attr(
    all(target_arch = "wasm32", target_os = "unknown"),
    js_export
)]
pub fn get_building_info(building_id: Serde<::land_use::buildings::BuildingID>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    building_id
        .0
        .get_ui_info(::browser_ui::BrowserUIID::local_first(world).into(), world);
}

#[cfg_attr(
    all(target_arch = "wasm32", target_os = "unknown"),
    js_export
)]
pub fn get_household_info(household_id: Serde<::economy::households::HouseholdID>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    household_id
        .0
        .get_ui_info(::browser_ui::BrowserUIID::local_first(world).into(), world);
}
