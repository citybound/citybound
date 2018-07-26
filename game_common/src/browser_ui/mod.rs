#![cfg_attr(feature = "server", allow(unused_variables, unused_imports))]

use kay::{World, ActorSystem, Actor, RawID};
use compact::{CVec, CHashMap};

#[derive(Compact, Clone)]
pub struct BrowserUI {
    id: BrowserUIID,
}

fn flatten_vertices(vertices: &[::monet::Vertex]) -> &[f32] {
    let new_len = vertices.len() * 3;
    unsafe { ::std::slice::from_raw_parts(vertices.as_ptr() as *const f32, new_len) }
}

fn flatten_points(points: &[::descartes::P3]) -> &[f32] {
    let new_len = points.len() * 3;
    unsafe { ::std::slice::from_raw_parts(points.as_ptr() as *const f32, new_len) }
}

#[cfg(feature = "browser")]
fn to_js_mesh(mesh: &::monet::Mesh) -> ::stdweb::Value {
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

impl BrowserUI {
    pub fn spawn(id: BrowserUIID, world: &mut World) -> BrowserUI {
        #[cfg(feature = "browser")]
        {
            for (name, mesh) in ::planning::rendering::static_meshes() {
                js! {
                    window.cbclient.setState(oldState => update(oldState, {
                        planning: {rendering: {staticMeshes: {
                            [@{name}]: {"$set": @{to_js_mesh(&mesh)}}
                        }}}
                    }));
                }
            }

            ::transport::lane::Lane::global_broadcast(world).get_render_info(id, world);
            ::transport::lane::SwitchLane::global_broadcast(world).get_render_info(id, world);
        }

        BrowserUI { id }
    }

    pub fn on_frame(&mut self, world: &mut World) {
        #[cfg(feature = "browser")]
        {
            use ::stdweb::unstable::TryInto;
            use ::stdweb::serde::Serde;

            ::planning::PlanManager::global_first(world).get_all_plans(self.id, world);

            let maybe_current_proposal_id: Result<Serde<::planning::ProposalID>, _> = js! {
                return window.cbclient.state.planning.currentProposal;
            }.try_into();
            if let Ok(Serde(current_proposal_id)) = maybe_current_proposal_id {
                ::planning::PlanManager::global_first(world).get_proposal_preview(
                    self.id,
                    current_proposal_id,
                    world,
                )
            }
        }
    }

    pub fn on_plans_update(
        &mut self,
        master: &::planning::Plan,
        proposals: &CHashMap<::planning::ProposalID, ::planning::Proposal>,
        world: &mut World,
    ) {
        #[cfg(feature = "browser")]
        {
            use ::stdweb::serde::Serde;
            js! {
                window.cbclient.setState(oldState => update(oldState, {
                    planning: {
                        master: {"$set": @{Serde(master)}},
                        proposals: {"$set": @{Serde(proposals)}}
                    }
                }));
            }
        }
    }

    pub fn on_proposal_preview(
        &mut self,
        proposal: ::planning::ProposalID,
        result: &::planning::PlanResult,
        actions: &CVec<CVec<::construction::Action>>,
        world: &mut World,
    ) {
        #[cfg(feature = "browser")]
        {
            use ::construction::Action;
            use ::planning::{Prototype};
            use ::transport::transport_planning::{RoadPrototype, LanePrototype, SwitchLanePrototype, IntersectionPrototype};
            use ::transport::rendering::{lane_and_marker_mesh, switch_marker_gap_mesh};
            use ::land_use::zone_planning::{LotPrototype, LotOccupancy, Lot};
            use ::monet::Mesh;

            let mut zones_mesh = Mesh::empty();
            let mut lanes_to_construct_mesh = Mesh::empty();
            let mut lanes_to_construct_marker_mesh = Mesh::empty();
            let mut switch_lanes_to_construct_marker_gap_mesh = Mesh::empty();
            let mut lanes_to_destruct_mesh = Mesh::empty();

            for (prototype_id, prototype) in result.prototypes.pairs() {
                let corresponding_action_exists =
                    actions.iter().filter_map(|action_group| {
                        action_group.iter().filter_map(|action| match *action {
                            Action::Construct(constructed_prototype_id, _) => {
                                if constructed_prototype_id == *prototype_id {
                                    Some((true, false))
                                } else {
                                    None
                                }
                            }
                            Action::Morph(_, new_prototype_id, _) => {
                                if new_prototype_id == *prototype_id {
                                    Some((true, true))
                                } else {
                                    None
                                }
                            }
                            Action::Destruct(destructed_prototype_id) => {
                                if destructed_prototype_id == *prototype_id {
                                    Some((false, false))
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }).next()
                    }).next();

                if let Some((is_construct, is_morph)) = corresponding_action_exists {
                    match *prototype {
                        Prototype::Road(RoadPrototype::Lane(LanePrototype(ref lane_path, _))) => {
                            let meshes = lane_and_marker_mesh(lane_path);
                            if is_construct && !is_morph {
                                lanes_to_construct_mesh += meshes.0;
                                lanes_to_construct_marker_mesh += (meshes.1).0;
                                lanes_to_construct_marker_mesh += (meshes.1).1;
                            } else if !is_construct {
                                lanes_to_destruct_mesh += meshes.0
                            }
                        }
                        Prototype::Road(RoadPrototype::SwitchLane(SwitchLanePrototype(
                            ref lane_path,
                        ))) => {
                            if is_construct && !is_morph {
                                switch_lanes_to_construct_marker_gap_mesh += switch_marker_gap_mesh(lane_path);
                            }
                        }
                        Prototype::Road(RoadPrototype::Intersection(IntersectionPrototype {
                            ref connecting_lanes,
                            ..
                        })) => {
                            for &LanePrototype(ref lane_path, _) in
                                connecting_lanes.values().flat_map(|lanes| lanes)
                            {
                                let meshes = lane_and_marker_mesh(lane_path);
                                if is_construct && !is_morph {
                                    lanes_to_construct_mesh += meshes.0;
                                } else if !is_construct {
                                    lanes_to_destruct_mesh += meshes.0;
                                }
                            }
                        },
                        Prototype::Lot(LotPrototype{ref lot, occupancy: LotOccupancy::Vacant, ..}) => {
                            zones_mesh += Mesh::from_area(&lot.area);
                        }
                        _ => {}
                    }
                }
            }
            
            js! {
                window.cbclient.setState(oldState => update(oldState, {
                    planning: {rendering: {
                        currentPreview: {"$set": {
                            zones: @{to_js_mesh(&zones_mesh)},
                            lanesToConstruct: @{to_js_mesh(&lanes_to_construct_mesh)},
                            lanesToConstructMarker: @{to_js_mesh(&lanes_to_construct_marker_mesh)},
                            lanesToDestruct: @{to_js_mesh(&lanes_to_destruct_mesh)},
                            switchLanesToConstructMarkerGap: @{to_js_mesh(&switch_lanes_to_construct_marker_gap_mesh)}
                        }}
                    }}
                }));
            }
        }
    }

    pub fn on_lane_constructed(&mut self, id: RawID, lane_path: &::descartes::LinePath, is_switch: bool, on_intersection: bool, world: &mut World) {
        #[cfg(feature = "browser")]
        {
            use ::transport::rendering::{lane_and_marker_mesh, switch_marker_gap_mesh};
            if is_switch {
                let gap_mesh = switch_marker_gap_mesh(lane_path);

                js!{
                    window.cbclient.setState(oldState => update(oldState, {
                        transport: {rendering: {
                            laneMarkerGap: {[@{format!("{:?}", id)}]: {"$set": @{to_js_mesh(&gap_mesh)}}}
                        }}
                    }));
                }

            } else {
                let meshes = lane_and_marker_mesh(lane_path);

                if on_intersection {
                    js!{
                        window.cbclient.setState(oldState => update(oldState, {
                            transport: {rendering: {
                                laneAsphalt: {[@{format!("{:?}", id)}]: {"$set": @{to_js_mesh(&meshes.0)}}}
                            }}
                        }));
                    }
                } else {
                    js!{
                        window.cbclient.setState(oldState => update(oldState, {
                            transport: {rendering: {
                                laneAsphalt: {[@{format!("{:?}", id)}]: {"$set": @{to_js_mesh(&meshes.0)}}},
                                laneMarker: {[@{format!("{:?}", id)}]: {"$set": @{to_js_mesh(&((meshes.1).0 + (meshes.1).1))}}}
                            }}
                        }));
                    }
                }
            }
        }
    }

    pub fn on_lane_destructed(&mut self, id: RawID, is_switch: bool, on_intersection: bool, world: &mut World) {
        #[cfg(feature = "browser")]
        {
            if is_switch {
                    js!{
                        window.cbclient.setState(oldState => update(oldState, {
                            transport: {rendering: {
                                laneMarkerGap: {"$unset": [@{format!("{:?}", id)}]}
                            }}
                        }));
                    }
            } else {
                if on_intersection {
                    js!{
                        window.cbclient.setState(oldState => update(oldState, {
                            transport: {rendering: {
                                laneAsphalt: {"$unset": [@{format!("{:?}", id)}]}
                            }}
                        }));
                    }
                } else {
                    js!{
                        window.cbclient.setState(oldState => update(oldState, {
                            transport: {rendering: {
                                laneAsphalt: {"$unset": [@{format!("{:?}", id)}]},
                                laneMarker: {"$unset": [@{format!("{:?}", id)}]}
                            }}
                        }));
                    }
                }
            }
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<BrowserUI>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
