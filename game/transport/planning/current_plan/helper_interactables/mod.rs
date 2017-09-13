use kay::{ActorSystem, Fate, World};
use kay::swarm::Swarm;
use compact::CVec;
use descartes::{P2, N, Norm, Band, Into2d, Curve, FiniteCurve, Path, RoughlyComparable};
use stagemaster::geometry::{AnyShape, CPath};
use super::CurrentPlanID;

use stagemaster::{UserInterfaceID, Event3d, Interactable3d, Interactable3dID,
                  MSG_Interactable3d_on_event};

#[derive(Compact, Clone)]
pub struct Deselecter {
    id: DeselecterID,
    current_plan: CurrentPlanID,
}

impl Deselecter {
    pub fn spawn(
        id: DeselecterID,
        user_interface: UserInterfaceID,
        current_plan: CurrentPlanID,
        world: &mut World,
    ) -> Deselecter {
        user_interface.add(id.into(), AnyShape::Everywhere, 2, world);
        Deselecter { id, current_plan }
    }

    pub fn clear(&mut self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(self.id.into(), world);
        Fate::Die
    }
}

impl Interactable3d for Deselecter {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        if let Event3d::DragFinished { .. } = event {
            self.current_plan.change_intent(
                Intent::Deselect,
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
    current_plan: CurrentPlanID,
}

impl Addable {
    pub fn spawn(
        id: AddableID,
        path: &CPath,
        user_interface: UserInterfaceID,
        current_plan: CurrentPlanID,
        world: &mut World,
    ) -> Addable {
        user_interface.add(
            id.into(),
            AnyShape::Band(Band::new(path.clone(), 3.0)),
            3,
            world,
        );

        Addable { id, path: path.clone(), current_plan }
    }

    pub fn clear(&mut self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(self.id.into(), world);
        Fate::Die
    }
}

impl Interactable3d for Addable {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        match event {
            Event3d::HoverStarted { .. } |
            Event3d::HoverOngoing { .. } => {
                self.current_plan.change_intent(
                    Intent::CreateNextLane,
                    IntentProgress::Preview,
                    world,
                );
            }
            Event3d::HoverStopped => {
                self.current_plan.change_intent(
                    Intent::None,
                    IntentProgress::Preview,
                    world,
                );
            }
            Event3d::DragStarted { .. } => {
                self.current_plan.change_intent(
                    Intent::CreateNextLane,
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
    current_plan: CurrentPlanID,
}

impl Draggable {
    pub fn spawn(
        id: DraggableID,
        stroke_ref: SelectableStrokeRef,
        path: &CPath,
        user_interface: UserInterfaceID,
        current_plan: CurrentPlanID,
        world: &mut World,
    ) -> Draggable {
        user_interface.add(
            id.into(),
            AnyShape::Band(Band::new(path.clone(), 5.0)),
            4,
            world,
        );

        Draggable {
            id,
            stroke_ref,
            path: path.clone(),
            current_plan,
        }
    }

    pub fn clear(&mut self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(self.id.into(), world);
        Fate::Die
    }
}

impl Interactable3d for Draggable {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        const MAXIMIZE_DISTANCE: N = 0.5;

        match event {
            Event3d::DragOngoing { from, to, .. } => {
                self.current_plan.change_intent(
                    Intent::MoveSelection(to.into_2d() - from.into_2d()),
                    IntentProgress::Preview,
                    world,
                );
            }
            Event3d::DragFinished { from, to, .. } => {
                let delta = to.into_2d() - from.into_2d();
                if delta.norm() < MAXIMIZE_DISTANCE {
                    self.current_plan.change_intent(
                        Intent::MaximizeSelection,
                        IntentProgress::Immediate,
                        world,
                    );
                } else {
                    self.current_plan.change_intent(
                        Intent::MoveSelection(delta),
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
    current_plan: CurrentPlanID,
}

impl Selectable {
    pub fn spawn(
        id: SelectableID,
        stroke_ref: SelectableStrokeRef,
        path: &CPath,
        user_interface: UserInterfaceID,
        current_plan: CurrentPlanID,
        world: &mut World,
    ) -> Selectable {
        user_interface.add(
            id.into(),
            AnyShape::Band(Band::new(path.clone(), 5.0)),
            3,
            world,
        );

        Selectable {
            id,
            stroke_ref,
            path: path.clone(),
            current_plan,
        }
    }

    pub fn clear(&mut self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(self.id.into(), world);
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
                    self.current_plan.change_intent(
                        Intent::Select(self.stroke_ref, start, end),
                        IntentProgress::Preview,
                        world,
                    );
                } else {
                    self.current_plan.change_intent(
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
                        self.current_plan.change_intent(
                            Intent::ContinueRoadAround(
                                self.stroke_ref,
                                ContinuationMode::Prepend,
                                to.into_2d(),
                            ),
                            IntentProgress::Finished,
                            world,
                        );
                    } else if start > self.path.length() - CONTINUE_DISTANCE {
                        self.current_plan.change_intent(
                            Intent::ContinueRoadAround(
                                self.stroke_ref,
                                ContinuationMode::Append,
                                to.into_2d(),
                            ),
                            IntentProgress::Finished,
                            world,
                        );
                    } else {
                        snap_start_end(&mut start, &mut end, &self.path);
                        start = start.min(end - MIN_SELECTION_SIZE).max(0.0);
                        end = end.max(start + MIN_SELECTION_SIZE).min(self.path.length());
                        self.current_plan.change_intent(
                            Intent::Select(self.stroke_ref, start, end),
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
    current_plan: CurrentPlanID,
}

impl StrokeCanvas {
    pub fn spawn(
        id: StrokeCanvasID,
        user_interface: UserInterfaceID,
        current_plan: CurrentPlanID,
        world: &mut World,
    ) -> StrokeCanvas {
        user_interface.add(id.into(), AnyShape::Everywhere, 1, world);
        StrokeCanvas { id, points: CVec::new(), current_plan }
    }

    pub fn set_points(&mut self, points: &CVec<P2>, _: &mut World) {
        self.points = points.clone();
    }

    // probably never called
    pub fn clear(&mut self, user_interface: UserInterfaceID, world: &mut World) -> Fate {
        user_interface.remove(self.id.into(), world);
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
                self.current_plan.on_stroke(
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
                        self.current_plan.on_stroke(
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
                        self.current_plan.on_stroke(
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
    system.add(Swarm::<Deselecter>::new(), |_| {});
    system.add(Swarm::<Addable>::new(), |_| {});
    system.add(Swarm::<Draggable>::new(), |_| {});
    system.add(Swarm::<Selectable>::new(), |_| {});
    system.add(Swarm::<StrokeCanvas>::new(), |_| {});
    auto_setup(system);
}

use super::{Intent, IntentProgress};

mod kay_auto;
pub use self::kay_auto::*;
