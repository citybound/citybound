use kay::{World, MachineID, Fate, TypedID, ActorSystem};
use compact::{CVec, CHashMap, CDict, COption};
use descartes::{N, P2, V2, Circle, RoughlyComparable, Into2d, Band, Segment, Path};
use monet::{RendererID, Renderable, RenderableID, Instance};
use stagemaster::{UserInterfaceID, Interactable3d, Interactable2d, Interactable3dID};
use stagemaster::geometry::{AnyShape, band_to_geometry, CPath};
use uuid::Uuid;
use ui_layers::GESTURE_LAYER;
use style::colors;

#[derive(Compact, Clone)]
pub struct Gesture {
    points: CVec<P2>,
    intent: GestureIntent,
    deleted: bool,
}

impl Gesture {
    pub fn new(points: CVec<P2>, intent: GestureIntent) -> Self {
        Gesture { points, intent, deleted: false }
    }
}

#[derive(Compact, Clone)]
pub enum GestureIntent {
    Road(RoadIntent),
    Zone(ZoneIntent),
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct GestureID(Uuid);

impl GestureID {
    pub fn new() -> GestureID {
        GestureID(Uuid::new_v4())
    }
}

#[derive(Compact, Clone, Default)]
pub struct Plan {
    gestures: CHashMap<GestureID, Gesture>,
}

impl Plan {
    pub fn merge<'a, I: IntoIterator<Item = &'a Plan>>(&self, others: I) -> Plan {
        let mut new_plan = self.clone();
        for other in others {
            for (key, value) in other.gestures.pairs() {
                new_plan.gestures.insert(*key, value.clone());
            }
        }
        new_plan
    }
}

// TODO: when applied, proposals can be flattened into the last
// version of each gesture and all intermediate gestures can be completely removed
#[derive(Compact, Clone, Default)]
pub struct Proposal {
    undoable_history: CVec<Plan>,
    redoable_history: CVec<Plan>,
}

impl Proposal {
    pub fn new() -> Proposal {
        Proposal::default()
    }

    pub fn start_new_step(&mut self) {
        self.undoable_history.push(Plan::default());
    }

    pub fn set_ongoing_step(&mut self, current_change: Plan) {
        if self.undoable_history.is_empty() {
            self.undoable_history.push(current_change);
        } else {
            *self.undoable_history.last_mut().unwrap() = current_change;
        }
        self.redoable_history.clear();
    }

    pub fn undo(&mut self) {
        if let Some(most_recent_step) = self.undoable_history.pop() {
            self.redoable_history.push(most_recent_step);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next_step_to_redo) = self.redoable_history.pop() {
            self.undoable_history.push(next_step_to_redo);
        }
    }

    pub fn current_history(&self) -> &[Plan] {
        &self.undoable_history
    }

