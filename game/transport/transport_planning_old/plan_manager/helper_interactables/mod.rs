use kay::{ActorSystem, Fate, World, Actor};
use compact::CVec;
use descartes::{P2, N, Band, Into2d, Curve, FiniteCurve, Path, RoughlyComparable};
use stagemaster::geometry::{AnyShape, CPath};
use planning_old::plan_manager::{PlanManagerID, Intent, IntentProgress};

use stagemaster::{UserInterfaceID, Event3d, Interactable3d, Interactable3dID};

use super::RoadIntent;

#[derive(Compact, Clone)]
pub struct Deselecter {
    id: DeselecterID,
    plan_manager: PlanManagerID,
}

impl Deselecter {
    pub fn spawn(
        id: DeselecterID,
        user_interface: UserInterfaceID,
        plan_manager: PlanManagerID,
        world: &mut World,
    ) -> Deselecter {
        user_interface.add(
            ::ui_layers::ROAD_LAYER,
            id.into(),
            AnyShape::Everywhere,
            2,
            world,
        );
        Deselecter { id, plan_manager }
    }

    pub fn clear(&mut self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(::ui_layers::ROAD_LAYER, self.id_as(), world);
        Fate::Die
    }
}

impl Interactable3d for Deselecter {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        if let Event3d::DragFinished { .. } = event {
            self.plan_manager.change_intent(
                Intent::RoadIntent(RoadIntent::Deselect),
                IntentProgress::Immediate,
                world,
            );
        }
    }
}

#[derive(Compact, Clone)]
pub struct Addable {
    id: AddableID,
    path: CPath,
    plan_manager: PlanManagerID,
}

impl Addable {
    pub fn spawn(
        id: AddableID,
        path: &CPath,
        user_interface: UserInterfaceID,
        plan_manager: PlanManagerID,
        world: &mut World,
    ) -> Addable {
        user_interface.add(
            ::ui_layers::ROAD_LAYER,
            id.into(),
            AnyShape::Band(Band::new(path.clone(), 3.0)),
            3,
            world,
        );

        Addable { id, path: path.clone(), plan_manager }
    }

    pub fn clear(&mut self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(::ui_layers::ROAD_LAYER, self.id_as(), world);
        Fate::Die
    }
}

impl Interactable3d for Addable {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        match event {
            Event3d::HoverStarted { .. } |
            Event3d::HoverOngoing { .. } => {
                self.plan_manager.change_intent(
                    Intent::RoadIntent(RoadIntent::CreateNextLane),
                    IntentProgress::Preview,
                    world,
                );
            }
            Event3d::HoverStopped => {
                self.plan_manager.change_intent(
                    Intent::None,
                    IntentProgress::Preview,
                    world,
                );
            }
            Event3d::DragStarted { .. } => {
                self.plan_manager.change_intent(
                    Intent::RoadIntent(RoadIntent::CreateNextLane),
                    IntentProgress::Immediate,
                    world,
                );
            }
            _ => {}
        };
    }
}

use super::SelectableStrokeRef;

#[derive(Compact, Clone)]
pub struct Draggable {
    id: DraggableID,
    stroke_ref: SelectableStrokeRef,
    path: CPath,
    plan_manager: PlanManagerID,
}

impl Draggable {
    pub fn spawn(
        id: DraggableID,
        stroke_ref: SelectableStrokeRef,
        path: &CPath,
        user_interface: UserInterfaceID,
        plan_manager: PlanManagerID,
        world: &mut World,
    ) -> Draggable {
        user_interface.add(
            ::ui_layers::ROAD_LAYER,
            id.into(),
            AnyShape::Band(Band::new(path.clone(), 5.0)),
            4,
            world,
        );

        Draggable {
            id,
            stroke_ref,
            path: path.clone(),
            plan_manager,
        }
    }

    pub fn clear(&mut self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(::ui_layers::ROAD_LAYER, self.id_as(), world);
        Fate::Die
    }
}

impl Interactable3d for Draggable {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        const MAXIMIZE_DISTANCE: N = 0.5;

