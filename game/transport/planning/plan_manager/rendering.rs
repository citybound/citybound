use kay::World;
use compact::CDict;
use descartes::{N, Band, FiniteCurve};
use monet::{Geometry, Instance, RendererID};
use stagemaster::geometry::band_to_geometry;
use super::super::road_plan::{RoadPlanDelta, RoadPlanResultDelta};
use super::super::plan_manager::MaterializedRoadView;
use super::super::lane_stroke::LaneStroke;
use super::SelectableStrokeRef;

pub fn render_strokes(
    origin_machine: u8,
    delta: &RoadPlanDelta,
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

pub fn render_trimmed_strokes(
    result_delta: &RoadPlanResultDelta,
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

pub fn render_intersections(
    result_delta: &RoadPlanResultDelta,
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

pub fn render_transfer_lanes(
    result_delta: &RoadPlanResultDelta,
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

pub fn render_selections(
    selections: &CDict<SelectableStrokeRef, (N, N)>,
    plan_delta: &RoadPlanDelta,
    materialized_view: &MaterializedRoadView,
    renderer_id: RendererID,
    scene_id: usize,
    world: &mut World,
) {
    let built_strokes = &materialized_view.built_strokes;
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
