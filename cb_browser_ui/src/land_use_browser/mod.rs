use kay::{World, ActorSystem, TypedID};
use compact::CVec;
use stdweb::serde::Serde;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use stdweb::js_export;
use browser_utils::to_js_mesh;
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
        .get_ui_info(BrowserLandUseUIID::local_first(world).into(), world);
}

#[derive(Compact, Clone)]
pub struct BrowserLandUseUI {
    id: BrowserLandUseUIID,
}

impl BrowserLandUseUI {
    pub fn spawn(id: BrowserLandUseUIID, world: &mut World) -> BrowserLandUseUI {
        {
            ::land_use::buildings::BuildingID::global_broadcast(world)
                .get_render_info(id.into(), world);
        }

        BrowserLandUseUI { id }
    }
}

use land_use::ui::{LandUseUI, LandUseUIID};

impl LandUseUI for BrowserLandUseUI {
    fn on_building_constructed(
        &mut self,
        id: ::land_use::buildings::BuildingID,
        lot: &::land_use::zone_planning::Lot,
        style: ::land_use::buildings::BuildingStyle,
        _world: &mut World,
    ) {
        let meshes = ::land_use::buildings::architecture::build_building(
            lot,
            style,
            &mut ::util::random::seed(id),
        );

        js!{
            window.cbReactApp.setState(oldState => update(oldState, {
                landUse: {rendering: {
                    wall: {[@{Serde(id)}]: {"$set": @{to_js_mesh(&meshes.wall)}}},
                    brickRoof: {
                        [@{Serde(id)}]: {"$set": @{to_js_mesh(&meshes.brick_roof)}}},
                    flatRoof: {
                        [@{Serde(id)}]: {"$set": @{to_js_mesh(&meshes.flat_roof)}}},
                    field: {
                        [@{Serde(id)}]: {"$set": @{to_js_mesh(&meshes.field)}}},
                }},
                households: {
                    buildingPositions: {[@{Serde(id)}]: {
                        "$set": @{Serde(lot.center_point())}
                    }},
                    buildingShapes: {[@{Serde(id)}]: {
                        "$set": @{Serde(lot.area.clone())}
                    }}
                }
            }));
        }
    }

    fn on_building_destructed(
        &mut self,
        id: ::land_use::buildings::BuildingID,
        _world: &mut World,
    ) {
        js!{
            window.cbReactApp.setState(oldState => update(oldState, {
                landUse: {rendering: {
                    wall: {"$unset": [@{Serde(id)}]},
                    brickRoof: {"$unset": [@{Serde(id)}]},
                    flatRoof: {"$unset": [@{Serde(id)}]},
                    field: {"$unset": [@{Serde(id)}]},
                }},
                households: {buildingPositions: {"$unset": [@{Serde(id)}]}}
            }));
        }
    }

    fn on_building_ui_info(
        &mut self,
        _id: ::land_use::buildings::BuildingID,
        style: ::land_use::buildings::BuildingStyle,
        households: &CVec<::economy::households::HouseholdID>,
        _world: &mut World,
    ) {
        js!{
            window.cbReactApp.setState(oldState => update(oldState, {
                households: {
                    inspectedBuildingState: {"$set": {
                        households: @{Serde(households)},
                        style: @{Serde(style)},
                    }}
                }
            }));
        }
    }
}

mod kay_auto;
pub use self::kay_auto::*;

pub fn setup(system: &mut ActorSystem) {
    system.register::<BrowserLandUseUI>();
    auto_setup(system);
}

pub fn spawn(world: &mut World) {
    BrowserLandUseUIID::spawn(world);
}
