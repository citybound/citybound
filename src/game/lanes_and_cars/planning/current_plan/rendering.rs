use kay::{ID, Recipient, Fate, Actor};
use descartes::{Band, Path, FiniteCurve};
use monet::{Thing, Instance};
use ::core::geometry::band_to_thing;
use super::CurrentPlan;
use super::super::plan::{PlanDelta, PlanResultDelta};
use super::super::lane_stroke::LaneStroke;

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
                if self.preview.is_none() {
                    let preview = self.update_preview();
                    render_strokes(&preview.plan_delta, renderer_id, scene_id);
                }
                if let Some(ref result_delta) = self.preview_result_delta {
                    // TODO: add something like prepare-render to monet to make sure
                    // we have new state in time
                    if !self.preview_result_delta_rendered {
                        self.preview_result_delta_rendered = true;

                        render_trimmed_strokes(result_delta, renderer_id, scene_id);
                        render_intersections(result_delta, renderer_id, scene_id);
                        render_transfer_lanes(result_delta, renderer_id, scene_id);
                    }
                }
                Fate::Live
            }
        }
    }
}

fn render_strokes(delta: &PlanDelta, renderer_id: ID, scene_id: usize) {
    let stroke_base_thing: Thing = delta.new_strokes
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
    let stroke_thing: Thing = delta.new_strokes
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
}

fn render_trimmed_strokes(result_delta: &PlanResultDelta, renderer_id: ID, scene_id: usize) {
    let trimmed_stroke_thing: Thing = result_delta.trimmed_strokes
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
}

fn render_intersections(result_delta: &PlanResultDelta, renderer_id: ID, scene_id: usize) {
    let intersections_thing: Thing = result_delta.intersections
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
    let connecting_strokes_thing: Thing = result_delta.intersections
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
}

fn render_transfer_lanes(result_delta: &PlanResultDelta, renderer_id: ID, scene_id: usize) {
    let transfer_strokes_thing: Thing = result_delta.transfer_strokes
        .to_create
        .values()
        .map(|lane_stroke| band_to_thing(&Band::new(lane_stroke.path().clone(), 0.3), 0.1))
        .sum();
    renderer_id <<
    UpdateThing {
        scene_id: scene_id,
        thing_id: 5503,
        thing: transfer_strokes_thing,
        instance: Instance::with_color([1.0, 0.5, 0.0]),
        is_decal: true,
    };
}

pub fn setup() {
    CurrentPlan::handle::<SetupInScene>();
    CurrentPlan::handle::<RenderToScene>();
}