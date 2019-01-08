use kay::{World, ActorSystem, TypedID};
use compact::CVec;
use stdweb::serde::Serde;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use stdweb::js_export;
use browser_utils::{to_js_mesh, flatten_instances};
use SYSTEM;
use ::std::collections::HashMap;

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), js_export)]
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
        households: &CVec<::economy::households::HouseholdID>,
        style: ::land_use::buildings::BuildingStyle,
        world: &mut World,
    ) {
        let building_mesh =
            ::land_use::buildings::architecture::build_building(lot, style, households, world);

        let material_updates: ::stdweb::Object = building_mesh
            .meshes
            .into_iter()
            .map(|(material, mesh)| {
                let update_op: ::stdweb::Object = Some(("$set", to_js_mesh(&mesh)))
                    .into_iter()
                    .collect::<HashMap<_, _>>()
                    .into();
                let material_update: ::stdweb::Object = Some((id.as_raw_string(), update_op))
                    .into_iter()
                    .collect::<HashMap<_, _>>()
                    .into();
                (material.to_string(), material_update)
            })
            .collect::<HashMap<_, _>>()
            .into();

        let prop_updates: ::stdweb::Object = building_mesh.props.into_iter().map(|(prop_type, instances)| {
            let update_op: ::stdweb::Object = Some(("$set", flatten_instances(&instances)))
                    .into_iter()
                    .collect::<HashMap<_, _>>()
                    .into();
                let material_update: ::stdweb::Object = Some((id.as_raw_string(), update_op))
                    .into_iter()
                    .collect::<HashMap<_, _>>()
                    .into();
                (prop_type.to_string(), material_update)
            })
            .collect::<HashMap<_, _>>()
            .into();;

        js! {
            window.cbReactApp.boundSetState(oldState => update(oldState, {
                landUse: {rendering: {
                    buildingMeshes: @{material_updates},
                    buildingProps: @{prop_updates}
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
        let unset_op: ::stdweb::Object = Some(("$unset", vec![id.as_raw_string()]))
            .into_iter()
            .collect::<HashMap<_, _>>()
            .into();
        let material_unsets: ::stdweb::Object = ::land_use::buildings::architecture::ALL_MATERIALS
            .iter()
            .map(|material| (material.to_string(), unset_op.clone()))
            .collect::<HashMap<_, _>>()
            .into();
        let prop_unsets: ::stdweb::Object = ::land_use::buildings::architecture::ALL_PROP_TYPES
            .iter()
            .map(|prop_type| (prop_type.to_string(), unset_op.clone()))
            .collect::<HashMap<_, _>>()
            .into();
        js! {
            window.cbReactApp.boundSetState(oldState => update(oldState, {
                landUse: {rendering: {
                    buildingMeshes: @{material_unsets},
                    buildingProps: @{prop_unsets}
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
        js! {
            window.cbReactApp.boundSetState(oldState => update(oldState, {
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