        match event {
            Event3d::DragOngoing { from, to, .. } => {
                self.plan_manager.change_intent(
                    Intent::RoadIntent(
                        RoadIntent::MoveSelection(to.into_2d() - from.into_2d()),
                    ),
                    IntentProgress::Preview,
                    world,
                );
            }
            Event3d::DragFinished { from, to, .. } => {
                let delta = to.into_2d() - from.into_2d();
                if delta.norm() < MAXIMIZE_DISTANCE {
                    self.plan_manager.change_intent(
                        Intent::RoadIntent(RoadIntent::MaximizeSelection),
                        IntentProgress::Immediate,
                        world,
                    );
                } else {
                    self.plan_manager.change_intent(
                        Intent::RoadIntent(RoadIntent::MoveSelection(delta)),
                        IntentProgress::Immediate,
                        world,
                    )
                }
            }
            _ => {}
        };
    }
}

#[derive(Compact, Clone)]
pub struct Selectable {
    id: SelectableID,
    stroke_ref: SelectableStrokeRef,
    path: CPath,
    plan_manager: PlanManagerID,
}

impl Selectable {
    pub fn spawn(
        id: SelectableID,
        stroke_ref: SelectableStrokeRef,
        path: &CPath,
        user_interface: UserInterfaceID,
        plan_manager: PlanManagerID,
        world: &mut World,
    ) -> Selectable {
        user_interface.add(
            ::ui_layers::ROAD_LAYER,
            id.into(),
            AnyShape::Band(Band::new(path.clone(), 5.0)),
            3,
            world,
        );

        Selectable {
            id,
            stroke_ref,
            path: path.clone(),
            plan_manager,
        }
    }

    pub fn clear(&mut self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(::ui_layers::ROAD_LAYER, self.id_as(), world);
        Fate::Die
    }
}

use super::ContinuationMode;

