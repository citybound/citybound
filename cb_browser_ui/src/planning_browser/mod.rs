use stdweb::serde::Serde;
use kay::{World, Actor, External, ActorSystem, TypedID};
use compact::{CHashMap};
use std::collections::HashMap;
use descartes::{LinePath, P2, EditArcLinePath, ResolutionStrategy, Closedness};
use michelangelo::{MeshGrouper};
use cb_planning::{
    Project, GestureID, PrototypeID, PlanHistory, PlanResult, PlanHistoryUpdate, ProjectUpdate,
    PlanResultUpdate, ActionGroups,
};
use cb_planning::plan_manager::ProjectID;
use cb_planning::plan_manager::ui::{PlanningUI, PlanningUIID};
use planning::{CBPlanningLogic, CBPlanManagerID, CBGestureIntent, CBPrototypeKind};
use land_use::zone_planning::{LandUse, LAND_USES, ZoneIntent, ZoneConfig};
use browser_utils::{updated_groups_to_js, to_js_mesh, FrameListener, FrameListenerID};

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use stdweb::js_export;
use SYSTEM;

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), js_export)]
pub fn start_new_gesture(
    project_id: Serde<ProjectID>,
    gesture_id: Serde<GestureID>,
    intent: Serde<CBGestureIntent>,
) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    CBPlanManagerID::global_first(world).start_new_gesture(
        project_id.0,
        gesture_id.0,
        intent.0,
        world,
    )
}

use descartes::{Corner};

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), js_export)]
pub fn with_control_point_added(
    intent: Serde<CBGestureIntent>,
    point: Serde<P2>,
    add_to_end: bool,
) -> Serde<CBGestureIntent> {
    Serde(match intent.0 {
        CBGestureIntent::Road(road_intent) => CBGestureIntent::Road(RoadIntent {
            path: road_intent
                .path
                .with_corner_added(add_to_end, Corner::new(point.0, None, None)),
            ..road_intent
        }),
        CBGestureIntent::Zone(zone_intent) => CBGestureIntent::Zone(ZoneIntent {
            boundary: zone_intent
                .boundary
                .with_corner_added(add_to_end, Corner::new(point.0, None, None)),
            ..zone_intent
        }),
        other => other,
    })
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), js_export)]
pub fn set_intent(
    project_id: Serde<ProjectID>,
    gesture_id: Serde<GestureID>,
    intent: Serde<CBGestureIntent>,
    commit: bool,
) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    CBPlanManagerID::global_first(world).set_intent(
        project_id.0,
        gesture_id.0,
        intent.0,
        commit,
        world,
    )
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), js_export)]
pub fn undo(project_id: Serde<ProjectID>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    CBPlanManagerID::global_first(world).undo(project_id.0, world)
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), js_export)]
pub fn redo(project_id: Serde<ProjectID>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    CBPlanManagerID::global_first(world).redo(project_id.0, world)
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), js_export)]
pub fn implement_project(project_id: Serde<ProjectID>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    CBPlanManagerID::global_first(world).implement(project_id.0, world);
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), js_export)]
pub fn start_new_project(project_id: Serde<ProjectID>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    CBPlanManagerID::global_first(world).start_new_project(project_id.0, world);
}

use transport::transport_planning::{RoadIntent, RoadLaneConfig};

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), js_export)]
pub fn new_road_intent(n_lanes_forward: usize, n_lanes_backward: usize) -> Serde<RoadIntent> {
    Serde(RoadIntent::new(
        vec![],
        RoadLaneConfig {
            n_lanes_forward: n_lanes_forward as u8,
            n_lanes_backward: n_lanes_backward as u8,
        },
    ))
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), js_export)]
pub fn new_zone_intent(new_land_use: Serde<LandUse>) -> Serde<ZoneIntent> {
    Serde(ZoneIntent {
        boundary: EditArcLinePath::new(
            vec![],
            ResolutionStrategy::AssumeLines,
            Closedness::AlwaysClosed,
        ),
        config: ZoneConfig {
            land_use: new_land_use.0,
            max_height: None,
            set_back: None,
        },
    })
}

#[derive(Compact, Clone)]
pub struct BrowserPlanningUI {
    id: BrowserPlanningUIID,
    state: External<BrowserPlanningUINonPersistedState>,
}

impl ::std::ops::Deref for BrowserPlanningUI {
    type Target = BrowserPlanningUINonPersistedState;

    fn deref(&self) -> &BrowserPlanningUINonPersistedState {
        &self.state
    }
}

impl ::std::ops::DerefMut for BrowserPlanningUI {
    fn deref_mut(&mut self) -> &mut BrowserPlanningUINonPersistedState {
        &mut self.state
    }
}

