use kay::{World, ActorSystem, Actor, RawID, External, TypedID};
use compact::CVec;
use std::collections::HashMap;
use descartes::LinePath;
use michelangelo::{MeshGrouper, Instance};
use stdweb::serde::Serde;

#[derive(Compact, Clone)]
pub struct BrowserUI {
    id: BrowserUIID,
    state: External<BrowserUINonPersistedState>,
}

impl ::std::ops::Deref for BrowserUI {
    type Target = BrowserUINonPersistedState;

    fn deref(&self) -> &BrowserUINonPersistedState {
        &self.state
    }
}

impl ::std::ops::DerefMut for BrowserUI {
    fn deref_mut(&mut self) -> &mut BrowserUINonPersistedState {
        &mut self.state
    }
}

pub struct BrowserUINonPersistedState {
    car_instance_buffers: HashMap<RawID, Vec<::michelangelo::Instance>>,
    
    // transport geometry
    asphalt_grouper: MeshGrouper<RawID>,
    lane_marker_grouper: MeshGrouper<RawID>,
    lane_marker_gaps_grouper: MeshGrouper<RawID>,
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
            ::transport::lane::LaneID::global_broadcast(world).get_render_info(id.into(), world);
            ::transport::lane::SwitchLaneID::global_broadcast(world)
                .get_render_info(id.into(), world);
            ::land_use::buildings::BuildingID::global_broadcast(world)
                .get_render_info(id.into(), world);
        }

        BrowserUI {
            id,
            state: External::new(BrowserUINonPersistedState {
                car_instance_buffers: HashMap::new(),
                asphalt_grouper: MeshGrouper::new(2000),
                lane_marker_grouper: MeshGrouper::new(2000),
                lane_marker_gaps_grouper: MeshGrouper::new(2000),
            }),
        }
    }
}

impl FrameListener for BrowserUI {
    fn on_frame(&mut self, world: &mut World) {
        ::simulation::SimulationID::global_first(world).get_info(self.id_as(), world);

        ::transport::lane::LaneID::global_broadcast(world).get_car_instances(self.id_as(), world);
        ::transport::lane::SwitchLaneID::global_broadcast(world)
            .get_car_instances(self.id_as(), world);

        let mut car_instances = Vec::with_capacity(600_000);

        for lane_instances in self.car_instance_buffers.values() {
            car_instances.extend_from_slice(lane_instances);
        }

        let car_instances_js: ::stdweb::web::TypedArray<f32> =
            flatten_instances(&car_instances).into();

        js! {
            window.cbReactApp.setState(oldState => update(oldState, {
                transport: {rendering: {
                    carInstances: {"$set": @{car_instances_js}}
                }}
            }))
        }
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



use transport::ui::{TransportUI, TransportUIID};

impl TransportUI for BrowserUI {
    fn on_lane_constructed(
        &mut self,
        id: RawID,
        lane_path: &LinePath,
        is_switch: bool,
        on_intersection: bool,
        _world: &mut World,
    ) {
        use ::transport::ui::{lane_mesh, marker_mesh, switch_marker_gap_mesh};
        if is_switch {
            let updated_lane_marker_gaps_groups = self
                .lane_marker_gaps_grouper
                .update(None, Some((id, switch_marker_gap_mesh(lane_path))));

            js!{
                window.cbReactApp.setState(oldState => update(oldState, {
                    transport: {rendering: {
                        laneMarkerGapGroups: {
                            "$add": @{updated_groups_to_js(
                                updated_lane_marker_gaps_groups
                            )}
                        }
                    }}
                }));
            }
        } else {
            let mesh = lane_mesh(lane_path);
            let updated_asphalt_groups = self.asphalt_grouper.update(None, Some((id, mesh)));

            if on_intersection {
                js!{
                    window.cbReactApp.setState(oldState => update(oldState, {
                        transport: {rendering: {
                            laneAsphaltGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_asphalt_groups
                                )}
                            }
                        }}
                    }));
                }
            } else {
                let marker_meshes = marker_mesh(lane_path);
                let updated_lane_marker_groups = self
                    .lane_marker_grouper
                    .update(None, Some((id, marker_meshes.0 + marker_meshes.1)));
                js!{
                    window.cbReactApp.setState(oldState => update(oldState, {
                        transport: {rendering: {
                            laneAsphaltGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_asphalt_groups
                                )}
                            },
                            laneMarkerGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_lane_marker_groups
                                )}
                            }
                        }}
                    }));
                }
            }
        }
    }

    fn on_lane_destructed(
        &mut self,
        id: RawID,
        is_switch: bool,
        on_intersection: bool,
        _world: &mut World,
    ) {
        if is_switch {
            let updated_lane_marker_gaps_groups =
                self.lane_marker_gaps_grouper.update(Some(id), None);

            js!{
                window.cbReactApp.setState(oldState => update(oldState, {
                    transport: {rendering: {
                        laneMarkerGapGroups: {
                            "$add": @{updated_groups_to_js(
                                updated_lane_marker_gaps_groups
                            )}
                        }
                    }}
                }));
            }
        } else {
            let updated_asphalt_groups = self.asphalt_grouper.update(Some(id), None);

            if on_intersection {
                js!{
                    window.cbReactApp.setState(oldState => update(oldState, {
                        transport: {rendering: {
                            laneAsphaltGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_asphalt_groups
                                )}
                            }
                        }}
                    }));
                }
            } else {
                let updated_lane_marker_groups = self.lane_marker_grouper.update(Some(id), None);
                js!{
                    window.cbReactApp.setState(oldState => update(oldState, {
                        transport: {rendering: {
                            laneAsphaltGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_asphalt_groups
                                )}
                            },
                            laneMarkerGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_lane_marker_groups
                                )}
                            }
                        }}
                    }));
                }
            }
        }
    }

    fn on_car_instances(&mut self, from_lane: RawID, instances: &CVec<Instance>, _: &mut World) {
        self.car_instance_buffers
            .insert(from_lane, instances.to_vec());
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