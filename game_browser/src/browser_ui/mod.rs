use kay::{World, ActorSystem, Actor, TypedID};
use compact::CVec;
use stdweb::serde::Serde;

#[derive(Compact, Clone)]
pub struct BrowserUI {
    id: BrowserUIID,
}

pub fn flatten_vertices(vertices: &[::michelangelo::Vertex]) -> &[f32] {
    let new_len = vertices.len() * 3;
    unsafe { ::std::slice::from_raw_parts(vertices.as_ptr() as *const f32, new_len) }
}

pub fn flatten_instances(instances: &[::michelangelo::Instance]) -> &[f32] {
    let new_len = instances.len() * 8;
    unsafe { ::std::slice::from_raw_parts(instances.as_ptr() as *const f32, new_len) }
}

pub fn to_js_mesh(mesh: &::michelangelo::Mesh) -> ::stdweb::Value {
    let vertices: ::stdweb::web::TypedArray<f32> = flatten_vertices(&mesh.vertices).into();
    let indices: ::stdweb::web::TypedArray<u16> = (&*mesh.indices).into();

    let value = js! {
        return {
            vertices: @{vertices},
            indices: @{indices}
        };
    };
    value
}

pub fn updated_groups_to_js(group_changes: Vec<::michelangelo::GroupChange>) -> ::stdweb::Array {
    ::stdweb::Array::from(
        group_changes
            .iter()
            .map(|change| {
                ::stdweb::Array::from(vec![
                    ::stdweb::Value::from(change.group_id as u32),
                    to_js_mesh(&change.new_group_mesh),
                ])
            }).collect::<Vec<_>>(),
    )
}

pub trait FrameListener {
    fn on_frame(&mut self, world: &mut World);
}

impl BrowserUI {
    pub fn spawn(id: BrowserUIID, world: &mut World) -> BrowserUI {
        {
            ::land_use::buildings::BuildingID::global_broadcast(world)
                .get_render_info(id.into(), world);
        }

        BrowserUI { id }
    }
}

impl FrameListener for BrowserUI {
    fn on_frame(&mut self, world: &mut World) {
        ::simulation::SimulationID::global_first(world).get_info(self.id_as(), world);
    }
}

use simulation::ui::{SimulationUI, SimulationUIID};

impl SimulationUI for BrowserUI {
    fn on_simulation_info(
        &mut self,
        current_instant: ::simulation::Instant,
        speed: u16,
        _world: &mut World,
    ) {
        js! {
            window.cbReactApp.setState(oldState => update(oldState, {
                simulation: {
                    ticks: {"$set": @{current_instant.ticks() as u32}},
                    time: {"$set": @{
                        Serde(::simulation::TimeOfDay::from(current_instant).hours_minutes())
                    }},
                    speed: {"$set": @{speed}}
                }
            }))
        }
    }
}

use land_use::ui::{LandUseUI, LandUseUIID};

impl LandUseUI for BrowserUI {
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

use economy::households::ui::{HouseholdUI, HouseholdUIID};

impl HouseholdUI for BrowserUI {
    fn on_household_ui_info(
        &mut self,
        id: ::economy::households::HouseholdID,
        core: &::economy::households::HouseholdCore,
        _world: &mut World,
    ) {
        js!{
            window.cbReactApp.setState(oldState => update(oldState, {
                households: {
                    householdInfo: {
                        [@{Serde(id)}]: {"$set": {
                            core: @{Serde(core)}
                        }}
                    }
                }
            }));
        }
    }
}

mod kay_auto;
pub use self::kay_auto::*;

pub fn setup(system: &mut ActorSystem) {
    system.register::<BrowserUI>();
    auto_setup(system);
}

pub fn spawn(world: &mut World) {
    BrowserUIID::spawn(world);
}