    fn apply_to(&self, base: &Plan) -> Plan {
        base.merge(&self.undoable_history)
    }
}

#[derive(Compact, Clone)]
pub struct PlanResult {
    prototypes: CVec<Prototype>,
}

#[derive(Compact, Clone)]
pub enum Prototype {
    Road(RoadPrototype),
    Zone,
}

#[derive(Compact, Clone)]
pub enum RoadPrototype {
    Lane(LanePrototype),
    Intersection(IntersectionPrototype),
}

#[derive(Compact, Clone)]
pub struct LanePrototype(CPath);

#[derive(Compact, Clone)]
pub struct IntersectionPrototype {
    connecting_lanes: CVec<LanePrototype>,
    timings: CVec<CVec<bool>>,
}

impl Plan {
    pub fn calculate_result(&self) -> PlanResult {
        let lane_prototypes = self.gestures
            .values()
            .filter_map(|gesture| if let GestureIntent::Road(ref road_intent) =
                gesture.intent
            {
                if gesture.points.len() >= 2 {
                    let center_points = gesture
                        .points
                        .windows(2)
                        .map(|point_pair| {
                            P2::from_coordinates(
                                (point_pair[0].coords + point_pair[1].coords) / 2.0,
                            )
                        })
                        .collect::<Vec<_>>();

                    // for each straight line segment, we have first: a point called END,
                    // marking the end of the circular arc that smoothes the first corner of
                    // this line segment and then second: a point called START,
                    // marking the beginning of the circular arc that smoothes the second corner
                    // of this line segments. Also, we remember the direction of the line segment

                    let mut end_start_directions = Vec::new();

                    for (i, point_pair) in gesture.points.windows(2).enumerate() {
                        let first_corner = point_pair[0];
                        let second_corner = point_pair[1];
                        let previous_center_point =
                            center_points.get(i - 1).unwrap_or(&first_corner);
                        let this_center_point = center_points[i];
                        let next_center_point = center_points.get(i + 1).unwrap_or(&second_corner);
                        let line_direction = (second_corner - first_corner).normalize();

                        let shorter_distance_to_first_corner =
                            (first_corner - previous_center_point).norm().min(
                                (first_corner - this_center_point).norm(),
                            );
                        let shorter_distance_to_second_corner =
                            (second_corner - this_center_point).norm().min(
                                (second_corner - next_center_point).norm(),
                            );

                        let end = first_corner + line_direction * shorter_distance_to_first_corner;
                        let start = second_corner -
                            line_direction * shorter_distance_to_second_corner;

                        end_start_directions.push((end, start, line_direction));
                    }

                    let mut segments = Vec::new();
                    let mut previous_point = gesture.points[0];
                    let mut previous_direction = (gesture.points[1] - gesture.points[0])
                        .normalize();

                    for (end, start, direction) in end_start_directions {
                        if let Some(valid_incoming_arc) =
                            Segment::arc_with_direction(previous_point, previous_direction, end)
                        {
                            segments.push(valid_incoming_arc);
                        }

                        if let Some(valid_connecting_line) = Segment::line(end, start) {
                            segments.push(valid_connecting_line);
                        }

                        previous_point = start;
                        previous_direction = direction;
                    }

                    Some(Prototype::Road(
                        RoadPrototype::Lane(LanePrototype(CPath::new(segments))),
                    ))
                } else {
                    None
                }
            } else {
                None
            })
            .collect();

        PlanResult { prototypes: lane_prototypes }
    }
}

#[derive(Compact, Clone)]
pub struct PlanManager {
    id: PlanManagerID,
    master_plan: Plan,
    proposals: CVec<Proposal>,
    implemented_proposals: CVec<Proposal>,
    ui_state: CHashMap<MachineID, PlanManagerUIState>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ControlPointRef(pub GestureID, pub usize);

#[derive(Compact, Clone)]
pub struct PlanManagerUIState {
    current_proposal: usize,
    canvas: GestureCanvasID,
    gesture_ongoing: bool,
    control_point_interactables: CVec<ControlPointInteractableID>,
    selected_points: CVec<ControlPointRef>,
    current_preview: COption<Plan>,
    current_result_preview: COption<PlanResult>,
    user_interface: UserInterfaceID,
}

impl PlanManager {
    pub fn spawn(id: PlanManagerID, _: &mut World) -> PlanManager {
        PlanManager {
            id,
            master_plan: Plan::default(),
            proposals: vec![Proposal::default()].into(),
            implemented_proposals: CVec::new(),
            ui_state: CHashMap::new(),
        }
    }

    pub fn switch_to(
        &mut self,
        user_interface: UserInterfaceID,
        proposal_id: usize,
        world: &mut World,
    ) {
        let machine = user_interface.as_raw().machine;

        if let Some(current_canvas) =
            self.ui_state.get_mut(machine).map(
                |ui_state| ui_state.canvas,
            )
        {
            current_canvas.remove(user_interface, world);
        };

        self.ui_state.insert(
            machine,
            PlanManagerUIState {
                current_proposal: proposal_id,
                canvas: GestureCanvasID::spawn(user_interface, self.id, proposal_id, world),
                gesture_ongoing: false,
                control_point_interactables: CVec::new(),
                selected_points: CVec::new(),
                current_preview: COption(None),
                current_result_preview: COption(None),
                user_interface,
            },
        );

        self.recreate_control_point_interactables_on_machine(machine, world);
    }

    fn clear_previews(&mut self, proposal_id: usize) {
        for state in self.ui_state.values_mut().filter(|state| {
            state.current_proposal == proposal_id
        })
        {
            state.current_preview = COption(None);
            state.current_result_preview = COption(None);
        }
    }