impl Interactable3d for Selectable {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        match event {
            Event3d::DragOngoing { from, to, .. } => {
                if let (Some(selection_start), Some(selection_end)) =
                    (
                        self.path.project_with_tolerance(
                            from.into_2d(),
                            SELECTION_OVERSHOOT_TOLERANCE,
                        ),
                        self.path.project_with_tolerance(
                            to.into_2d(),
                            SELECTION_OVERSHOOT_TOLERANCE,
                        ),
                    )
                {
                    let mut start = selection_start.min(selection_end);
                    let mut end = selection_end.max(selection_start);
                    snap_start_end(&mut start, &mut end, &self.path);
                    self.plan_manager.change_intent(
                        Intent::RoadIntent(
                            RoadIntent::Select(self.stroke_ref, start, end),
                        ),
                        IntentProgress::Preview,
                        world,
                    );
                } else {
                    self.plan_manager.change_intent(
                        Intent::None,
                        IntentProgress::Preview,
                        world,
                    );
                }
            }
            Event3d::DragFinished { from, to, .. } => {
                if let (Some(selection_start), Some(selection_end)) =
                    (
                        self.path.project_with_tolerance(
                            from.into_2d(),
                            SELECTION_OVERSHOOT_TOLERANCE,
                        ),
                        self.path.project_with_tolerance(
                            to.into_2d(),
                            SELECTION_OVERSHOOT_TOLERANCE,
                        ),
                    )
                {
                    let mut start = selection_start.min(selection_end);
                    let mut end = selection_end.max(selection_start);
                    if end < CONTINUE_DISTANCE {
                        self.plan_manager.change_intent(
                            Intent::RoadIntent(RoadIntent::ContinueRoadAround(
                                self.stroke_ref,
                                ContinuationMode::Prepend,
                                to.into_2d(),
                            )),
                            IntentProgress::Finished,
                            world,
                        );
                    } else if start > self.path.length() - CONTINUE_DISTANCE {
                        self.plan_manager.change_intent(
                            Intent::RoadIntent(RoadIntent::ContinueRoadAround(
                                self.stroke_ref,
                                ContinuationMode::Append,
                                to.into_2d(),
                            )),
                            IntentProgress::Finished,
                            world,
                        );
                    } else {
                        snap_start_end(&mut start, &mut end, &self.path);
                        start = start.min(end - MIN_SELECTION_SIZE).max(0.0);
                        end = end.max(start + MIN_SELECTION_SIZE).min(self.path.length());
                        self.plan_manager.change_intent(
                            Intent::RoadIntent(
                                RoadIntent::Select(self.stroke_ref, start, end),
                            ),
                            IntentProgress::Immediate,
                            world,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

const START_END_SNAP_DISTANCE: N = 10.0;
const SEGMENT_SNAP_DISTANCE: N = 5.0;
const CONTINUE_DISTANCE: N = 6.0;
const MIN_SELECTION_SIZE: N = 2.0;
const SELECTION_OVERSHOOT_TOLERANCE: N = 30.0;

fn snap_start_end(start: &mut N, end: &mut N, path: &CPath) {
    if *start < START_END_SNAP_DISTANCE {
        *start = 0.0
    }
    if *end > path.length() - START_END_SNAP_DISTANCE {
        *end = path.length()
    }
    let mut offset = 0.0;
    for segment in path.segments() {
        let next_offset = offset + segment.length();
        if start.is_roughly_within(offset, SEGMENT_SNAP_DISTANCE) {
            *start = offset
        }
        if end.is_roughly_within(next_offset, SEGMENT_SNAP_DISTANCE) {
            *end = next_offset
        }
        offset = next_offset;
    }
}

#[derive(Compact, Clone)]
pub struct StrokeCanvas {
    id: StrokeCanvasID,
    points: CVec<P2>,
    plan_manager: PlanManagerID,
}

impl StrokeCanvas {
    pub fn spawn(
        id: StrokeCanvasID,
        user_interface: UserInterfaceID,
        plan_manager: PlanManagerID,
        world: &mut World,
    ) -> StrokeCanvas {
        user_interface.add(
            ::ui_layers::ROAD_LAYER,
            id.into(),
            AnyShape::Everywhere,
            1,
            world,
        );
        StrokeCanvas { id, points: CVec::new(), plan_manager }
    }

    pub fn set_points(&mut self, points: &CVec<P2>, _: &mut World) {
        self.points = points.clone();
    }

    // probably never called
    pub fn clear(&mut self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(::ui_layers::ROAD_LAYER, self.id_as(), world);
        Fate::Die
    }
}

#[derive(Copy, Clone)]
pub enum StrokeState {
    Preview,
    Intermediate,
    Finished,
}

#[derive(Compact, Clone)]
pub struct Stroke(pub CVec<P2>, pub StrokeState);

const FINISH_STROKE_TOLERANCE: f32 = 5.0;

impl Interactable3d for StrokeCanvas {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        match event {
            Event3d::HoverStarted { at, .. } |
            Event3d::HoverOngoing { at, .. } => {
                let mut preview_points = self.points.clone();
                preview_points.push(at.into_2d());
                self.plan_manager.on_stroke(
                    preview_points,
                    StrokeState::Preview,
                    world,
                );
            }
            Event3d::DragStarted { at, .. } => {
                let new_point = at.into_2d();
                let maybe_last_point = self.points.last().cloned();

                let finished = if let Some(last_point) = maybe_last_point {
                    if new_point.is_roughly_within(last_point, FINISH_STROKE_TOLERANCE) {
                        self.plan_manager.on_stroke(
                            self.points.clone(),
                            StrokeState::Finished,
                            world,
                        );
                        self.points.clear();
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };

                if !finished {
                    self.points.push(new_point);
                    if self.points.len() > 1 {
                        self.plan_manager.on_stroke(
                            self.points.clone(),
                            StrokeState::Intermediate,
                            world,
                        );
                    }
                }
            }
            _ => {}
        };
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Deselecter>();
    system.register::<Addable>();
    system.register::<Draggable>();
    system.register::<Selectable>();
    system.register::<StrokeCanvas>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
