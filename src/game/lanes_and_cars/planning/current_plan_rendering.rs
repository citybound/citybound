use kay::{Recipient, Fate, ActorSystem};
use descartes::{Band, Path, FiniteCurve};
use monet::{Thing, Instance};
use ::core::geometry::band_to_thing;
use super::{CurrentPlan, LaneStroke, DrawingStatus, SelectableStrokeRef};

use monet::SetupInScene;

impl Recipient<SetupInScene> for CurrentPlan {
    fn receive(&mut self, _msg: &SetupInScene) -> Fate {
        Fate::Live
    }
}

use monet::RenderToScene;
use monet::UpdateThing;

impl Recipient<RenderToScene> for CurrentPlan {
    fn receive(&mut self, msg: &RenderToScene) -> Fate {match *msg{
        RenderToScene{renderer_id, scene_id} => {
            if self.ui_state.dirty {
                let thing : Thing = self.current_plan_result_delta.trimmed_strokes.to_create.values()
                    .filter(|stroke| stroke.nodes().len() > 1)
                    .map(LaneStroke::preview_thing)
                    .sum();
                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 500,
                    thing: thing,
                    instance: Instance::with_color([0.3, 0.3, 0.5])
                };
                let intersections_thing : Thing = self.current_plan_result_delta.intersections.to_create.values()
                    .filter(|i| i.shape.segments().len() > 0)
                    .map(|i| band_to_thing(&Band::new(i.shape.clone(), 0.4), 0.5))
                    .sum();
                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 501,
                    thing: intersections_thing,
                    instance: Instance::with_color([0.0, 0.0, 1.0])
                };
                let connecting_strokes_thing : Thing = self.current_plan_result_delta.intersections.to_create.values()
                    .filter(|i| !i.strokes.is_empty())
                    .map(|i| -> Thing {i.strokes.iter().map(LaneStroke::preview_thing).sum()})
                    .sum();
                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 502,
                    thing: connecting_strokes_thing,
                    instance: Instance::with_color([0.5, 0.5, 1.0])
                };
                let transfer_strokes_thing : Thing = self.current_plan_result_delta.transfer_strokes.to_create.values()
                    .map(|lane_stroke|
                        band_to_thing(&Band::new(lane_stroke.path().clone(), 0.3), 0.1))
                    .sum();
                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 503,
                    thing: transfer_strokes_thing,
                    instance: Instance::with_color([1.0, 0.5, 0.0])
                };
                self.ui_state.dirty = false;
            }
            if let DrawingStatus::WithSelection(stroke_ref, start, end) = self.ui_state.drawing_status {
                let stroke = match stroke_ref {
                    SelectableStrokeRef::New(stroke_idx) => &self.delta.new_strokes[stroke_idx],
                    SelectableStrokeRef::RemainingOld(old_stroke_ref) => self.current_remaining_old_strokes.mapping.get(old_stroke_ref).unwrap()
                };
                if let Some(subsection) = stroke.path().subsection(start, end) {
                    let selection_thing : Thing = band_to_thing(&Band::new(subsection, 2.5), 0.1);
                    renderer_id << UpdateThing{
                        scene_id: scene_id,
                        thing_id: 504,
                        thing: selection_thing,
                        instance: Instance::with_color([0.0, 0.0, 1.0])
                    };
                }
            } else {
                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 504,
                    thing: Thing::new(vec![], vec![]),
                    instance: Instance::with_color([0.0, 0.0, 1.0])
                };
            }
            Fate::Live
        }
    }}
}

pub fn setup(system: &mut ActorSystem) {
    system.add_inbox::<SetupInScene, CurrentPlan>();
    system.add_inbox::<RenderToScene, CurrentPlan>();
}