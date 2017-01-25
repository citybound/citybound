use kay::{ID, Recipient, Actor, Fate};
use kay::swarm::{Swarm, SubActor, CreateWith};
use descartes::{N, Band, Curve, Into2d, FiniteCurve, Path, RoughlyComparable};
use ::core::geometry::{CPath, AnyShape};

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
                            1);
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

#[derive(Copy, Clone)]
pub enum SelectionState {
    Ongoing,
    Finished,
}

#[derive(Copy, Clone)]
pub struct Select(pub SelectableStrokeRef, pub N, pub N, pub SelectionState);

use core::ui::Event3d;

const START_END_SNAP_DISTANCE: N = 10.0;
const SEGMENT_SNAP_DISTANCE: N = 5.0;

impl Recipient<Event3d> for Selectable {
    fn receive(&mut self, msg: &Event3d) -> Fate {
        match *msg {
            Event3d::DragOngoing { from, to } => {
                println!("drag ongoing");
                if let (Some(selection_start), Some(selection_end)) =
                    (self.path.project(from.into_2d()), self.path.project(to.into_2d())) {
                    println!("{:?} {:?}", selection_start, selection_end);
                    let mut start = selection_start.min(selection_end);
                    let mut end = selection_end.max(selection_start);
                    if start < START_END_SNAP_DISTANCE {
                        start = 0.0
                    }
                    if end > self.path.length() - START_END_SNAP_DISTANCE {
                        end = self.path.length()
                    }
                    let mut offset = 0.0;
                    for segment in self.path.segments() {
                        let next_offset = offset + segment.length();
                        if start.is_roughly_within(offset, SEGMENT_SNAP_DISTANCE) {
                            start = offset
                        }
                        if end.is_roughly_within(next_offset, SEGMENT_SNAP_DISTANCE) {
                            end = next_offset
                        }
                        offset = next_offset;
                    }
                    CurrentPlan::id() <<
                    Select(self.stroke_ref, start, end, SelectionState::Ongoing);
                }
                Fate::Live
            }
            _ => Fate::Live,
        }
    }
}

pub fn setup() {
    Swarm::<Selectable>::register_default();
    Swarm::<Selectable>::handle::<CreateWith<Selectable, InitInteractable>>();
    Swarm::<Selectable>::handle::<ClearInteractable>();
    Swarm::<Selectable>::handle::<Event3d>();
}