    #[allow(mutable_transmutes)]
    fn ensure_preview(&self, machine_id: MachineID, proposal_id: usize) -> (&Plan, &PlanResult) {
        let ui_state = self.ui_state.get(machine_id).expect(
            "Should already have a ui state for this machine",
        );

        // super ugly of course, maybe we can use a cell or similar in the future
        unsafe {
            let ui_state_mut: &mut PlanManagerUIState = ::std::mem::transmute(ui_state);
            if ui_state.current_proposal != proposal_id {
                ui_state_mut.current_preview = COption(None);
            }

            if ui_state.current_preview.is_none() {
                let preview_plan = self.proposals[proposal_id].apply_to(&self.master_plan);
                let preview_plan_result = preview_plan.calculate_result();

                ui_state_mut.current_preview = COption(Some(preview_plan));
                ui_state_mut.current_result_preview = COption(Some(preview_plan_result));
            }
        }

        (
            ui_state.current_preview.as_ref().unwrap(),
            ui_state.current_result_preview.as_ref().unwrap(),
        )
    }

    fn recreate_control_point_interactables_on_machine(
        &mut self,
        machine_id: MachineID,
        world: &mut World,
    ) {
        let new_control_point_interactables = {
            let (user_interface, proposal_id, gesture_ongoing) = {
                let state = self.ui_state.get(machine_id).unwrap();
                (
                    state.user_interface,
                    state.current_proposal,
                    state.gesture_ongoing,
                )
            };

            if gesture_ongoing {
                CVec::new()
            } else {
                let (preview, _) = self.ensure_preview(machine_id, proposal_id);

                preview
                    .gestures
                    .pairs()
                    .flat_map(|(gesture_id, gesture)| {
                        gesture
                            .points
                            .iter()
                            .enumerate()
                            .map(|(point_index, point)| {
                                ControlPointInteractableID::spawn(
                                    user_interface,
                                    self.id,
                                    proposal_id,
                                    *gesture_id,
                                    point_index,
                                    *point,
                                    world,
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect()
            }
        };

        let state = self.ui_state.get_mut(machine_id).unwrap();

        for control_point_interactable in state.control_point_interactables.drain() {
            control_point_interactable.remove(state.user_interface, world);
        }

        state.control_point_interactables = new_control_point_interactables;
    }

    fn recreate_control_point_interactables(&mut self, proposal_id: usize, world: &mut World) {
        let machines_with_this_proposal = self.ui_state
            .pairs()
            .filter(|&(_, state)| state.current_proposal == proposal_id)
            .map(|(machine_id, _)| *machine_id)
            .collect::<Vec<_>>();

        for machine_id in machines_with_this_proposal {
            self.recreate_control_point_interactables_on_machine(machine_id, world);
        }
    }

    pub fn get_current_version_of(&self, gesture_id: GestureID, proposal_id: usize) -> &Gesture {
        self.proposals[proposal_id]
            .current_history()
            .iter()
            .rfold(None, |found, step| {
                found.or_else(|| step.gestures.get(gesture_id))
            })
            .into_iter()
            .chain(self.master_plan.gestures.get(gesture_id))
            .next()
            .expect("Expected gesture (that point should be added to) to exist!")
    }

    pub fn select_point(
        &mut self,
        point_ref: ControlPointRef,
        machine_id: MachineID,
        world: &mut World,
    ) {
        {
            let ui_state = self.ui_state.get_mut(machine_id).expect(
                "should already have ui state",
            );
            if !ui_state.selected_points.contains(&point_ref) {
                ui_state.selected_points.push(point_ref);
            }
        }
        self.recreate_control_point_interactables_on_machine(machine_id, world);
    }

    pub fn clear_selection(&mut self, machine_id: MachineID, world: &mut World) {
        {
            let ui_state = self.ui_state.get_mut(machine_id).expect(
                "should already have ui state",
            );
            ui_state.selected_points.clear();
        }
        self.recreate_control_point_interactables_on_machine(machine_id, world);
    }

    pub fn start_new_gesture(
        &mut self,
        proposal_id: usize,
        machine_id: MachineID,
        new_gesture_id: GestureID,
        start: P2,
        world: &mut World,
    ) {
        let new_gesture = Gesture::new(
            vec![start].into(),
            GestureIntent::Road(RoadIntent::new(2, 2)),
        );

        let new_step = Plan { gestures: Some((new_gesture_id, new_gesture)).into_iter().collect() };

        self.proposals[proposal_id].set_ongoing_step(new_step);
        self.proposals[proposal_id].start_new_step();

        self.ui_state
            .get_mut(machine_id)
            .expect("should already have ui state")
            .gesture_ongoing = true;

        self.clear_previews(proposal_id);
        self.recreate_control_point_interactables(proposal_id, world);
    }

    pub fn finish_gesture(&mut self, machine_id: MachineID, world: &mut World) {
        self.ui_state
            .get_mut(machine_id)
            .expect("should already have ui state")
            .gesture_ongoing = false;
        self.recreate_control_point_interactables_on_machine(machine_id, world);
    }

    pub fn add_control_point(
        &mut self,
        proposal_id: usize,
        gesture_id: GestureID,
        new_point: P2,
        add_to_end: bool,
        commit: bool,
        world: &mut World,
    ) {
        let new_step = {
            let current_gesture = self.get_current_version_of(gesture_id, proposal_id);

            let changed_gesture = if add_to_end {
                let end_index = if commit || current_gesture.points.len() == 1 {
                    current_gesture.points.len()
                } else {
                    current_gesture.points.len() - 1
                };
                Gesture {
                    points: current_gesture.points[..end_index]
                        .iter()
                        .cloned()
                        .chain(Some(new_point))
                        .collect(),
                    ..current_gesture.clone()
                }
            } else {
                let start_index = if commit || current_gesture.points.len() == 1 {
                    0
                } else {
                    1
                };
                Gesture {
                    points: Some(new_point)
                        .into_iter()
                        .chain(current_gesture.points[start_index..].iter().cloned())
                        .collect(),
                    ..current_gesture.clone()
                }
            };

            Plan { gestures: Some((gesture_id, changed_gesture)).into_iter().collect() }
        };

        self.proposals[proposal_id].set_ongoing_step(new_step);

        if commit {
            self.proposals[proposal_id].start_new_step();
        }

        self.clear_previews(proposal_id);
        self.recreate_control_point_interactables(proposal_id, world);
    }

    pub fn move_control_point(
        &mut self,
        proposal_id: usize,
        gesture_id: GestureID,
        point_index: usize,
        new_position: P2,
        is_move_finished: bool,
        world: &mut World,
    ) {
        let current_change = {
            let current_gesture = self.get_current_version_of(gesture_id, proposal_id);

            let mut new_gesture_points = current_gesture.points.clone();
            new_gesture_points[point_index] = new_position;

            let new_gesture = Gesture {
                points: new_gesture_points,
                ..current_gesture.clone()
            };


            Plan { gestures: Some((gesture_id, new_gesture)).into_iter().collect() }
        };

        self.proposals[proposal_id].set_ongoing_step(current_change);

        // TODO: can we update only part of the preview
        // for better rendering performance while dragging?
        self.clear_previews(proposal_id);

        if is_move_finished {
            self.proposals[proposal_id].start_new_step();
            self.recreate_control_point_interactables(proposal_id, world);
        }
    }

    pub fn undo(&mut self, proposal_id: usize, _: &mut World) {
        self.proposals[proposal_id].undo();
    }

    pub fn redo(&mut self, proposal_id: usize, _: &mut World) {
        self.proposals[proposal_id].redo();
    }
}

impl Renderable for PlanManager {
    fn setup_in_scene(&mut self, renderer_id: RendererID, scene_id: usize, world: &mut World) {
        let dot_geometry = band_to_geometry(
            &Band::new(
                CPath::new(
                    Segment::arc_with_direction(
                        P2::new(-CONTROL_POINT_HANDLE_RADIUS, 0.0),
                        V2::new(0.0, 1.0),
                        P2::new(CONTROL_POINT_HANDLE_RADIUS, 0.0),
                    ).into_iter()
                        .chain(Segment::arc_with_direction(
                            P2::new(CONTROL_POINT_HANDLE_RADIUS, 0.0),
                            V2::new(0.0, -1.0),
                            P2::new(-CONTROL_POINT_HANDLE_RADIUS, 0.0),
                        ))
                        .collect(),
                ),
                0.3,
            ),
            1.0,
        );

        renderer_id.add_batch(scene_id, 20_000, dot_geometry, world);
    }

    fn render_to_scene(
        &mut self,
        renderer_id: RendererID,
        scene_id: usize,
        frame: usize,
        world: &mut World,
    ) {
        // TODO: clean up this mess
        let proposal_id = self.ui_state
            .get(renderer_id.as_raw().machine)
            .expect("should have ui state for this renderer")
            .current_proposal;
        let (preview, result_preview) =
            self.ensure_preview(renderer_id.as_raw().machine, proposal_id);

        for (i, prototype) in result_preview.prototypes.iter().enumerate() {
            if let Prototype::Road(RoadPrototype::Lane(LanePrototype(ref lane_path))) = *prototype {
                let line_geometry =
                    band_to_geometry(&Band::new(lane_path.clone(), LANE_WIDTH), 0.1);

                renderer_id.update_individual(
                    scene_id,
                    18_000 + i as u16,
                    line_geometry,
                    Instance::with_color(colors::STROKE_BASE),
                    true,
                    world,
                );
            }
        }

        for (i, gesture) in preview.gestures.values().enumerate() {
            if gesture.points.len() >= 2 {
                let line_path = CPath::new(
                    gesture
                        .points
                        .windows(2)
                        .filter_map(|window| Segment::line(window[0], window[1]))
                        .collect(),
                );

                let line_geometry = band_to_geometry(&Band::new(line_path, 0.3), 1.0);

                renderer_id.update_individual(
                    scene_id,
                    19_000 + i as u16,
                    line_geometry,
                    Instance::with_color(colors::GESTURE_LINES),
                    true,
                    world,
                );
            }
        }

        let selected_points = &self.ui_state
            .get(renderer_id.as_raw().machine)
            .expect("should have ui state for this renderer")
            .selected_points;

        let control_point_instances = preview
            .gestures
            .pairs()
            .flat_map(|(gesture_id, gesture)| {
                gesture.points.iter().enumerate().map(move |(point_index,
                       point)| {
                    Instance {
                        instance_position: [point.x, point.y, 0.0],
                        instance_direction: [1.0, 0.0],
                        instance_color: if selected_points.contains(&ControlPointRef(
                            *gesture_id,
                            point_index,
                        ))
                        {
                            colors::CONTROL_POINT_SELECTED
                        } else {
                            colors::CONTROL_POINT
                        },
                    }
                })
            })
            .collect();

        renderer_id.add_several_instances(scene_id, 20_000, frame, control_point_instances, world);
    }
}

const LANE_WIDTH: N = 6.0;

#[derive(Compact, Clone)]
pub struct ControlPointInteractable {
    id: ControlPointInteractableID,
    plan_manager: PlanManagerID,
    proposal_id: usize,
    gesture_id: GestureID,
    point_index: usize,
}

const CONTROL_POINT_HANDLE_RADIUS: N = 3.0;

impl ControlPointInteractable {
    pub fn spawn(
        id: ControlPointInteractableID,
        user_interface: UserInterfaceID,
        plan_manager: PlanManagerID,
        proposal_id: usize,
        gesture_id: GestureID,
        point_index: usize,
        position: P2,
        world: &mut World,
    ) -> Self {
        user_interface.add(
            GESTURE_LAYER,
            id.into(),
            AnyShape::Circle(Circle {
                center: position,
                radius: CONTROL_POINT_HANDLE_RADIUS,
            }),
            1,
            world,
        );

        ControlPointInteractable {
            id,
            plan_manager,
            proposal_id,
            gesture_id,
            point_index,
        }
    }

    pub fn remove(&self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(GESTURE_LAYER, self.id.into(), world);
        Fate::Die
    }
}

use stagemaster::Event3d;

impl Interactable3d for ControlPointInteractable {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        let drag_info = match event {
            Event3d::DragOngoing { from, to, .. } => Some((from, to, false)),
            Event3d::DragFinished { from, to, .. } => Some((from, to, true)),
            _ => None,
        };

        if let Some((from, to, is_finished)) = drag_info {
            self.plan_manager.move_control_point(
                self.proposal_id,
                self.gesture_id,
                self.point_index,
                to.into_2d(),
                is_finished,
                world,
            );

            if is_finished {
                self.plan_manager.select_point(
                    ControlPointRef(self.gesture_id, self.point_index),
                    self.id.as_raw().machine,
                    world,
                );
            }
        }
    }
}

#[derive(Compact, Clone)]
pub struct GestureCanvas {
    id: GestureCanvasID,
    plan_manager: PlanManagerID,
    for_machine: MachineID,
    proposal_id: usize,
    last_point: COption<P2>,
    current_mode: GestureCanvasMode,
}

#[derive(Compact, Clone)]
enum GestureCanvasMode {
    StartNewGesture,
    AddToEndOfExisting(GestureID),
    AddToBeginningOfExisting(GestureID),
}

impl GestureCanvas {
    pub fn spawn(
        id: GestureCanvasID,
        user_interface: UserInterfaceID,
        plan_manager: PlanManagerID,
        proposal_id: usize,
        world: &mut World,
    ) -> Self {
        user_interface.add(GESTURE_LAYER, id.into(), AnyShape::Everywhere, 0, world);

        GestureCanvas {
            id,
            for_machine: user_interface.as_raw().machine,
            plan_manager,
            proposal_id,
            last_point: COption(None),
            current_mode: GestureCanvasMode::StartNewGesture,
        }
    }

    pub fn remove(&self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(GESTURE_LAYER, self.id.into(), world);
        Fate::Die
    }

    pub fn finish_gesture(&mut self, world: &mut World) {
        self.current_mode = GestureCanvasMode::StartNewGesture;
    }
}

impl Interactable3d for GestureCanvas {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        if let Some((position, is_click)) =
            match event {
                Event3d::DragStarted { at, .. } => Some((at, true)),
                Event3d::HoverOngoing { at, .. } => Some((at, false)),
                _ => None,
            }
        {
            let finished = if is_click {
                if let Some(last_point) = *self.last_point {
                    if (position.into_2d() - last_point).norm() < CONTROL_POINT_HANDLE_RADIUS {
                        self.plan_manager.finish_gesture(self.for_machine, world);
                        self.current_mode = GestureCanvasMode::StartNewGesture;
                        self.last_point = COption(None);
                        true
                    } else {
                        self.last_point = COption(Some(position.into_2d()));
                        false
                    }
                } else {
                    self.last_point = COption(Some(position.into_2d()));
                    false
                }
            } else {
                false
            };

            if !finished {
                match self.current_mode {
                    GestureCanvasMode::StartNewGesture => {
                        if is_click {
                            let new_gesture_id = GestureID::new();

                            self.plan_manager.start_new_gesture(
                                self.proposal_id,
                                self.for_machine,
                                new_gesture_id,
                                position.into_2d(),
                                world,
                            );
                            self.current_mode =
                                GestureCanvasMode::AddToEndOfExisting(new_gesture_id);
                        }
                    }
                    GestureCanvasMode::AddToEndOfExisting(gesture_id) => {
                        self.plan_manager.add_control_point(
                            self.proposal_id,
                            gesture_id,
                            position.into_2d(),
                            true,
                            is_click,
                            world,
                        );
                    }
                    GestureCanvasMode::AddToBeginningOfExisting(gesture_id) => {
                        self.plan_manager.add_control_point(
                            self.proposal_id,
                            gesture_id,
                            position.into_2d(),
                            false,
                            is_click,
                            world,
                        );
                    }
                }
            }
        }
    }
}

// Specific stuff

#[derive(Compact, Clone)]
pub struct RoadIntent {
    n_lanes_forward: u8,
    n_lanes_backward: u8,
}

impl RoadIntent {
    pub fn new(n_lanes_forward: u8, n_lanes_backward: u8) -> Self {
        RoadIntent { n_lanes_forward, n_lanes_backward }
    }
}

#[derive(Compact, Clone)]
pub enum ZoneIntent {
    LandUse(LandUse),
    MaxHeight(u8),
    SetBack(u8),
}

#[derive(Copy, Clone)]
pub enum LandUse {
    Residential,
    Commercial,
    Industrial,
    Agricultural,
    Recreational,
    Official,
}

pub fn setup(system: &mut ActorSystem, user_interface: UserInterfaceID) {
    system.register::<PlanManager>();
    system.register::<GestureCanvas>();
    system.register::<ControlPointInteractable>();
    auto_setup(system);

    let plan_manager = PlanManagerID::spawn(&mut system.world());
    plan_manager.switch_to(user_interface, 0, &mut system.world());
}

pub mod kay_auto;
use self::kay_auto::*;