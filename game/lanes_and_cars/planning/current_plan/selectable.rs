use kay::{ID, ActorSystem, Fate};
use kay::swarm::{Swarm, SubActor};
use descartes::{N, Band, Curve, Into2d, FiniteCurve, Path, RoughlyComparable};
use stagemaster::geometry::{CPath, AnyShape};

use super::{SelectableStrokeRef, CurrentPlan};

#[derive(SubActor, Compact, Clone)]
pub struct Selectable {
    _id: Option<ID>,
    stroke_ref: SelectableStrokeRef,
    path: CPath,
}

impl Selectable {
    pub fn new(stroke_ref: SelectableStrokeRef, path: CPath) -> Self {
        Selectable {
            _id: None,
            stroke_ref: stroke_ref,
            path: path,
        }
    }
}

use super::InitInteractable;
use stagemaster::{UserInterface, AddInteractable};

pub fn setup(system: &mut ActorSystem) {
    system.add(
        Swarm::<Selectable>::new(),
        Swarm::<Selectable>::subactors(|mut each_selectable| {
            let ui_id = each_selectable.world().id::<UserInterface>();
            let cp_id = each_selectable.world().id::<CurrentPlan>();

            each_selectable.on_create_with(move |_: &InitInteractable, selectable, world| {
                world.send(
                    ui_id,
                    AddInteractable(
                        selectable.id(),
                        AnyShape::Band(Band::new(selectable.path.clone(), 5.0)),
                        3,
                    ),
                );
                Fate::Live
            });

            each_selectable.on(move |_: &ClearInteractable, selectable, world| {
                world.send(ui_id, RemoveInteractable(selectable.id()));
                Fate::Die
            });

            each_selectable.on(move |&event, selectable, world| {
                match event {
                    Event3d::DragOngoing { from, to, .. } => {
                        if let (Some(selection_start), Some(selection_end)) =
                            (
                                selectable.path.project_with_tolerance(
                                    from.into_2d(),
                                    SELECTION_OVERSHOOT_TOLERANCE,
                                ),
                                selectable.path.project_with_tolerance(
                                    to.into_2d(),
                                    SELECTION_OVERSHOOT_TOLERANCE,
                                ),
                            )
                        {
                            let mut start = selection_start.min(selection_end);
                            let mut end = selection_end.max(selection_start);
                            snap_start_end(&mut start, &mut end, &selectable.path);
                            world.send(
                                cp_id,
                                ChangeIntent(
                                    Intent::Select(selectable.stroke_ref, start, end),
                                    IntentProgress::Preview,
                                ),
                            );
                        } else {
                            world.send(cp_id, ChangeIntent(Intent::None, IntentProgress::Preview));
                        }
                    }
                    Event3d::DragFinished { from, to, .. } => {
                        if let (Some(selection_start), Some(selection_end)) =
                            (
                                selectable.path.project_with_tolerance(
                                    from.into_2d(),
                                    SELECTION_OVERSHOOT_TOLERANCE,
                                ),
                                selectable.path.project_with_tolerance(
                                    to.into_2d(),
                                    SELECTION_OVERSHOOT_TOLERANCE,
                                ),
                            )
                        {
                            let mut start = selection_start.min(selection_end);
                            let mut end = selection_end.max(selection_start);
                            if end < CONTINUE_DISTANCE {
                                world.send(
                                    cp_id,
                                    ChangeIntent(
                                        Intent::ContinueRoadAround(
                                            selectable.stroke_ref,
                                            ContinuationMode::Prepend,
                                            to.into_2d(),
                                        ),
                                        IntentProgress::Finished,
                                    ),
                                );
                            } else if start > selectable.path.length() - CONTINUE_DISTANCE {
                                world.send(
                                    cp_id,
                                    ChangeIntent(
                                        Intent::ContinueRoadAround(
                                            selectable.stroke_ref,
                                            ContinuationMode::Append,
                                            to.into_2d(),
                                        ),
                                        IntentProgress::Finished,
                                    ),
                                );
                            } else {
                                snap_start_end(&mut start, &mut end, &selectable.path);
                                start = start.min(end - MIN_SELECTION_SIZE).max(0.0);
                                end = end.max(start + MIN_SELECTION_SIZE).min(
                                    selectable.path.length(),
                                );
                                world.send(
                                    cp_id,
                                    ChangeIntent(
                                        Intent::Select(selectable.stroke_ref, start, end),
                                        IntentProgress::Immediate,
                                    ),
                                );
                            }
                        }
                    }
                    _ => {}
                }
                Fate::Live
            });
        }),
    );
}

use super::ClearInteractable;
use stagemaster::RemoveInteractable;
use stagemaster::Event3d;
use super::{ChangeIntent, Intent, IntentProgress, ContinuationMode};

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
