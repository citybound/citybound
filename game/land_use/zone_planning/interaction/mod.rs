use kay::World;
use compact::CVec;
use descartes::{Band, SimpleShape};
use stagemaster::geometry::band_to_geometry;
use monet::{RendererID, Geometry, Instance};
use planning::{PlanResult, Prototype};
use construction::Action;
use style::colors;

use super::{LotPrototype, Lot, LotOccupancy};

pub fn render_preview(
    result_preview: &PlanResult,
    _maybe_action_preview: &Option<CVec<CVec<Action>>>,
    renderer_id: RendererID,
    scene_id: usize,
    _frame: usize,
    world: &mut World,
) {
    let mut lot_geometry = Geometry::empty();
    let mut lot_outline_geometry = Geometry::empty();

    for prototype in result_preview.prototypes.values() {
        if let Prototype::Lot(LotPrototype { lot: Lot { ref shape, .. }, occupancy }) = *prototype {
            if occupancy == LotOccupancy::Vacant {
                lot_outline_geometry +=
                    band_to_geometry(&Band::new(shape.outline().clone(), 2.0), 0.1);
            } else {
                lot_geometry += Geometry::from_shape(shape);
            }

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
