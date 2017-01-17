use kay::{Recipient, Fate, ActorSystem, Individual};
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
    fn receive(&mut self, msg: &RenderToScene) -> Fate {
        match *msg {
            RenderToScene { renderer_id, scene_id } => {
                if self.preview.ui_state.dirty {
                    let changed_old_stroke_thing: Thing = self.preview
                        .delta
                        .strokes_to_destroy
                        .values()
                        .filter(|stroke| stroke.nodes().len() > 1)
                        .map(LaneStroke::preview_thing)
                        .sum();
                    renderer_id <<
                    UpdateThing {
                        scene_id: scene_id,
                        thing_id: 5497,
                        thing: changed_old_stroke_thing,
                        instance: Instance::with_color([1.0, 0.4, 0.4]),
                        is_decal: true,
                    };
                    let stroke_base_thing: Thing = self.preview
                        .delta
                        .new_strokes
                        .iter()
                        .filter(|stroke| stroke.nodes().len() > 1)
                        .map(|stroke| band_to_thing(&Band::new(stroke.path().clone(), 6.0), 0.1))
                        .sum();
                    renderer_id <<
                    UpdateThing {
                        scene_id: scene_id,
                        thing_id: 5498,
                        thing: stroke_base_thing,
                        instance: Instance::with_color([1.0, 1.0, 1.0]),
                        is_decal: true,
                    };
                    let stroke_thing: Thing = self.preview
                        .delta
                        .new_strokes
                        .iter()
                        .filter(|stroke| stroke.nodes().len() > 1)
                        .map(LaneStroke::preview_thing)
                        .sum();
                    renderer_id <<
                    UpdateThing {
                        scene_id: scene_id,
                        thing_id: 5499,
                        thing: stroke_thing,
                        instance: Instance::with_color([0.6, 0.6, 0.6]),
                        is_decal: true,
                    };
                    let trimmed_stroke_thing: Thing = self.preview
                        .current_plan_result_delta
                        .trimmed_strokes
                        .to_create
                        .values()
                        .filter(|stroke| stroke.nodes().len() > 1)
                        .map(LaneStroke::preview_thing)
                        .sum();
                    renderer_id <<
                    UpdateThing {
                        scene_id: scene_id,
                        thing_id: 5500,
                        thing: trimmed_stroke_thing,
                        instance: Instance::with_color([0.3, 0.3, 0.5]),
                        is_decal: true,
                    };
                    let intersections_thing: Thing = self.preview
                        .current_plan_result_delta
                        .intersections
                        .to_create
                        .values()
                        .filter(|i| i.shape.segments().len() > 0)
                        .map(|i| band_to_thing(&Band::new(i.shape.clone(), 0.4), 0.5))
                        .sum();
                    renderer_id <<
                    UpdateThing {
                        scene_id: scene_id,
                        thing_id: 5501,
                        thing: intersections_thing,
                        instance: Instance::with_color([0.0, 0.0, 1.0]),
                        is_decal: true,
                    };
                    let connecting_strokes_thing: Thing = self.preview
                        .current_plan_result_delta
                        .intersections
                        .to_create
                        .values()
                        .filter(|i| !i.strokes.is_empty())
                        .map(|i| -> Thing { i.strokes.iter().map(LaneStroke::preview_thing).sum() })
                        .sum();
                    renderer_id <<
                    UpdateThing {
                        scene_id: scene_id,
                        thing_id: 5502,
                        thing: connecting_strokes_thing,
                        instance: Instance::with_color([0.5, 0.5, 1.0]),
                        is_decal: true,
                    };
                    let transfer_strokes_thing: Thing = self.preview
                        .current_plan_result_delta
                        .transfer_strokes
                        .to_create
                        .values()
                        .map(|lane_stroke| {
                            band_to_thing(&Band::new(lane_stroke.path().clone(), 0.3), 0.1)
                        })
                        .sum();
                    renderer_id <<
                    UpdateThing {
                        scene_id: scene_id,
                        thing_id: 5503,
                        thing: transfer_strokes_thing,
                        instance: Instance::with_color([1.0, 0.5, 0.0]),
                        is_decal: true,
                    };
                    self.preview.ui_state.dirty = false;
                }
                if let DrawingStatus::WithSelections(ref selections, _) =
                    self.preview.ui_state.drawing_status {
                    let selection_thing = selections.pairs()
                        .filter_map(|(&selection_ref, &(start, end))| {
                            let stroke = match selection_ref {
                                SelectableStrokeRef::New(stroke_idx) => {
                                    &self.preview.delta.new_strokes[stroke_idx]
                                }
                                SelectableStrokeRef::RemainingOld(old_stroke_ref) => {
                                    self.preview
                                        .current_remaining_old_strokes
                                        .mapping
                                        .get(old_stroke_ref)
                                        .unwrap()
                                }
                            };
                            stroke.path()
                                .subsection(start, end)
                                .map(|subsection| band_to_thing(&Band::new(subsection, 5.0), 0.1))
                        })
                        .sum();
                    renderer_id <<
                    UpdateThing {
                        scene_id: scene_id,
                        thing_id: 5504,
                        thing: selection_thing,
                        instance: Instance::with_color([0.0, 0.0, 1.0]),
                        is_decal: true,
                    };
                } else {
                    renderer_id <<
                    UpdateThing {
                        scene_id: scene_id,
                        thing_id: 5504,
                        thing: Thing::new(vec![], vec![]),
                        instance: Instance::with_color([0.0, 0.0, 1.0]),
                        is_decal: true,
                    };
                }
                Fate::Live
            }
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    CurrentPlan::handle::<SetupInScene>();
    CurrentPlan::handle::<RenderToScene>();
}
