#![cfg_attr(feature = "server", allow(unused_variables, unused_imports))]
#![cfg_attr(feature = "cargo-clippy", allow(unused_variables, unused_imports))]

use kay::{World, ActorSystem, Actor, RawID, External};
use compact::{CVec, CHashMap};
use std::collections::HashMap;
use descartes::LinePath;
use michelangelo::{MeshGrouper, Instance};
use planning::{ProposalID, Proposal, PrototypeID, PlanHistory, PlanResult,
PlanHistoryUpdate, ProposalUpdate, PlanResultUpdate, ActionGroups};
use ::land_use::zone_planning::{LandUse, LAND_USES};

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
    // TODO: replace these with only known states and store them in JS only
    master_plan: PlanHistory,
    proposals: HashMap<ProposalID, Proposal>,
    result_preview: PlanResult,
    actions_preview: ActionGroups,
    awaiting_preview_update: bool,
    // planning geometry
    lanes_to_construct_grouper: MeshGrouper<PrototypeID>,
    lanes_to_construct_marker_grouper: MeshGrouper<PrototypeID>,
    lanes_to_construct_marker_gaps_grouper: MeshGrouper<PrototypeID>,
    zone_groupers: HashMap<LandUse, MeshGrouper<PrototypeID>>,
    zone_outline_groupers: HashMap<LandUse, MeshGrouper<PrototypeID>>,
    // transport geometry
    asphalt_grouper: MeshGrouper<RawID>,
    lane_marker_grouper: MeshGrouper<RawID>,
    lane_marker_gaps_grouper: MeshGrouper<RawID>,
}

#[cfg(feature = "browser")]
fn flatten_vertices(vertices: &[::michelangelo::Vertex]) -> &[f32] {
    let new_len = vertices.len() * 3;
    unsafe { ::std::slice::from_raw_parts(vertices.as_ptr() as *const f32, new_len) }
}

#[cfg(feature = "browser")]
fn flatten_points(points: &[::descartes::P3]) -> &[f32] {
    let new_len = points.len() * 3;
    unsafe { ::std::slice::from_raw_parts(points.as_ptr() as *const f32, new_len) }
}

#[cfg(feature = "browser")]
fn flatten_instances(instances: &[::michelangelo::Instance]) -> &[f32] {
    let new_len = instances.len() * 8;
    unsafe { ::std::slice::from_raw_parts(instances.as_ptr() as *const f32, new_len) }
}

