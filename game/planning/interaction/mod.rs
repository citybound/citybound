use kay::{World, MachineID, Fate, TypedID, ActorSystem, Actor, External};
use compact::{CVec, COption};
use descartes::{P2, Into2d, Area, AreaError, CurvedPath, ClosedLinePath};
use stagemaster::{UserInterfaceID, Interactable3d, Interactable3dID, Interactable2d,
Interactable2dID};
use ui_layers::UILayer;
use imgui::ImGuiSetCond_FirstUseEver;

use super::{Plan, PlanResult, GestureID, ProposalID, PlanManager, PlanManagerID, Gesture,
GestureIntent};
use transport::transport_planning::RoadIntent;
use land_use::zone_planning::{ZoneIntent, LandUse};
use construction::{Construction, Action};
use style::dimensions::CONTROL_POINT_HANDLE_RADIUS;
use stagemaster::combo::{Bindings, Combo2};
use browser_ui::BrowserUIID;

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ControlPointRef(pub GestureID, pub usize);

#[derive(Compact, Clone)]
pub struct PlanManagerUIState {
    pub current_proposal: ProposalID,
    canvas: GestureCanvasID,
    gesture_ongoing: bool,
    gesture_interactables: CVec<GestureInteractableID>,
    pub selected_points: CVec<ControlPointRef>,
    current_preview: COption<Plan>,
    current_result_preview: COption<PlanResult>,
    current_action_preview: COption<CVec<CVec<Action>>>,
    pub user_interface: UserInterfaceID,
}

impl PlanManager {
    pub fn get_all_plans(&mut self, ui: BrowserUIID, world: &mut World) {
        ui.send_all_plans(self.master_plan.clone(), self.proposals.clone(), world);
        let (line_meshes, lane_meshes, switching_lane_meshes) = self.render_preview_new(world);
        ui.send_preview(line_meshes, lane_meshes, switching_lane_meshes, world);
    }
}

impl PlanManager {
    pub fn switch_to(
        &mut self,
        user_interface: UserInterfaceID,
        proposal_id: ProposalID,
        world: &mut World,
    ) {
        let machine = user_interface.as_raw().machine;

        if let Some((current_canvas, current_interactables)) = self
            .ui_state
            .get_mut(machine)
            .map(|ui_state| (ui_state.canvas, &mut ui_state.gesture_interactables))
        {
            current_canvas.remove(user_interface, world);
            for gesture_interactable in current_interactables.drain() {
                gesture_interactable.remove(user_interface, world);
            }
        };

        self.ui_state.insert(
            machine,
            PlanManagerUIState {
                current_proposal: proposal_id,
                canvas: GestureCanvasID::spawn(user_interface, self.id, proposal_id, world),
                gesture_ongoing: false,
                gesture_interactables: CVec::new(),
                selected_points: CVec::new(),
                current_preview: COption(None),
                current_result_preview: COption(None),
                current_action_preview: COption(None),
                user_interface,
            },
        );

        self.recreate_gesture_interactables_on_machine(machine, world);
    }

    pub fn clear_previews(&mut self, proposal_id: ProposalID) {
        for state in self
            .ui_state
            .values_mut()
            .filter(|state| state.current_proposal == proposal_id)
        {
            state.current_preview = COption(None);
            state.current_result_preview = COption(None);
            state.current_action_preview = COption(None);
        }
    }

