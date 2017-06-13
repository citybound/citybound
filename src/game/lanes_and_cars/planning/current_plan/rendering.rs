use kay::{Fate, ActorSystem, World};
use compact::CDict;
use descartes::{N, Band, Path, FiniteCurve};
use monet::{Thing, Instance, RendererID};
use stagemaster::geometry::band_to_thing;
use super::{CurrentPlan, SelectableStrokeRef};
use super::super::plan::{PlanDelta, BuiltStrokes, PlanResultDelta};
use super::super::lane_stroke::LaneStroke;

use monet::SetupInScene;
use monet::RenderToScene;

pub fn setup(system: &mut ActorSystem) {
    system.extend::<CurrentPlan, _>(|mut the_cp| {
        the_cp.on(|_: &SetupInScene, _, _| Fate::Live);

        the_cp.on(|&RenderToScene { renderer_id, scene_id }, plan, world| {
            if plan.preview.is_none() {
                let preview = plan.update_preview(world);
                render_strokes(&preview.plan_delta, renderer_id, scene_id, world);

            }
            if !plan.interactables_valid {
                plan.update_interactables(world);
            }
            if let Some(ref result_delta) = plan.preview_result_delta {
                // TODO: add something like prepare-render to monet to make sure
                // we have new state in time
                if !plan.preview_result_delta_rendered {
                    plan.preview_result_delta_rendered = true;

                    render_trimmed_strokes(result_delta, renderer_id, scene_id, world);
                    render_intersections(result_delta, renderer_id, scene_id, world);
                    render_transfer_lanes(result_delta, renderer_id, scene_id, world);
                }
            }
            if let Some(ref built_strokes) = plan.built_strokes {
                render_selections(&plan.preview.as_ref().unwrap().selections,
                                  &plan.preview.as_ref().unwrap().plan_delta,
                                  built_strokes,
                                  renderer_id,
                                  scene_id,
                                  world);
            }

            Fate::Live
        });
    });
}

fn render_strokes(delta: &PlanDelta, renderer_id: RendererID, scene_id: usize, world: &mut World) {
    let destroyed_strokes_thing: Thing = delta
        .strokes_to_destroy
        .pairs()
        .filter(|&(_, stroke)| stroke.nodes().len() > 1)
        .map(|(_, stroke)| band_to_thing(&Band::new(stroke.path().clone(), 5.0), 0.1))
        .sum();
    renderer_id.update_thing(scene_id,
                             5496,
                             destroyed_strokes_thing,
                             Instance::with_color([1.0, 0.0, 0.0]),
                             true,
                             world);
    let stroke_base_thing: Thing = delta
        .new_strokes
        .iter()
        .filter(|stroke| stroke.nodes().len() > 1)
        .map(|stroke| band_to_thing(&Band::new(stroke.path().clone(), 6.0), 0.1))
        .sum();
    renderer_id.update_thing(scene_id,
                             5498,
                             stroke_base_thing,
                             Instance::with_color([1.0, 1.0, 1.0]),
                             true,
                             world);
    let stroke_thing: Thing = delta
        .new_strokes
        .iter()
        .filter(|stroke| stroke.nodes().len() > 1)
        .map(LaneStroke::preview_thing)
        .sum();
    renderer_id.update_thing(scene_id,
                             5499,
                             stroke_thing,
                             Instance::with_color([0.6, 0.6, 0.6]),
                             true,
                             world);
}

fn render_trimmed_strokes(result_delta: &PlanResultDelta,
                          renderer_id: RendererID,
                          scene_id: usize,
                          world: &mut World) {
    let trimmed_stroke_thing: Thing = result_delta
        .trimmed_strokes
        .to_create
        .values()
        .filter(|stroke| stroke.nodes().len() > 1)
        .map(LaneStroke::preview_thing)
        .sum();
    renderer_id.update_thing(scene_id,
                             5500,
                             trimmed_stroke_thing,
                             Instance::with_color([0.3, 0.3, 0.5]),
                             true,
                             world);
}

fn render_intersections(result_delta: &PlanResultDelta,
                        renderer_id: RendererID,
                        scene_id: usize,
                        world: &mut World) {
    let intersections_thing: Thing = result_delta
        .intersections
        .to_create
        .values()
        .filter(|i| !i.shape.segments().is_empty())
        .map(|i| band_to_thing(&Band::new(i.shape.clone(), 0.4), 0.5))
        .sum();
    renderer_id.update_thing(scene_id,
                             5501,
                             intersections_thing,
                             Instance::with_color([0.0, 0.0, 1.0]),
                             true,
                             world);
    let connecting_strokes_thing: Thing = result_delta
        .intersections
        .to_create
        .values()
        .filter(|i| !i.strokes.is_empty())
        .map(|i| -> Thing { i.strokes.iter().map(LaneStroke::preview_thing).sum() })
        .sum();
    renderer_id.update_thing(scene_id,
                             5502,
                             connecting_strokes_thing,
                             Instance::with_color([0.5, 0.5, 1.0]),
                             true,
                             world);
}

fn render_transfer_lanes(result_delta: &PlanResultDelta,
                         renderer_id: RendererID,
                         scene_id: usize,
                         world: &mut World) {
    let transfer_strokes_thing: Thing = result_delta
        .transfer_strokes
        .to_create
        .values()
        .map(|lane_stroke| band_to_thing(&Band::new(lane_stroke.path().clone(), 0.3), 0.1))
        .sum();
    renderer_id.update_thing(scene_id,
                             5503,
                             transfer_strokes_thing,
                             Instance::with_color([1.0, 0.5, 0.0]),
                             true,
                             world);
}

fn render_selections(selections: &CDict<SelectableStrokeRef, (N, N)>,
                     plan_delta: &PlanDelta,
                     built_strokes: &BuiltStrokes,
                     renderer_id: RendererID,
                     scene_id: usize,
                     world: &mut World) {
    let addable_thing = selections
        .pairs()
        .filter_map(|(&selection_ref, &(start, end))| {
            let stroke = selection_ref.get_stroke(plan_delta, built_strokes);
            stroke.path().subsection(start, end).and_then(|subsection| {
                subsection
                    .shift_orthogonally(5.0)
                    .map(|shifted_subsection| {
                        band_to_thing(&Band::new(shifted_subsection, 5.0), 0.1)
                    })
            })
        })
        .sum();
    renderer_id.update_thing(scene_id,
                             5497,
                             addable_thing,
                             Instance::with_color([0.8, 0.8, 1.0]),
                             true,
                             world);
    let selection_thing = selections
        .pairs()
        .filter_map(|(&selection_ref, &(start, end))| {
            let stroke = selection_ref.get_stroke(plan_delta, built_strokes);
            stroke
                .path()
                .subsection(start, end)
                .map(|subsection| band_to_thing(&Band::new(subsection, 5.0), 0.1))
        })
        .sum();
    renderer_id.update_thing(scene_id,
                             5504,
                             selection_thing,
                             Instance::with_color([0.0, 0.0, 1.0]),
                             true,
                             world);
}
