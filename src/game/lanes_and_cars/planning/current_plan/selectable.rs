use kay::{ID, Recipient, Actor, Fate};
use kay::swarm::{Swarm, SubActor, CreateWith};
use descartes::{N, Band, Curve, Into2d, FiniteCurve, Path, RoughlyComparable};
use core::geometry::{CPath, AnyShape};

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
use core::ui::Add;

impl Recipient<InitInteractable> for Selectable {
    fn receive(&mut self, _msg: &InitInteractable) -> Fate {
        ::core::ui::UserInterface::id() <<
        Add::Interactable3d(self.id(),
                            AnyShape::Band(Band::new(self.path.clone(), 5.0)),
                            3);
        Fate::Live
    }
}

use super::ClearInteractable;
use core::ui::Remove;

impl Recipient<ClearInteractable> for Selectable {
    fn receive(&mut self, _msg: &ClearInteractable) -> Fate {
        ::core::ui::UserInterface::id() << Remove::Interactable3d(self.id());
        Fate::Die
    }
}

use core::ui::Event3d;
use super::{ChangeIntent, Intent, IntentProgress, ContinuationMode};

const START_END_SNAP_DISTANCE: N = 10.0;
const SEGMENT_SNAP_DISTANCE: N = 5.0;
const CONTINUE_DISTANCE: N = 6.0;
const MIN_SELECTION_SIZE: N = 2.0;

impl Recipient<Event3d> for Selectable {
    fn receive(&mut self, msg: &Event3d) -> Fate {
        match *msg {
            Event3d::DragOngoing { from, to } => {
                if let (Some(selection_start), Some(selection_end)) =
                    (self.path.project(from.into_2d()), self.path.project(to.into_2d())) {
                    let mut start = selection_start.min(selection_end);
                    let mut end = selection_end.max(selection_start);
                    snap_start_end(&mut start, &mut end, &self.path);
                    CurrentPlan::id() <<
                    ChangeIntent(Intent::Select(self.stroke_ref, start, end),
                                 IntentProgress::Preview);
                }
                Fate::Live
            }
            Event3d::DragFinished { from, to } => {
                if let (Some(selection_start), Some(selection_end)) =
                    (self.path.project(from.into_2d()), self.path.project(to.into_2d())) {
                    let mut start = selection_start.min(selection_end);
                    let mut end = selection_end.max(selection_start);
                    if end < CONTINUE_DISTANCE {
                        CurrentPlan::id() <<
                        ChangeIntent(Intent::ContinueRoadAround(self.stroke_ref,
                                                                ContinuationMode::Prepend,
                                                                to.into_2d()),
                                     IntentProgress::Finished);
                    } else if start > self.path.length() - CONTINUE_DISTANCE {
                        CurrentPlan::id() <<
                        ChangeIntent(Intent::ContinueRoadAround(self.stroke_ref,
                                                                ContinuationMode::Append,
                                                                to.into_2d()),
                                     IntentProgress::Finished);
                    } else {
                        snap_start_end(&mut start, &mut end, &self.path);
                        start = start.min(end - MIN_SELECTION_SIZE).max(0.0);
                        end = end.max(start + MIN_SELECTION_SIZE).min(self.path.length());
                        CurrentPlan::id() <<
                        ChangeIntent(Intent::Select(self.stroke_ref, start, end),
                                     IntentProgress::Immediate);
                    }
                }
                Fate::Live
            }
            _ => Fate::Live,
        }
    }
}

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

pub fn setup() {
    Swarm::<Selectable>::register_default();
    Swarm::<Selectable>::handle::<CreateWith<Selectable, InitInteractable>>();
    Swarm::<Selectable>::handle::<ClearInteractable>();
    Swarm::<Selectable>::handle::<Event3d>();
}