    #[allow(mutable_transmutes)]
    pub fn try_ensure_preview(
        &self,
        machine_id: MachineID,
        proposal_id: ProposalID,
        world: &mut World,
    ) -> (&Plan, Option<&PlanResult>, &Option<CVec<CVec<Action>>>) {
        let ui_state = self
            .ui_state
            .get(machine_id)
            .expect("Should already have a ui state for this machine");

        // super ugly of course, maybe we can use a cell or similar in the future
        unsafe {
            let ui_state_mut: &mut PlanManagerUIState =
                &mut *(ui_state as *const PlanManagerUIState as *mut PlanManagerUIState);
            if ui_state.current_proposal != proposal_id {
                ui_state_mut.current_preview = COption(None);
            }

            if ui_state.current_preview.is_none() {
                let preview_plan = self
                    .proposals
                    .get(proposal_id)
                    .unwrap()
                    .apply_to_with_ongoing(&self.master_plan);

                match preview_plan.calculate_result(self.master_version) {
                    Ok(preview_plan_result) => {
                        Construction::global_first(world).simulate(
                            preview_plan_result.clone(),
                            self.id,
                            proposal_id,
                            world,
                        );
                        ui_state_mut.current_result_preview = COption(Some(preview_plan_result));
                    }
                    Err(err) => match err {
                        AreaError::LeftOver(string) => {
                            println!("Preview Plan Error: {}", string);
                        }
                        _ => {
                            println!("Preview Plan Error: {:?}", err);
                        }
                    },
                }

                ui_state_mut.current_preview = COption(Some(preview_plan));
            }
        }

        (
            ui_state.current_preview.as_ref().unwrap(),
            ui_state.current_result_preview.as_ref(),
            &*ui_state.current_action_preview,
        )
    }

    pub fn on_simulated_actions(
        &mut self,
        proposal_id: ProposalID,
        actions: &CVec<CVec<Action>>,
        _: &mut World,
    ) {
        for state in self
            .ui_state
            .values_mut()
            .filter(|state| state.current_proposal == proposal_id)
        {
            // TODO: avoid clone
            state.current_action_preview = COption(Some(actions.clone()));
        }
    }