pub struct BrowserPlanningUINonPersistedState {
    // TODO: replace these with only known states and store them in JS only
    master_plan: PlanHistory<CBGestureIntent>,
    projects: HashMap<ProjectID, Project<CBGestureIntent>>,
    result_preview: PlanResult<CBPrototypeKind>,
    actions_preview: ActionGroups,
    awaiting_preview_update: bool,

    // planning geometry
    lanes_to_construct_grouper: MeshGrouper<PrototypeID>,
    lanes_to_construct_marker_grouper: MeshGrouper<PrototypeID>,
    lanes_to_construct_marker_gaps_grouper: MeshGrouper<PrototypeID>,
    zone_groupers: HashMap<LandUse, MeshGrouper<PrototypeID>>,
    zone_outline_groupers: HashMap<LandUse, MeshGrouper<PrototypeID>>,
    building_outlines_grouper: MeshGrouper<PrototypeID>,
}

use descartes::ArcLinePath;
use michelangelo::{Mesh};
use dimensions::CONTROL_POINT_HANDLE_RADIUS;

pub fn static_meshes() -> Vec<(&'static str, Mesh)> {
    let dot_mesh = Mesh::from_path_as_band(
        &ArcLinePath::circle(P2::new(0.0, 0.0), CONTROL_POINT_HANDLE_RADIUS)
            .unwrap()
            .to_line_path(),
        0.3,
        0.2,
    );

    let split_mesh = Mesh::from_path_as_band(
        &LinePath::new(vec![P2::new(0.0, -10.0), P2::new(0.0, 10.0)].into()).unwrap(),
        0.3,
        0.2,
    );

    let change_n_lanes_mesh = Mesh::from_path_as_band(
        &LinePath::new(vec![P2::new(-3.0, 0.0), P2::new(3.0, 0.0)].into()).unwrap(),
        0.3,
        0.2,
    );

    vec![
        ("GestureDot", dot_mesh),
        ("GestureSplit", split_mesh),
        ("GestureChangeNLanes", change_n_lanes_mesh),
    ]
}

impl BrowserPlanningUI {
    pub fn spawn(id: BrowserPlanningUIID, _world: &mut World) -> BrowserPlanningUI {
        {
            for (name, mesh) in static_meshes() {
                js! {
                    window.cbReactApp.boundSetState(oldState => update(oldState, {
                        planning: {rendering: {staticMeshes: {
                            [@{name}]: {"$set": @{to_js_mesh(&mesh)}}
                        }}}
                    }));
                }
            }
        }

        BrowserPlanningUI {
            id,
            state: External::new(BrowserPlanningUINonPersistedState {
                master_plan: PlanHistory::new(),
                projects: HashMap::new(),
                result_preview: PlanResult::new(),
                actions_preview: ActionGroups::new(),
                awaiting_preview_update: false,
                lanes_to_construct_grouper: MeshGrouper::new(2000),
                lanes_to_construct_marker_grouper: MeshGrouper::new(2000),
                lanes_to_construct_marker_gaps_grouper: MeshGrouper::new(2000),
                zone_groupers: LAND_USES
                    .iter()
                    .map(|land_use| (*land_use, MeshGrouper::new(2000)))
                    .collect(),
                zone_outline_groupers: LAND_USES
                    .iter()
                    .map(|land_use| (*land_use, MeshGrouper::new(2000)))
                    .collect(),
                building_outlines_grouper: MeshGrouper::new(2000),
            }),
        }
    }
}

impl FrameListener for BrowserPlanningUI {
    fn on_frame(&mut self, world: &mut World) {
        use stdweb::unstable::TryInto;

        CBPlanManagerID::global_first(world).get_all_plans(
            self.id_as(),
            self.master_plan.as_known_state(),
            self.projects
                .iter()
                .map(|(project_id, project)| (*project_id, project.as_known_state()))
                .collect(),
            world,
        );

        let maybe_current_project_id: Result<Serde<ProjectID>, _> = js! {
            return (window.cbReactApp.state.uiMode == "planning" &&
                window.cbReactApp.state.planning.currentProject);
        }
        .try_into();
        if let Ok(Serde(current_project_id)) = maybe_current_project_id {
            if !self.awaiting_preview_update {
                CBPlanManagerID::global_first(world).get_project_preview_update(
                    self.id_as(),
                    current_project_id,
                    self.result_preview.as_known_state(),
                    world,
                );
                self.awaiting_preview_update = true;
            }
        }
    }
}

