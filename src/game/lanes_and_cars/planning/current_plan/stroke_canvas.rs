use kay::{Recipient, Actor, Fate};
use compact::CVec;
use descartes::{P2, Into2d, RoughlyComparable};
use core::geometry::AnyShape;
use super::CurrentPlan;

#[derive(Compact, Clone, Default)]
pub struct StrokeCanvas {
    points: CVec<P2>,
}

impl Actor for StrokeCanvas {}

#[derive(Copy, Clone)]
pub enum StrokeState {
    Preview,
    Intermediate,
    Finished,
}

#[derive(Compact, Clone)]
pub struct Stroke(pub CVec<P2>, pub StrokeState);

use core::ui::Event3d;

const FINISH_STROKE_TOLERANCE: f32 = 5.0;

impl Recipient<Event3d> for StrokeCanvas {
    fn receive(&mut self, msg: &Event3d) -> Fate {
        match *msg {
            Event3d::HoverStarted { at } |
            Event3d::HoverOngoing { at } => {
                if !self.points.is_empty() {
                    let mut preview_points = self.points.clone();
                    preview_points.push(at.into_2d());
                    CurrentPlan::id() << Stroke(preview_points, StrokeState::Preview);
                }
                Fate::Live
            }
            Event3d::DragStarted { at } => {
                let new_point = at.into_2d();
                let maybe_last_point = self.points.last().cloned();

                let finished = if let Some(last_point) = maybe_last_point {
                    if new_point.is_roughly_within(last_point, FINISH_STROKE_TOLERANCE) {
                        CurrentPlan::id() << Stroke(self.points.clone(), StrokeState::Finished);
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
                        CurrentPlan::id() << Stroke(self.points.clone(), StrokeState::Intermediate);
                    }
                }
                Fate::Live
            }
            _ => Fate::Live,
        }
    }
}

#[derive(Copy, Clone)]
pub struct AddToUI;

use core::ui::{UserInterface, Add, Focus};

impl Recipient<AddToUI> for StrokeCanvas {
    fn receive(&mut self, _msg: &AddToUI) -> Fate {
        UserInterface::id() << Add::Interactable3d(StrokeCanvas::id(), AnyShape::Everywhere, 0);
        UserInterface::id() << Focus(StrokeCanvas::id());
        Fate::Live
    }
}

pub fn setup() {
    StrokeCanvas::register_default();
    StrokeCanvas::handle::<Event3d>();
    StrokeCanvas::handle::<AddToUI>();
    StrokeCanvas::id() << AddToUI;
}