    fn recreate_gesture_interactables_on_machine(
        &mut self,
        machine_id: MachineID,
        world: &mut World,
    ) {
        let new_gesture_interactables = {
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
                let (preview, ..) = self.try_ensure_preview(machine_id, proposal_id, world);

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
                                ).into()
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .chain(
                        ::transport::transport_planning::interaction::spawn_gesture_interactables(
                            preview,
                            user_interface,
                            self.id,
                            proposal_id,
                            world,
                        ),
                    )
                    .collect()
            }
        };

        let state = self.ui_state.get_mut(machine_id).unwrap();

        for gesture_interactable in state.gesture_interactables.drain() {
            gesture_interactable.remove(state.user_interface, world);
        }

        state.gesture_interactables = new_gesture_interactables;
    }

    pub fn recreate_gesture_interactables(&mut self, proposal_id: ProposalID, world: &mut World) {
        let machines_with_this_proposal = self
            .ui_state
            .pairs()
            .filter(|&(_, state)| state.current_proposal == proposal_id)
            .map(|(machine_id, _)| *machine_id)
            .collect::<Vec<_>>();

        for machine_id in machines_with_this_proposal {
            self.recreate_gesture_interactables_on_machine(machine_id, world);
        }
    }

    pub fn select_point(
        &mut self,
        point_ref: ControlPointRef,
        machine_id: MachineID,
        world: &mut World,
    ) {
        {
            let ui_state = self
                .ui_state
                .get_mut(machine_id)
                .expect("should already have ui state");
            if !ui_state.selected_points.contains(&point_ref) {
                ui_state.selected_points.push(point_ref);
            }
        }
        self.recreate_gesture_interactables_on_machine(machine_id, world);
    }

    pub fn clear_selection(&mut self, machine_id: MachineID, world: &mut World) {
        {
            let ui_state = self
                .ui_state
                .get_mut(machine_id)
                .expect("should already have ui state");
            ui_state.selected_points.clear();
        }
        self.recreate_gesture_interactables_on_machine(machine_id, world);
    }

    pub fn start_new_gesture(
        &mut self,
        proposal_id: ProposalID,
        machine_id: MachineID,
        new_gesture_id: GestureID,
        intent: &GestureIntent,
        start: P2,
        world: &mut World,
    ) {
        let new_gesture = Gesture::new(vec![start].into(), intent.clone());

        let new_step = Plan {
            gestures: Some((new_gesture_id, new_gesture)).into_iter().collect(),
        };

        self.proposals
            .get_mut(proposal_id)
            .unwrap()
            .set_ongoing_step(new_step);
        self.proposals
            .get_mut(proposal_id)
            .unwrap()
            .start_new_step();

        self.ui_state
            .get_mut(machine_id)
            .expect("should already have ui state")
            .gesture_ongoing = true;

        self.clear_previews(proposal_id);
        self.recreate_gesture_interactables(proposal_id, world);
    }

    pub fn finish_gesture(&mut self, machine_id: MachineID, world: &mut World) {
        self.ui_state
            .get_mut(machine_id)
            .expect("should already have ui state")
            .gesture_ongoing = false;
        self.recreate_gesture_interactables_on_machine(machine_id, world);
    }

    pub fn add_control_point(
        &mut self,
        proposal_id: ProposalID,
        gesture_id: GestureID,
        new_point: P2,
        add_to_end: bool,
        commit: bool,
        world: &mut World,
    ) {
        let new_step = {
            let current_gesture = self.get_current_version_of(gesture_id, proposal_id);

            let changed_gesture = if add_to_end {
                Gesture {
                    points: current_gesture
                        .points
                        .iter()
                        .cloned()
                        .chain(Some(new_point))
                        .collect(),
                    ..current_gesture.clone()
                }
            } else {
                Gesture {
                    points: Some(new_point)
                        .into_iter()
                        .chain(current_gesture.points.iter().cloned())
                        .collect(),
                    ..current_gesture.clone()
                }
            };

            Plan {
                gestures: Some((gesture_id, changed_gesture)).into_iter().collect(),
            }
        };

        self.proposals
            .get_mut(proposal_id)
            .unwrap()
            .set_ongoing_step(new_step);

        if commit {
            self.proposals
                .get_mut(proposal_id)
                .unwrap()
                .start_new_step();
        }

        self.clear_previews(proposal_id);
        self.recreate_gesture_interactables(proposal_id, world);
    }

    pub fn move_control_point(
        &mut self,
        proposal_id: ProposalID,
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

            Plan {
                gestures: Some((gesture_id, new_gesture)).into_iter().collect(),
            }
        };

        self.proposals
            .get_mut(proposal_id)
            .unwrap()
            .set_ongoing_step(current_change);

        // TODO: can we update only part of the preview
        // for better rendering performance while dragging?
        self.clear_previews(proposal_id);

        if is_move_finished {
            self.proposals
                .get_mut(proposal_id)
                .unwrap()
                .start_new_step();
            self.recreate_gesture_interactables(proposal_id, world);
        }
    }

    pub fn set_intent(
        &mut self,
        proposal_id: ProposalID,
        gesture_id: GestureID,
        new_intent: &GestureIntent,
        is_move_finished: bool,
        world: &mut World,
    ) {
        let current_change = {
            let current_gesture = self.get_current_version_of(gesture_id, proposal_id);

            let new_gesture = Gesture {
                intent: new_intent.clone(),
                ..current_gesture.clone()
            };

            Plan {
                gestures: Some((gesture_id, new_gesture)).into_iter().collect(),
            }
        };

        self.proposals
            .get_mut(proposal_id)
            .unwrap()
            .set_ongoing_step(current_change);

        // TODO: can we update only part of the preview
        // for better rendering performance while dragging?
        self.clear_previews(proposal_id);

        if is_move_finished {
            self.proposals
                .get_mut(proposal_id)
                .unwrap()
                .start_new_step();
            self.recreate_gesture_interactables(proposal_id, world);
        }
    }

    pub fn undo(&mut self, proposal_id: ProposalID, world: &mut World) {
        self.proposals.get_mut(proposal_id).unwrap().undo();
        self.clear_previews(proposal_id);
        self.recreate_gesture_interactables(proposal_id, world);
    }

    pub fn redo(&mut self, proposal_id: ProposalID, world: &mut World) {
        self.proposals.get_mut(proposal_id).unwrap().redo();
        self.clear_previews(proposal_id);
        self.recreate_gesture_interactables(proposal_id, world);
    }
}

pub trait GestureInteractable {
    fn remove(&self, user_interface: UserInterfaceID, world: &mut World) -> Fate;
}