impl PlanningUI<CBPlanningLogic> for BrowserPlanningUI {
    fn on_plans_update(
        &mut self,
        master_update: &PlanHistoryUpdate<CBGestureIntent>,
        project_updates: &CHashMap<ProjectID, ProjectUpdate<CBGestureIntent>>,
        _world: &mut World,
    ) {
        if !master_update.is_empty() {
            self.master_plan.apply_update(master_update);
            js! {
                window.cbReactApp.boundSetState(oldState => update(oldState, {
                    planning: {
                        master: {"$set": @{Serde(&self.master_plan)}},
                    }
                }));
                console.log("Got master plan update");
            }
        }
        for (project_id, project_update) in project_updates.pairs() {
            match project_update {
                ProjectUpdate::None => {}
                ProjectUpdate::ChangedOngoing(new_ongoing) => {
                    js! {
                        window.cbReactApp.boundSetState(oldState => update(oldState, {
                            planning: {
                                projects: {
                                    [@{Serde(*project_id)}]: {
                                        ongoing: {"$set": @{Serde(new_ongoing)}}
                                    }
                                }
                            }
                        }));
                        console.log("Changed ongoing project");
                    }
                    self.projects
                        .get_mut(project_id)
                        .expect("Should already have project")
                        .set_ongoing_step(new_ongoing.clone());
                }
                ProjectUpdate::ChangedCompletely(new_project) => {
                    js! {
                        window.cbReactApp.boundSetState(oldState => update(oldState, {
                            planning: {
                                projects: {
                                    [@{Serde(*project_id)}]: {"$set": @{Serde(new_project)}}
                                }
                            }
                        }));
                        console.log("Replaced ongoing project");
                    }
                    self.projects.insert(*project_id, new_project.clone());
                }
                ProjectUpdate::Removed => {
                    js! {
                       window.cbReactApp.boundSetState(oldState => update(oldState, {
                           planning: {
                               projects: {
                                   "$unset": [@{Serde(*project_id)}]
                               }
                           }
                       }));
                       console.log("Removed ongoing project");
                    }
                    self.projects.remove(project_id);
                }
            }
        }
    }

