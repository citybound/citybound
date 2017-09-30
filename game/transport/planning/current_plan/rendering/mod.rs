use kay::World;
use compact::CDict;
use descartes::{N, Band, Path, FiniteCurve};
use monet::{Geometry, Instance, RendererID};
use stagemaster::geometry::band_to_geometry;
use super::{CurrentPlan, CurrentPlanID, SelectableStrokeRef};
use super::super::plan::{PlanDelta, BuiltStrokes, PlanResultDelta};
use super::super::lane_stroke::LaneStroke;

use monet::{Renderable, RenderableID, MSG_Renderable_setup_in_scene,
            MSG_Renderable_render_to_scene};

impl Renderable for CurrentPlan {
    fn setup_in_scene(&mut self, _renderer_id: RendererID, _scene_id: usize, _: &mut World) {}

    fn render_to_scene(
        &mut self,
        renderer_id: RendererID,
        scene_id: usize,
        _frame: usize,
        world: &mut World,
    ) {
        if self.preview_rendered_in.get(renderer_id).is_none() {
            let origin_machine = self.id._raw_id.machine;
            let preview = if self.preview.is_none() {
                self.preview_rendered_in = CDict::new();
                self.update_preview(world)
            } else {
                self.preview.as_ref().unwrap()
            };
            render_strokes(
                origin_machine,
                &preview.plan_delta,
                renderer_id,
                scene_id,
                world,
            );
        }
        if !self.interactables_valid {
            self.update_interactables(world);
        }
        if let Some(ref result_delta) = *self.preview_result_delta {
            // TODO: add something like prepare-render to monet to make sure
            // we have new state in time
            if self.preview_result_delta_rendered_in
                .get(renderer_id)
                .is_none()
            {
                self.preview_result_delta_rendered_in.insert(
                    renderer_id,
                    (),
                );

                render_trimmed_strokes(result_delta, renderer_id, scene_id, world);
                render_intersections(result_delta, renderer_id, scene_id, world);
                render_transfer_lanes(result_delta, renderer_id, scene_id, world);
            }
        }
        if let Some(ref built_strokes) = *self.built_strokes {
            render_selections(
                &self.preview.as_ref().unwrap().selections,
                &self.preview.as_ref().unwrap().plan_delta,
                built_strokes,
                renderer_id,
                scene_id,
                world,
            );
        }
    }
}

fn render_strokes(
    origin_machine: u8,
    delta: &PlanDelta,
    renderer_id: RendererID,
    scene_id: usize,
    world: &mut World,
) {
    let destroyed_strokes_geometry: Geometry = delta
        .strokes_to_destroy
        .pairs()
        .filter(|&(_, stroke)| stroke.nodes().len() > 1)
        .map(|(_, stroke)| {
            band_to_geometry(&Band::new(stroke.path().clone(), 5.0), 0.1)
        })
        .sum();
    renderer_id.update_individual(
        scene_id,
        5496 + u16::from(world.local_machine_id()) * 10_000,
        destroyed_strokes_geometry,
        Instance::with_color([1.0, 0.0, 0.0]),
        true,
        world,
    );
    let stroke_base_geometry: Geometry = delta
        .new_strokes
        .iter()
        .filter(|stroke| stroke.nodes().len() > 1)
        .map(|stroke| {
            band_to_geometry(&Band::new(stroke.path().clone(), 6.0), 0.1)
        })
        .sum();
    renderer_id.update_individual(
        scene_id,
        5498 + u16::from(world.local_machine_id()) * 10_000,
        stroke_base_geometry,
        Instance::with_color(if origin_machine == 0 {
            [0.2, 0.2, 1.0]
        } else {
            [1.0, 0.5, 0.2]
        }),
        true,
        world,
    );
    let stroke_geometry: Geometry = delta
        .new_strokes
        .iter()
        .filter(|stroke| stroke.nodes().len() > 1)
        .map(LaneStroke::preview_geometry)
        .sum();
    renderer_id.update_individual(
        scene_id,
        5499 + u16::from(world.local_machine_id()) * 10_000,
        stroke_geometry,
        Instance::with_color([0.6, 0.6, 0.6]),
        true,
        world,
    );
}