#[derive(Compact, Clone)]
pub struct ControlPointInteractable {
    id: ControlPointInteractableID,
    plan_manager: PlanManagerID,
    proposal_id: ProposalID,
    gesture_id: GestureID,
    point_index: usize,
}

impl ControlPointInteractable {
    #[cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]
    pub fn spawn(
        id: ControlPointInteractableID,
        user_interface: UserInterfaceID,
        plan_manager: PlanManagerID,
        proposal_id: ProposalID,
        gesture_id: GestureID,
        point_index: usize,
        position: P2,
        world: &mut World,
    ) -> Self {
        user_interface.add(
            UILayer::Gesture as usize,
            id.into(),
            COption(Some(Area::new_simple(
                ClosedLinePath::new(
                    CurvedPath::circle(position, CONTROL_POINT_HANDLE_RADIUS)
                        .unwrap()
                        .to_line_path(),
                ).unwrap(),
            ))),
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
}

impl GestureInteractable for ControlPointInteractable {
    fn remove(&self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(UILayer::Gesture as usize, self.id.into(), world);
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

        if let Some((_from, to, is_finished)) = drag_info {
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

#[derive(Serialize, Deserialize, Clone)]
pub struct PlanManagerSettings {
    pub bindings: Bindings,
}

impl Default for PlanManagerSettings {
    fn default() -> Self {
        use stagemaster::combo::Button::*;

        PlanManagerSettings {
            bindings: Bindings::new(vec![
                ("Implement Plan", Combo2::new(&[Return], &[])),
                ("Undo", Combo2::new(&[LControl, Z], &[LWin, Z])),
                (
                    "Redo",
                    Combo2::new(&[LControl, LShift, Z], &[LWin, LShift, Z]),
                ),
            ]),
        }
    }
}

#[derive(Compact, Clone)]
pub struct GestureCanvas {
    id: GestureCanvasID,
    plan_manager: PlanManagerID,
    for_machine: MachineID,
    proposal_id: ProposalID,
    last_point: COption<P2>,
    current_mode: GestureCanvasMode,
    current_intent: GestureIntent,
    settings: External<PlanManagerSettings>,
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
        proposal_id: ProposalID,
        world: &mut World,
    ) -> Self {
        user_interface.add(
            UILayer::Gesture as usize,
            id.into(),
            COption(None),
            0,
            world,
        );
        user_interface.add_2d(id.into(), world);
        user_interface.focus(id.into(), world);

        GestureCanvas {
            id,
            for_machine: user_interface.as_raw().machine,
            plan_manager,
            proposal_id,
            last_point: COption(None),
            current_mode: GestureCanvasMode::StartNewGesture,
            current_intent: GestureIntent::Road(RoadIntent::new(2, 2)),
            settings: External::new(::ENV.load_settings("Planning")),
        }
    }

    pub fn remove(&self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.unfocus(self.id.into(), world);
        user_interface.remove(UILayer::Gesture as usize, self.id.into(), world);
        user_interface.remove_2d(self.id.into(), world);
        Fate::Die
    }
}

impl Interactable3d for GestureCanvas {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        match event {
            Event3d::Combos(combos) => {
                self.settings.bindings.do_rebinding(&combos.current);
                let bindings = &self.settings.bindings;

                if bindings["Implement Plan"].is_freshly_in(&combos) {
                    self.plan_manager.implement(self.proposal_id, world);
                }

                if bindings["Redo"].is_freshly_in(&combos) {
                    self.current_mode = GestureCanvasMode::StartNewGesture;
                    self.plan_manager.redo(self.proposal_id, world);
                } else if bindings["Undo"].is_freshly_in(&combos) {
                    self.current_mode = GestureCanvasMode::StartNewGesture;
                    self.plan_manager.undo(self.proposal_id, world);
                }
            }
            _ => {
                if let Some((position, is_click)) = match event {
                    Event3d::DragStarted { at, .. } => Some((at, true)),
                    Event3d::HoverOngoing { at, .. } => Some((at, false)),
                    _ => None,
                } {
                    let hovering_last = if let Some(last_point) = *self.last_point {
                        (position.into_2d() - last_point).norm() < CONTROL_POINT_HANDLE_RADIUS
                    } else {
                        false
                    };

                    if is_click {
                        if hovering_last {
                            self.plan_manager.finish_gesture(self.for_machine, world);
                            self.current_mode = GestureCanvasMode::StartNewGesture;
                            self.last_point = COption(None);
                        } else {
                            self.last_point = COption(Some(position.into_2d()));
                        }
                    }

                    if !hovering_last {
                        match self.current_mode {
                            GestureCanvasMode::StartNewGesture => {
                                if is_click {
                                    let new_gesture_id = GestureID::new();

                                    self.plan_manager.start_new_gesture(
                                        self.proposal_id,
                                        self.for_machine,
                                        new_gesture_id,
                                        // GestureIntent::Road(RoadIntent::new(2, 2)),
                                        //GestureIntent::Zone(ZoneIntent::LandUse(LandUse::
                                        // Residential)),
                                        self.current_intent.clone(),
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
    }
}

impl Interactable2d for GestureCanvas {
    fn draw(&mut self, world: &mut World, ui: &::imgui::Ui<'static>) {
        ui.window(im_str!("Canvas Mode"))
            .size((200.0, 50.0), ImGuiSetCond_FirstUseEver)
            .collapsible(false)
            .build(|| {
                if ui.small_button(im_str!("Road")) {
                    self.current_intent = GestureIntent::Road(RoadIntent::new(2, 2));
                }
                if ui.small_button(im_str!("Zone")) {
                    self.current_intent =
                        GestureIntent::Zone(ZoneIntent::LandUse(LandUse::Residential));
                }
                if ui.small_button(im_str!("Implement")) {
                    self.plan_manager.implement(self.proposal_id, world);
                }
                if ui.small_button(im_str!("Build 10x10 grid")) {
                    use transport::transport_planning::RoadIntent;
                    use super::{GestureID};
                    use descartes::N;

                    const GRID_SPACING: N = 200.0;

                    for x in 0..10 {
                        let id = GestureID::new();
                        let p1 = P2::new(x as f32 * GRID_SPACING, 0.0);
                        let p2 = P2::new(x as f32 * GRID_SPACING, 10.0 * GRID_SPACING);
                        self.plan_manager.start_new_gesture(
                            self.proposal_id,
                            self.for_machine,
                            id,
                            GestureIntent::Road(RoadIntent::new(3, 3)),
                            p1,
                            world,
                        );
                        self.plan_manager.add_control_point(
                            self.proposal_id,
                            id,
                            p2,
                            true,
                            true,
                            world,
                        );
                        self.plan_manager.finish_gesture(self.for_machine, world);
                    }

                    for y in 0..10 {
                        let id = GestureID::new();
                        let p1 = P2::new(0.0, y as f32 * GRID_SPACING);
                        let p2 = P2::new(10.0 * GRID_SPACING, y as f32 * GRID_SPACING);
                        self.plan_manager.start_new_gesture(
                            self.proposal_id,
                            self.for_machine,
                            id,
                            GestureIntent::Road(RoadIntent::new(3, 3)),
                            p1,
                            world,
                        );
                        self.plan_manager.add_control_point(
                            self.proposal_id,
                            id,
                            p2,
                            true,
                            true,
                            world,
                        );
                        self.plan_manager.finish_gesture(self.for_machine, world);
                    }
                }
                if ui.small_button(im_str!("Spawn cars")) {
                    use transport::lane::Lane;
                    for _ in (0..50) {
                        Lane::global_broadcast(world).manually_spawn_car_add_lane(world);
                    }
                }
            });

        ui.window(im_str!("Settings")).build(|| {
            ui.text(im_str!("Planning"));
            ui.separator();

            if self.settings.bindings.settings_ui(&ui) {
                ::ENV.write_settings("Planning", &*self.settings)
            }

            ui.spacing();
        });
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<GestureCanvas>();
    system.register::<ControlPointInteractable>();
    auto_setup(system);
}

pub mod kay_auto;
pub use self::kay_auto::*;
