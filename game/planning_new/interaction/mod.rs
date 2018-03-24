use kay::{World, MachineID, Fate, TypedID, ActorSystem};
use compact::{CVec, COption};
use descartes::{N, P2, Into2d, Circle, Path};
use stagemaster::{UserInterfaceID, Interactable3d, Interactable3dID};
use stagemaster::geometry::AnyShape;
use ui_layers::GESTURE_LAYER;

use super::{Plan, PlanResult, GestureID, PlanManager, PlanManagerID, Gesture, GestureIntent};
use super::transport_planning_new::RoadIntent;

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ControlPointRef(pub GestureID, pub usize);

#[derive(Compact, Clone)]
pub struct PlanManagerUIState {
    pub current_proposal: usize,
    canvas: GestureCanvasID,
    gesture_ongoing: bool,
    control_point_interactables: CVec<ControlPointInteractableID>,
    pub selected_points: CVec<ControlPointRef>,
    current_preview: COption<Plan>,
    current_result_preview: COption<PlanResult>,
    user_interface: UserInterfaceID,
}

impl PlanManager {
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
    pub fn ensure_preview(
        &self,
        machine_id: MachineID,
        proposal_id: usize,
    ) -> (&Plan, &PlanResult) {
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

#[derive(Compact, Clone)]
pub struct ControlPointInteractable {
    id: ControlPointInteractableID,
    plan_manager: PlanManagerID,
    proposal_id: usize,
    gesture_id: GestureID,
    point_index: usize,
}

pub const CONTROL_POINT_HANDLE_RADIUS: N = 3.0;

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

pub fn setup(system: &mut ActorSystem) {
    system.register::<GestureCanvas>();
    system.register::<ControlPointInteractable>();

    auto_setup(system);
}

pub mod kay_auto;
use self::kay_auto::*;