#[cfg(feature = "browser")]
fn to_js_mesh(mesh: &::michelangelo::Mesh) -> ::stdweb::Value {
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

#[cfg(feature = "browser")]
fn updated_groups_to_js(group_changes: Vec<::michelangelo::GroupChange>) -> ::stdweb::Array {
    ::stdweb::Array::from(
        group_changes
            .iter()
            .map(|change| {
                ::stdweb::Array::from(vec![
                    ::stdweb::Value::from(change.group_id as u32),
                    to_js_mesh(&change.new_group_mesh),
                ])
            })
            .collect::<Vec<_>>(),
    )
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
            ::land_use::buildings::Building::global_broadcast(world).get_render_info(id, world);
        }

        BrowserUI {
            id,
            state: External::new(BrowserUINonPersistedState {
                car_instance_buffers: HashMap::new(),
                master_plan: ::planning::PlanHistory::new(),
                proposals: HashMap::new(),
                result_preview: ::planning::PlanResult::new(),
                actions_preview: ::planning::ActionGroups::new(),
                awaiting_preview_update: false,
                lanes_to_construct_grouper: MeshGrouper::new(2000),
                lanes_to_construct_marker_grouper: MeshGrouper::new(2000),
                lanes_to_construct_marker_gaps_grouper: MeshGrouper::new(2000),
                zone_groupers: LAND_USES
                    .into_iter()
                    .map(|land_use| (*land_use, MeshGrouper::new(2000)))
                    .collect(),
                zone_outline_groupers: LAND_USES
                    .into_iter()
                    .map(|land_use| (*land_use, MeshGrouper::new(2000)))
                    .collect(),
                asphalt_grouper: MeshGrouper::new(2000),
                lane_marker_grouper: MeshGrouper::new(2000),
                lane_marker_gaps_grouper: MeshGrouper::new(2000),
            }),
        }
    }

    pub fn on_frame(&mut self, world: &mut World) {
        #[cfg(feature = "browser")]
        {
            use ::stdweb::unstable::TryInto;
            use ::stdweb::serde::Serde;

            ::planning::PlanManager::global_first(world).get_all_plans(
                self.id,
                self.master_plan.as_known_state(),
                self.proposals
                    .iter()
                    .map(|(proposal_id, proposal)| (*proposal_id, proposal.as_known_state()))
                    .collect(),
                world,
            );

            let maybe_current_proposal_id: Result<Serde<ProposalID>, _> = js! {
                return (window.cbclient.state.uiMode.startsWith("main/Planning") &&
                    window.cbclient.state.planning.currentProposal);
            }.try_into();
            if let Ok(Serde(current_proposal_id)) = maybe_current_proposal_id {
                if !self.awaiting_preview_update {
                    ::planning::PlanManager::global_first(world).get_proposal_preview_update(
                        self.id,
                        current_proposal_id,
                        self.result_preview.as_known_state(),
                        world,
                    );
                    self.awaiting_preview_update = true;
                }
            }

            ::transport::lane::Lane::global_broadcast(world).get_car_instances(self.id, world);
            ::transport::lane::SwitchLane::global_broadcast(world)
                .get_car_instances(self.id, world);

            let mut car_instances = Vec::with_capacity(600_000);

            for lane_instances in self.car_instance_buffers.values() {
                car_instances.extend_from_slice(lane_instances);
            }

            let car_instances_js: ::stdweb::web::TypedArray<f32> =
                flatten_instances(&car_instances).into();

            js! {
                window.cbclient.setState(oldState => update(oldState, {
                    transport: {rendering: {
                        carInstances: {"$set": @{car_instances_js}}
                    }}
                }))
            }
        }
    }

    pub fn on_plans_update(
        &mut self,
        master_update: &PlanHistoryUpdate,
        proposal_updates: &CHashMap<ProposalID, ProposalUpdate>,
        _world: &mut World,
    ) {
        #[cfg(feature = "browser")]
        {
            use ::stdweb::serde::Serde;
            if !master_update.is_empty() {
                self.master_plan.apply_update(master_update);
                js! {
                    window.cbclient.setState(oldState => update(oldState, {
                        planning: {
                            master: {"$set": @{Serde(&self.master_plan)}},
                        }
                    }));
                }
            }
            for (proposal_id, proposal_update) in proposal_updates.pairs() {
                match proposal_update {
                    ProposalUpdate::None => {}
                    ProposalUpdate::ChangedOngoing(new_ongoing) => {
                        js! {
                            window.cbclient.setState(oldState => update(oldState, {
                                planning: {
                                    proposals: {
                                        [@{Serde(*proposal_id)}]: {
                                            ongoing: {"$set": @{Serde(new_ongoing)}}
                                        }
                                    }
                                }
                            }));
                        }
                        self.proposals
                            .get_mut(proposal_id)
                            .expect("Should already have proposal")
                            .set_ongoing_step(new_ongoing.clone());
                    }
                    ProposalUpdate::ChangedCompletely(new_proposal) => {
                        js! {
                            window.cbclient.setState(oldState => update(oldState, {
                                planning: {
                                    proposals: {
                                        [@{Serde(*proposal_id)}]: {"$set": @{Serde(new_proposal)}}
                                    }
                                }
                            }));
                        }
                        self.proposals.insert(*proposal_id, new_proposal.clone());
                    }
                    ProposalUpdate::Removed => {
                        js! {
                           window.cbclient.setState(oldState => update(oldState, {
                               planning: {
                                   proposals: {
                                       "$unset": [@{Serde(*proposal_id)}]
                                   }
                               }
                           }));
                        }
                        self.proposals.remove(proposal_id);
                    }
                }
            }
        }
    }

    pub fn on_proposal_preview_update(
        &mut self,
        _proposal_id: ProposalID,
        result_update: &PlanResultUpdate,
        new_actions: &ActionGroups,
        _world: &mut World,
    ) {
        #[cfg(feature = "browser")]
        {
            use ::planning::PrototypeKind;
            use ::transport::transport_planning::{RoadPrototype, LanePrototype,
SwitchLanePrototype, IntersectionPrototype};
            use ::transport::rendering::{lane_mesh, marker_mesh, switch_marker_gap_mesh};
            use ::land_use::zone_planning::LotPrototype;
            use ::michelangelo::Mesh;

            let mut lanes_to_construct_add = Vec::new();
            let mut lanes_to_construct_rem = Vec::new();

            let mut lanes_to_construct_marker_add = Vec::new();
            let mut lanes_to_construct_marker_rem = Vec::new();

            let mut lanes_to_construct_marker_gaps_add = Vec::new();
            let mut lanes_to_construct_marker_gaps_rem = Vec::new();

            let mut zones_add: HashMap<LandUse, _> = LAND_USES
                .into_iter()
                .map(|land_use| (*land_use, Vec::new()))
                .collect();
            let mut zones_rem: HashMap<LandUse, _> = LAND_USES
                .into_iter()
                .map(|land_use| (*land_use, Vec::new()))
                .collect();

            let mut zone_outlines_add: HashMap<LandUse, _> = LAND_USES
                .into_iter()
                .map(|land_use| (*land_use, Vec::new()))
                .collect();
            let mut zone_outlines_rem: HashMap<LandUse, _> = LAND_USES
                .into_iter()
                .map(|land_use| (*land_use, Vec::new()))
                .collect();

            for prototype_id in &result_update.prototypes_to_drop {
                let prototype = self
                    .result_preview
                    .prototypes
                    .get(*prototype_id)
                    .expect("Should have prototype about to be dropped");

                let corresponding_action = self.actions_preview.corresponding_action(*prototype_id);
                match prototype.kind {
                    PrototypeKind::Road(RoadPrototype::Lane(_)) => match corresponding_action {
                        Some(ref action) if action.is_construct() => {
                            lanes_to_construct_rem.push(*prototype_id);
                            lanes_to_construct_marker_rem.push(*prototype_id);
                        }
                        _ => {}
                    },
                    PrototypeKind::Road(RoadPrototype::SwitchLane(_)) => match corresponding_action
                    {
                        Some(ref action) if action.is_construct() => {
                            lanes_to_construct_marker_gaps_rem.push(*prototype_id);
                        }
                        _ => {}
                    },
                    PrototypeKind::Road(RoadPrototype::Intersection(_)) => {
                        match corresponding_action {
                            Some(ref action) if action.is_construct() => {
                                lanes_to_construct_rem.push(*prototype_id);
                            }
                            _ => {}
                        }
                    }
                    PrototypeKind::Lot(LotPrototype { ref lot, .. }) => {
                        for land_use in &lot.land_uses {
                            zones_rem
                                .get_mut(land_use)
                                .expect("Should have land use to update removes")
                                .push(*prototype_id);
                            zone_outlines_rem
                                .get_mut(land_use)
                                .expect("Should have land use to update removes")
                                .push(*prototype_id);
                        }
                    }
                    _ => {}
                }
            }

            for new_prototype in &result_update.new_prototypes {
                let corresponding_action = new_actions.corresponding_action(new_prototype.id);
                match new_prototype.kind {
                    PrototypeKind::Road(RoadPrototype::Lane(LanePrototype(ref lane_path, _))) => {
                        match corresponding_action {
                            Some(ref action) if action.is_construct() => {
                                lanes_to_construct_add
                                    .push((new_prototype.id, lane_mesh(lane_path)));
                                let marker = marker_mesh(lane_path);
                                lanes_to_construct_marker_add
                                    .push((new_prototype.id, marker.0 + marker.1));
                            }
                            _ => {}
                        }
                    }
                    PrototypeKind::Road(RoadPrototype::SwitchLane(SwitchLanePrototype(
                        ref lane_path,
                    ))) => match corresponding_action {
                        Some(ref action) if action.is_construct() => {
                            lanes_to_construct_marker_gaps_add
                                .push((new_prototype.id, switch_marker_gap_mesh(lane_path)));
                        }
                        _ => {}
                    },
                    PrototypeKind::Road(RoadPrototype::Intersection(IntersectionPrototype {
                        ref connecting_lanes,
                        ..
                    })) => match corresponding_action {
                        Some(ref action) if action.is_construct() => {
                            let mut intersection_mesh = Mesh::empty();
                            for &LanePrototype(ref lane_path, _) in
                                connecting_lanes.values().flat_map(|lanes| lanes)
                            {
                                intersection_mesh += lane_mesh(lane_path);
                            }
                            lanes_to_construct_add.push((new_prototype.id, intersection_mesh))
                        }
                        _ => {}
                    },
                    PrototypeKind::Lot(LotPrototype { ref lot, .. }) => {
                        let mesh = Mesh::from_area(&lot.area);
                        let outline_mesh = Mesh::from_path_as_band_asymmetric(
                            lot.area.primitives[0].boundary.path(),
                            1.5,
                            -0.5,
                            0.0,
                        );
                        for land_use in &lot.land_uses {
                            zones_add
                                .get_mut(land_use)
                                .expect("Should have land use to update adds")
                                .push((new_prototype.id, mesh.clone()));
                            zone_outlines_add
                                .get_mut(land_use)
                                .expect("Should have land use to update adds")
                                .push((new_prototype.id, outline_mesh.clone()));
                        }
                    }
                    _ => {}
                }
            }

            let updated_lanes_to_construct_groups = self
                .lanes_to_construct_grouper
                .update(lanes_to_construct_rem, lanes_to_construct_add);

            let updated_lanes_to_construct_marker_groups = self
                .lanes_to_construct_marker_grouper
                .update(lanes_to_construct_marker_rem, lanes_to_construct_marker_add);

            let updated_lanes_to_construct_marker_gaps_groups =
                self.lanes_to_construct_marker_gaps_grouper.update(
                    lanes_to_construct_marker_gaps_rem,
                    lanes_to_construct_marker_gaps_add,
                );

            let updated_zones_all_groups: ::stdweb::Object = self
                .zone_groupers
                .iter_mut()
                .map(|(land_use, grouper)| {
                    let rem = zones_rem
                        .remove(land_use)
                        .expect("Should have land use removes");
                    let add = zones_add
                        .remove(land_use)
                        .expect("Should have land use adds");
                    let updated_groups_js = updated_groups_to_js(grouper.update(rem, add));
                    let add_op: ::stdweb::Object = Some(("$add", updated_groups_js))
                        .into_iter()
                        .collect::<HashMap<_, _>>()
                        .into();
                    (land_use.to_string(), add_op)
                })
                .collect::<HashMap<_, _>>()
                .into();

            let updated_zones_all_outline_groups: ::stdweb::Object = self
                .zone_outline_groupers
                .iter_mut()
                .map(|(land_use, grouper)| {
                    let rem = zone_outlines_rem
                        .remove(land_use)
                        .expect("Should have land use removes");
                    let add = zone_outlines_add
                        .remove(land_use)
                        .expect("Should have land use adds");
                    let updated_groups_js = updated_groups_to_js(grouper.update(rem, add));
                    let add_op: ::stdweb::Object = Some(("$add", updated_groups_js))
                        .into_iter()
                        .collect::<HashMap<_, _>>()
                        .into();
                    (land_use.to_string(), add_op)
                })
                .collect::<HashMap<_, _>>()
                .into();

            js! {
                window.cbclient.setState(oldState => update(oldState, {
                    planning: {rendering: {
                        currentPreview: {
                            lanesToConstructGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_lanes_to_construct_groups
                                )}
                            },
                            lanesToConstructMarkerGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_lanes_to_construct_marker_groups
                                )}
                            },
                            lanesToConstructMarkerGapsGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_lanes_to_construct_marker_gaps_groups
                                )}
                            },
                            zoneGroups: @{updated_zones_all_groups},
                            zoneOutlineGroups: @{updated_zones_all_outline_groups}
                        }
                    }}
                }));
            }

            self.result_preview.apply_update(result_update);
            self.actions_preview = new_actions.clone();
            self.awaiting_preview_update = false;
        }
    }

    pub fn on_lane_constructed(
        &mut self,
        id: RawID,
        lane_path: &LinePath,
        is_switch: bool,
        on_intersection: bool,
        _world: &mut World,
    ) {
        #[cfg(feature = "browser")]
        {
            use ::transport::rendering::{lane_mesh, marker_mesh, switch_marker_gap_mesh};
            if is_switch {
                let updated_lane_marker_gaps_groups = self
                    .lane_marker_gaps_grouper
                    .update(None, Some((id, switch_marker_gap_mesh(lane_path))));

                js!{
                    window.cbclient.setState(oldState => update(oldState, {
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
                        window.cbclient.setState(oldState => update(oldState, {
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
                        window.cbclient.setState(oldState => update(oldState, {
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
    }

    pub fn on_lane_destructed(
        &mut self,
        id: RawID,
        is_switch: bool,
        on_intersection: bool,
        _world: &mut World,
    ) {
        #[cfg(feature = "browser")]
        {
            if is_switch {
                let updated_lane_marker_gaps_groups =
                    self.lane_marker_gaps_grouper.update(Some(id), None);

                js!{
                    window.cbclient.setState(oldState => update(oldState, {
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
                        window.cbclient.setState(oldState => update(oldState, {
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
                    let updated_lane_marker_groups =
                        self.lane_marker_grouper.update(Some(id), None);
                    js!{
                        window.cbclient.setState(oldState => update(oldState, {
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
    }

    pub fn on_car_instances(
        &mut self,
        from_lane: RawID,
        instances: &CVec<Instance>,
        _: &mut World,
    ) {
        self.car_instance_buffers
            .insert(from_lane, instances.to_vec());
    }

    pub fn on_building_constructed(
        &mut self,
        id: ::land_use::buildings::BuildingID,
        lot: &::land_use::zone_planning::Lot,
        style: ::land_use::buildings::BuildingStyle,
        _world: &mut World,
    ) {
        #[cfg(feature = "browser")]
        {
            let meshes = ::land_use::buildings::architecture::build_building(
                lot,
                style,
                &mut ::util::random::seed(id),
            );

            js!{
                window.cbclient.setState(oldState => update(oldState, {
                    landUse: {rendering: {
                        wall: {[@{format!("{:?}", id)}]: {"$set": @{to_js_mesh(&meshes.wall)}}},
                        brickRoof: {
                            [@{format!("{:?}", id)}]: {"$set": @{to_js_mesh(&meshes.brick_roof)}}},
                        flatRoof: {
                            [@{format!("{:?}", id)}]: {"$set": @{to_js_mesh(&meshes.flat_roof)}}},
                        field: {
                            [@{format!("{:?}", id)}]: {"$set": @{to_js_mesh(&meshes.field)}}},
                    }}
                }));
            }
        }
    }

    pub fn on_building_destructed(
        &mut self,
        id: ::land_use::buildings::BuildingID,
        _world: &mut World,
    ) {
        #[cfg(feature = "browser")]
        {
            js!{
                window.cbclient.setState(oldState => update(oldState, {
                    landUse: {rendering: {
                        wall: {"$unset": [@{format!("{:?}", id)}]},
                        brickRoof: {"$unset": [@{format!("{:?}", id)}]},
                        flatRoof: {"$unset": [@{format!("{:?}", id)}]},
                        field: {"$unset": [@{format!("{:?}", id)}]},
                    }}
                }));
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