fn render_trimmed_strokes(
    result_delta: &PlanResultDelta,
    renderer_id: RendererID,
    scene_id: usize,
    world: &mut World,
) {
    let trimmed_stroke_geometry: Geometry = result_delta
        .trimmed_strokes
        .to_create
        .values()
        .filter(|stroke| stroke.nodes().len() > 1)
        .map(LaneStroke::preview_geometry)
        .sum();
    renderer_id.update_individual(
        scene_id,
        5500 + u16::from(world.local_machine_id()) * 10_000,
        trimmed_stroke_geometry,
        Instance::with_color([0.3, 0.3, 0.3]),
        true,
        world,
    );
}

fn render_intersections(
    result_delta: &PlanResultDelta,
    renderer_id: RendererID,
    scene_id: usize,
    world: &mut World,
) {
    // let intersections_geometry: Geometry = result_delta
    //     .intersections
    //     .to_create
    //     .values()
    //     .filter(|i| !i.shape.segments().is_empty())
    //     .map(|i| band_to_geometry(&Band::new(i.shape.clone(), 0.4), 0.5))
    //     .sum();
    // renderer_id.update_individual(
    //     scene_id,
    //     5501 + u16::from(world.local_machine_id()) * 10_000,
    //     intersections_geometry,
    //     Instance::with_color([0.0, 0.0, 1.0]),
    //     true,
    //     world,
    // );
    let connecting_strokes_geometry: Geometry = result_delta
        .intersections
        .to_create
        .values()
        .filter(|i| !i.strokes.is_empty())
        .map(|i| -> Geometry {
            i.strokes.iter().map(LaneStroke::preview_geometry).sum()
        })
        .sum();
    renderer_id.update_individual(
        scene_id,
        5502 + u16::from(world.local_machine_id()) * 10_000,
        connecting_strokes_geometry,
        Instance::with_color([0.5, 0.5, 0.5]),
        true,
        world,
    );
}

fn render_transfer_lanes(
    result_delta: &PlanResultDelta,
    renderer_id: RendererID,
    scene_id: usize,
    world: &mut World,
) {
    let transfer_strokes_geometry: Geometry = result_delta
        .transfer_strokes
        .to_create
        .values()
        .map(|lane_stroke| {
            band_to_geometry(&Band::new(lane_stroke.path().clone(), 0.3), 0.1)
        })
        .sum();
    renderer_id.update_individual(
        scene_id,
        5503 + u16::from(world.local_machine_id()) * 10_000,
        transfer_strokes_geometry,
        Instance::with_color([1.0, 1.0, 1.0]),
        true,
        world,
    );
}

fn render_selections(
    selections: &CDict<SelectableStrokeRef, (N, N)>,
    plan_delta: &PlanDelta,
    built_strokes: &BuiltStrokes,
    renderer_id: RendererID,
    scene_id: usize,
    world: &mut World,
) {
    let addable_geometry = selections
        .pairs()
        .filter_map(|(&selection_ref, &(start, end))| {
            let stroke = selection_ref.get_stroke(plan_delta, built_strokes);
            stroke.path().subsection(start, end).and_then(|subsection| {
                subsection.shift_orthogonally(5.0).map(
                    |shifted_subsection| {
                        band_to_geometry(&Band::new(shifted_subsection, 5.0), 0.1)
                    },
                )
            })
        })
        .sum();
    renderer_id.update_individual(
        scene_id,
        5497 + u16::from(world.local_machine_id()) * 10_000,
        addable_geometry,
        Instance::with_color([0.8, 0.8, 1.0]),
        true,
        world,
    );
    let selection_geometry = selections
        .pairs()
        .filter_map(|(&selection_ref, &(start, end))| {
            let stroke = selection_ref.get_stroke(plan_delta, built_strokes);
            stroke.path().subsection(start, end).map(|subsection| {
                band_to_geometry(&Band::new(subsection, 5.0), 0.1)
            })
        })
        .sum();
    renderer_id.update_individual(
        scene_id,
        5504 + u16::from(world.local_machine_id()) * 10_000,
        selection_geometry,
        Instance::with_color([0.0, 0.0, 1.0]),
        true,
        world,
    );
}

mod kay_auto;
pub use self::kay_auto::*;