    fn on_project_preview_update(
        &mut self,
        _project_id: ProjectID,
        effective_history: &PlanHistory<CBGestureIntent>,
        result_update: &PlanResultUpdate<CBPrototypeKind>,
        new_actions: &ActionGroups,
        _world: &mut World,
    ) {
        use transport::transport_planning::{
            RoadPrototype, LanePrototype, SwitchLanePrototype, IntersectionPrototype,
        };
        use transport::ui::{lane_mesh, marker_mesh, switch_marker_gap_mesh};
        use land_use::zone_planning::{LotPrototype, LotOccupancy};

        let mut lanes_to_construct_add = Vec::new();
        let mut lanes_to_construct_rem = Vec::new();

        let mut lanes_to_construct_marker_add = Vec::new();
        let mut lanes_to_construct_marker_rem = Vec::new();

        let mut lanes_to_construct_marker_gaps_add = Vec::new();
        let mut lanes_to_construct_marker_gaps_rem = Vec::new();

        let mut zones_add: HashMap<LandUse, _> = LAND_USES
            .iter()
            .map(|land_use| (*land_use, Vec::new()))
            .collect();
        let mut zones_rem: HashMap<LandUse, _> = LAND_USES
            .iter()
            .map(|land_use| (*land_use, Vec::new()))
            .collect();

        let mut zone_outlines_add: HashMap<LandUse, _> = LAND_USES
            .iter()
            .map(|land_use| (*land_use, Vec::new()))
            .collect();
        let mut zone_outlines_rem: HashMap<LandUse, _> = LAND_USES
            .iter()
            .map(|land_use| (*land_use, Vec::new()))
            .collect();

        let mut building_outlines_add = Vec::new();
        let mut building_outlines_rem = Vec::new();

        for prototype_id in &result_update.prototypes_to_drop {
            let prototype = self
                .result_preview
                .prototypes
                .get(*prototype_id)
                .expect("Should have prototype about to be dropped");

            let corresponding_action = self.actions_preview.corresponding_action(*prototype_id);
            match prototype.kind {
                CBPrototypeKind::Road(RoadPrototype::Lane(_)) => match corresponding_action {
                    Some(ref action) if action.is_construct() => {
                        lanes_to_construct_rem.push(*prototype_id);
                        lanes_to_construct_marker_rem.push(*prototype_id);
                    }
                    _ => {}
                },
                CBPrototypeKind::Road(RoadPrototype::SwitchLane(_)) => match corresponding_action {
                    Some(ref action) if action.is_construct() => {
                        lanes_to_construct_marker_gaps_rem.push(*prototype_id);
                    }
                    _ => {}
                },
                CBPrototypeKind::Road(RoadPrototype::Intersection(_)) => match corresponding_action
                {
                    Some(ref action) if action.is_construct() => {
                        lanes_to_construct_rem.push(*prototype_id);
                    }
                    _ => {}
                },
                CBPrototypeKind::Lot(LotPrototype {
                    ref lot, occupancy, ..
                }) => {
                    if let LotOccupancy::Occupied(_) = occupancy {
                        building_outlines_rem.push(*prototype_id)
                    }
                    for config in &lot.zone_configs {
                        zones_rem
                            .get_mut(&config.land_use)
                            .expect("Should have land use to update removes")
                            .push(*prototype_id);
                        zone_outlines_rem
                            .get_mut(&config.land_use)
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
                CBPrototypeKind::Road(RoadPrototype::Lane(LanePrototype(ref lane_path, _))) => {
                    match corresponding_action {
                        Some(ref action) if action.is_construct() => {
                            lanes_to_construct_add.push((new_prototype.id, lane_mesh(lane_path)));
                            let marker = marker_mesh(lane_path);
                            lanes_to_construct_marker_add
                                .push((new_prototype.id, marker.0 + marker.1));
                        }
                        _ => {}
                    }
                }
                CBPrototypeKind::Road(RoadPrototype::SwitchLane(SwitchLanePrototype(
                    ref lane_path,
                ))) => match corresponding_action {
                    Some(ref action) if action.is_construct() => {
                        lanes_to_construct_marker_gaps_add
                            .push((new_prototype.id, switch_marker_gap_mesh(lane_path)));
                    }
                    _ => {}
                },
                CBPrototypeKind::Road(RoadPrototype::Intersection(IntersectionPrototype {
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
                CBPrototypeKind::Lot(LotPrototype {
                    ref lot, occupancy, ..
                }) => {
                    let mesh = Mesh::from_area(&lot.area);
                    let outline_mesh = Mesh::from_path_as_band_asymmetric(
                        lot.area.primitives[0].boundary.path(),
                        1.5,
                        -0.5,
                        0.0,
                    );
                    if let LotOccupancy::Occupied(_) = occupancy {
                        let thin_outline_mesh = Mesh::from_path_as_band_asymmetric(
                            lot.area.primitives[0].boundary.path(),
                            0.75,
                            -0.25,
                            0.0,
                        );
                        building_outlines_add.push((new_prototype.id, thin_outline_mesh))
                    }
                    for config in &lot.zone_configs {
                        zones_add
                            .get_mut(&config.land_use)
                            .expect("Should have land use to update adds")
                            .push((new_prototype.id, mesh.clone()));
                        zone_outlines_add
                            .get_mut(&config.land_use)
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

        let updated_building_outlines_groups = self
            .building_outlines_grouper
            .update(building_outlines_rem, building_outlines_add);

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RoadInfo {
            center_line: LinePath,
            outline: LinePath,
            n_lanes_forward: usize,
            n_lanes_backward: usize,
        }

        let road_infos: HashMap<GestureID, RoadInfo> =
            ::transport::transport_planning::gesture_intent_smooth_paths(effective_history)
                .into_iter()
                .map(|(gesture_id, _, road_intent, path)| {
                    (
                        gesture_id,
                        RoadInfo {
                            outline: ::descartes::Band::new_asymmetric(
                                path.clone(),
                                f32::from(road_intent.n_lanes_backward)
                                    * ::dimensions::LANE_DISTANCE
                                    + 0.4 * ::dimensions::LANE_DISTANCE,
                                f32::from(road_intent.n_lanes_forward)
                                    * ::dimensions::LANE_DISTANCE
                                    + 0.4 * ::dimensions::LANE_DISTANCE,
                            )
                            .outline()
                            .0,
                            center_line: path,
                            n_lanes_forward: road_intent.n_lanes_forward as usize,
                            n_lanes_backward: road_intent.n_lanes_backward as usize,
                        },
                    )
                })
                .collect();

        js! {
            window.cbReactApp.boundSetState(oldState => update(oldState, {
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
                        zoneOutlineGroups: @{updated_zones_all_outline_groups},
                        buildingOutlinesGroup: {
                            "$add": @{updated_groups_to_js(
                                updated_building_outlines_groups
                            )}
                        },
                    },
                    roadInfos: {"$set": @{Serde(road_infos)}},
                }}
            }));
        }

        self.result_preview.apply_update(result_update);
        self.actions_preview = new_actions.clone();
        self.awaiting_preview_update = false;
    }
}

mod kay_auto;
pub use self::kay_auto::*;

pub fn setup(system: &mut ActorSystem) {
    system.register::<BrowserPlanningUI>();
    auto_setup(system);
}

pub fn spawn(world: &mut World) {
    BrowserPlanningUIID::spawn(world);
}
