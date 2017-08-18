use kay::{ActorSystem, Fate};
use compact::CVec;
use descartes::{P2, Into2d, RoughlyComparable};
use stagemaster::geometry::AnyShape;
use super::CurrentPlan;

#[derive(Compact, Clone, Default)]
pub struct StrokeCanvas {
    points: CVec<P2>,
}

#[derive(Copy, Clone)]
pub enum StrokeState {
    Preview,
    Intermediate,
    Finished,
}

#[derive(Compact, Clone)]
pub struct Stroke(pub CVec<P2>, pub StrokeState);

use stagemaster::Event3d;

const FINISH_STROKE_TOLERANCE: f32 = 5.0;

pub fn setup(system: &mut ActorSystem) {
    system.add(StrokeCanvas::default(), |mut the_canvas| {
        let cp_id = the_canvas.world().id::<CurrentPlan>();
        let ui_id = the_canvas.world().id::<UserInterface>();
        let canvas_id = the_canvas.world().id::<StrokeCanvas>();

        the_canvas.on(move |event, canvas, world| {
            match *event {
                Event3d::HoverStarted { at, .. } |
                Event3d::HoverOngoing { at, .. } => {
                    let mut preview_points = canvas.points.clone();
                    preview_points.push(at.into_2d());
                    world.send(cp_id, Stroke(preview_points, StrokeState::Preview));
                }
                Event3d::DragStarted { at, .. } => {
                    let new_point = at.into_2d();
                    let maybe_last_point = canvas.points.last().cloned();

                    let finished = if let Some(last_point) = maybe_last_point {
                        if new_point.is_roughly_within(last_point, FINISH_STROKE_TOLERANCE) {
                            world.send(cp_id, Stroke(canvas.points.clone(), StrokeState::Finished));
                            canvas.points.clear();
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if !finished {
                        canvas.points.push(new_point);
                        if canvas.points.len() > 1 {
                            world.send(
                                cp_id,
                                Stroke(canvas.points.clone(), StrokeState::Intermediate),
                            );
                        }
                    }
                }
                _ => {}
            };
            Fate::Live
        });

        the_canvas.on(|&SetPoints(ref points), canvas, _| {
            canvas.points = points.clone();
            Fate::Live
        });

        the_canvas.on(move |_: &InitInteractable, _, world| {
            world.send(ui_id, AddInteractable(canvas_id, AnyShape::Everywhere, 1));
            Fate::Live
        });

        the_canvas.world().send(canvas_id, InitInteractable);

    });
}

#[derive(Compact, Clone)]
pub struct SetPoints(pub CVec<P2>);

use super::InitInteractable;
use stagemaster::{UserInterface, AddInteractable};
