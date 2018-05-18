use kay::World;
use compact::CVec;
use descartes::{Band, SimpleShape};
use stagemaster::geometry::band_to_geometry;
use monet::{RendererID, Geometry, Instance};
use planning::{PlanResult, Prototype};
use construction::Action;
use style::colors;
use render_layers::RenderLayers;

use super::{LotPrototype, Lot, LotOccupancy};

pub fn render_preview(
    result_preview: &PlanResult,
    _maybe_action_preview: &Option<CVec<CVec<Action>>>,
    renderer_id: RendererID,
    _frame: usize,
    world: &mut World,
) {
    let mut lot_geometry = Geometry::empty();
    let mut lot_outline_geometry = Geometry::empty();

    for prototype in result_preview.prototypes.values() {
        if let Prototype::Lot(LotPrototype { lot: Lot { ref shape, .. }, occupancy, .. }) =
            *prototype
        {
            if occupancy == LotOccupancy::Vacant {
                lot_outline_geometry +=
                    band_to_geometry(&Band::new(shape.outline().clone(), 2.0), 0.1);
            } else {
                lot_geometry += band_to_geometry(&Band::new(shape.outline().clone(), 1.0), 0.1);
                //Geometry::from_shape(shape);
            }

        }
    }

    renderer_id.update_individual(
        RenderLayers::PlanningLot as u32,
        lot_geometry,
        Instance::with_color(colors::CONTROL_POINT),
        true,
        world,
    );

    renderer_id.update_individual(
        RenderLayers::PlanningLotOutline as u32,
        lot_outline_geometry,
        Instance::with_color(colors::CONTROL_POINT_SELECTED),
        true,
        world,
    );
}
