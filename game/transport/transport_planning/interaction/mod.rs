use kay::{World, MachineID, Fate, TypedID, ActorSystem};
use compact::{CVec, COption};
use descartes::{N, Band, LinePath, WithUniqueOrthogonal, Into2d};
use monet::{RendererID, Instance, Mesh};
use stagemaster::user_interface::{UserInterfaceID, Interactable3d, Interactable3dID, Event3d};
use style::colors;
use render_layers::RenderLayers;

use ui_layers::UILayer;

use planning::{Plan, GestureIntent, PlanResult, Prototype, GestureID, ProposalID, PlanManagerID};
use planning::interaction::{GestureInteractable, GestureInteractableID};
use construction::Action;

use super::{RoadIntent, RoadPrototype, LanePrototype, SwitchLanePrototype, IntersectionPrototype,
gesture_intent_smooth_paths};
use style::dimensions::{LANE_DISTANCE, CENTER_LANE_DISTANCE, LANE_MARKER_WIDTH,
LANE_MARKER_DASH_GAP, LANE_MARKER_DASH_LENGTH};

pub fn render_preview(
    result_preview: &PlanResult,
    maybe_action_preview: &Option<CVec<CVec<Action>>>,
    renderer_id: RendererID,
    frame: usize,
    world: &mut World,
) {
    let mut lane_mesh = Mesh::empty();
    let mut switch_lane_mesh = Mesh::empty();
    let mut intersection_mesh = Mesh::empty();

    const EFFECTIVE_LANE_WIDTH: N = LANE_DISTANCE - LANE_MARKER_WIDTH;

    if let Some(ref action_preview) = *maybe_action_preview {
        for (prototype_id, prototype) in result_preview.prototypes.pairs() {
            let corresponding_construction_action_exists =
                action_preview.iter().any(|action_group| {
                    action_group.iter().any(|action| match *action {
                        Action::Construct(constructed_prototype_id, _) => {
                            constructed_prototype_id == *prototype_id
                        }
                        _ => false,
                    })
                });
            if corresponding_construction_action_exists {
                match *prototype {
                    Prototype::Road(RoadPrototype::Lane(LanePrototype(ref lane_path, _))) => {
                        lane_mesh += Mesh::from_band(
                            &Band::new(lane_path.clone(), EFFECTIVE_LANE_WIDTH),
                            0.1,
                        );
                    }
                    Prototype::Road(RoadPrototype::SwitchLane(SwitchLanePrototype(
                        ref lane_path,
                    ))) => {
                        for dash in lane_path.dash(LANE_MARKER_DASH_GAP, LANE_MARKER_DASH_LENGTH) {
                            switch_lane_mesh +=
                                Mesh::from_band(&Band::new(dash, LANE_MARKER_WIDTH), 0.1);
                        }
                    }
                    Prototype::Road(RoadPrototype::Intersection(IntersectionPrototype {
                        ref area,
                        ref connecting_lanes,
                        ..
                    })) => {
                        intersection_mesh += Mesh::from_band(
                            &Band::new(area.primitives[0].boundary.path().clone(), 0.1),
                            0.1,
                        );

                        for &LanePrototype(ref lane_path, ref timings) in
                            connecting_lanes.values().flat_map(|lanes| lanes)
                        {
                            lane_mesh += Mesh::from_band(
                                &Band::new(lane_path.clone(), EFFECTIVE_LANE_WIDTH),
                                0.1,
                            );
                            if timings[(frame / 10) % timings.len()] {
                                intersection_mesh +=
                                    Mesh::from_band(&Band::new(lane_path.clone(), 0.1), 0.1);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    renderer_id.update_individual(
        RenderLayers::PlanningLane as u32,
        lane_mesh,
        Instance::with_color(colors::STROKE_BASE),
        true,
        world,
    );

    renderer_id.update_individual(
        RenderLayers::PlanningSwitchLane as u32,
        switch_lane_mesh,
        Instance::with_color(colors::STROKE_BASE),
        true,
        world,
    );

    renderer_id.update_individual(
        RenderLayers::PlanningIntersection as u32,
        intersection_mesh,
        Instance::with_color(colors::SELECTION_STROKE),
        true,
        world,
    );
}

pub fn render_preview_new(
    result_preview: &PlanResult,
    maybe_action_preview: &Option<CVec<CVec<Action>>>,
) -> (Mesh, Mesh) {
    let mut lane_mesh = Mesh::empty();
    let mut switch_lane_mesh = Mesh::empty();
    let mut intersection_mesh = Mesh::empty();

    const EFFECTIVE_LANE_WIDTH: N = LANE_DISTANCE - LANE_MARKER_WIDTH;

    if let Some(ref action_preview) = *maybe_action_preview {
        for (prototype_id, prototype) in result_preview.prototypes.pairs() {
            let corresponding_construction_action_exists =
                action_preview.iter().any(|action_group| {
                    action_group.iter().any(|action| match *action {
                        Action::Construct(constructed_prototype_id, _) => {
                            constructed_prototype_id == *prototype_id
                        }
                        _ => false,
                    })
                });
            if corresponding_construction_action_exists {
                match *prototype {
                    Prototype::Road(RoadPrototype::Lane(LanePrototype(ref lane_path, _))) => {
                        lane_mesh += Mesh::from_band(
                            &Band::new(lane_path.clone(), EFFECTIVE_LANE_WIDTH),
                            0.1,
                        );
                    }
                    Prototype::Road(RoadPrototype::SwitchLane(SwitchLanePrototype(
                        ref lane_path,
                    ))) => {
                        for dash in lane_path.dash(LANE_MARKER_DASH_GAP, LANE_MARKER_DASH_LENGTH) {
                            switch_lane_mesh +=
                                Mesh::from_band(&Band::new(dash, LANE_MARKER_WIDTH), 0.1);
                        }
                    }
                    Prototype::Road(RoadPrototype::Intersection(IntersectionPrototype {
                        ref area,
                        ref connecting_lanes,
                        ..
                    })) => {
                        intersection_mesh += Mesh::from_band(
                            &Band::new(area.primitives[0].boundary.path().clone(), 0.1),
                            0.1,
                        );

                        for &LanePrototype(ref lane_path, ref timings) in
                            connecting_lanes.values().flat_map(|lanes| lanes)
                        {
                            lane_mesh += Mesh::from_band(
                                &Band::new(lane_path.clone(), EFFECTIVE_LANE_WIDTH),
                                0.1,
                            );
                            // if timings[(frame / 10) % timings.len()] {
                            //     intersection_mesh +=
                            //         Mesh::from_band(&Band::new(lane_path.clone(), 0.1), 0.1);
                            // }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    (lane_mesh, switch_lane_mesh)
}

#[derive(Compact, Clone)]
pub struct LaneCountInteractable {
    id: LaneCountInteractableID,
    plan_manager: PlanManagerID,
    for_machine: MachineID,
    proposal_id: ProposalID,
    gesture_id: GestureID,
    forward: bool,
    path: LinePath,
    initial_intent: RoadIntent,
}

impl LaneCountInteractable {
    #[cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]
    pub fn spawn(
        id: LaneCountInteractableID,
        user_interface: UserInterfaceID,
        plan_manager: PlanManagerID,
        proposal_id: ProposalID,
        gesture_id: GestureID,
        forward: bool,
        path: &LinePath,
        initial_intent: RoadIntent,
        world: &mut World,
    ) -> Self {
        user_interface.add(
            UILayer::Gesture as usize,
            id.into(),
            COption(Some(Band::new(path.clone(), 3.0).as_area())),
            1,
            world,
        );

        LaneCountInteractable {
            id,
            for_machine: user_interface.as_raw().machine,
            plan_manager,
            proposal_id,
            gesture_id,
            forward,
            path: path.clone(),
            initial_intent,
        }
    }
}

impl GestureInteractable for LaneCountInteractable {
    fn remove(&self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(UILayer::Gesture as usize, self.id.into(), world);
        Fate::Die
    }
}

impl Interactable3d for LaneCountInteractable {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        if let Some((from, to, is_drag_finished)) = match event {
            Event3d::DragOngoing { from, to, .. } => Some((from, to, false)),
            Event3d::DragFinished { from, to, .. } => Some((from, to, true)),
            _ => None,
        } {
            if let Some((closest_point_along, closest_point)) =
                self.path.project_with_tolerance(from.into_2d(), 3.0)
            {
                let closest_point_direction = self.path.direction_along(closest_point_along);

                let n_lanes_delta = ((to.into_2d() - closest_point)
                    .dot(&closest_point_direction.orthogonal()))
                    / LANE_DISTANCE;

                let new_intent = if self.forward {
                    RoadIntent {
                        n_lanes_forward: (self.initial_intent.n_lanes_forward as isize
                            + n_lanes_delta as isize)
                            .max(0) as u8,
                        n_lanes_backward: self.initial_intent.n_lanes_backward,
                    }
                } else {
                    RoadIntent {
                        n_lanes_backward: (self.initial_intent.n_lanes_backward as isize
                            + n_lanes_delta as isize)
                            .max(0) as u8,
                        n_lanes_forward: self.initial_intent.n_lanes_forward,
                    }
                };

                self.plan_manager.set_intent(
                    self.proposal_id,
                    self.gesture_id,
                    GestureIntent::Road(new_intent),
                    is_drag_finished,
                    world,
                );
            }
        }
    }
}

pub fn spawn_gesture_interactables(
    plan: &Plan,
    user_interface: UserInterfaceID,
    plan_manager: PlanManagerID,
    proposal_id: ProposalID,
    world: &mut World,
) -> Vec<GestureInteractableID> {
    let gesture_intent_smooth_paths = gesture_intent_smooth_paths(plan);

    gesture_intent_smooth_paths
        .into_iter()
        .flat_map(|(gesture_id, road_intent, path)| {
            path.shift_orthogonally(
                CENTER_LANE_DISTANCE / 2.0
                    + (f32::from(road_intent.n_lanes_forward) - 0.5) * LANE_DISTANCE,
            ).map(|shifted_path_forward| {
                    LaneCountInteractableID::spawn(
                        user_interface,
                        plan_manager,
                        proposal_id,
                        gesture_id,
                        true,
                        shifted_path_forward,
                        road_intent,
                        world,
                    ).into()
                })
                .into_iter()
                .chain(
                    path.shift_orthogonally(
                        -(CENTER_LANE_DISTANCE / 2.0
                            + (f32::from(road_intent.n_lanes_backward) - 0.5) * LANE_DISTANCE),
                    ).map(|shifted_path_backward| {
                        LaneCountInteractableID::spawn(
                            user_interface,
                            plan_manager,
                            proposal_id,
                            gesture_id,
                            false,
                            shifted_path_backward.reverse(),
                            road_intent,
                            world,
                        ).into()
                    }),
                )
        })
        .collect()
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<LaneCountInteractable>();

    auto_setup(system);
}

pub mod kay_auto;
pub use self::kay_auto::*;
