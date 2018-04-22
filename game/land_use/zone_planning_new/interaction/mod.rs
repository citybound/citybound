use kay::World;
use descartes::{Band, SimpleShape};
use stagemaster::geometry::band_to_geometry;
use monet::{RendererID, Geometry, Instance};
use planning_new::{PlanResult, Prototype};
use style::colors;

use super::LotPrototype;

pub fn render_preview(
    result_preview: &PlanResult,
    renderer_id: RendererID,
    scene_id: usize,
    frame: usize,
    world: &mut World,
) {
    let mut lot_geometry = Geometry::empty();
    let mut lot_outline_geometry = Geometry::empty();

    for prototype in &result_preview.prototypes {
        if let Prototype::Lot(LotPrototype { ref shape, .. }) = *prototype {
            lot_geometry += Geometry::from_shape(shape);

            lot_outline_geometry += band_to_geometry(&Band::new(shape.outline().clone(), 2.0), 0.1);
        }
    }

    renderer_id.update_individual(
        scene_id,
        18_003,
        lot_geometry,
        Instance::with_color(colors::RESIDENTIAL),
        true,
        world,
    );

    renderer_id.update_individual(
        scene_id,
        18_004,
        lot_outline_geometry,
        Instance::with_color(colors::CONTROL_POINT_SELECTED),
        true,
        world,
    );
}