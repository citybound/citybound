use kay::World;
use compact::CVec;
use descartes::Band;
use monet::{RendererID, Mesh, Instance};
use planning::{PlanResult, PrototypeKind};
use construction::Action;
use style::colors;
use style::dimensions::LOT_OUTLINE_WIDTH;
use render_layers::RenderLayers;

use super::{LotPrototype, Lot, LotOccupancy, LandUse};

pub fn render_preview(
    result_preview: &PlanResult,
    _maybe_action_preview: &Option<CVec<CVec<Action>>>,
    renderer_id: RendererID,
    _frame: usize,
    world: &mut World,
) {
    let mut lot_residential_mesh = Mesh::empty();
    let mut lot_occupied_outline_mesh = Mesh::empty();
    let mut lot_vacant_outline_mesh = Mesh::empty();

    for prototype in result_preview.prototypes.values() {
        if let PrototypeKind::Lot(LotPrototype {
            lot:
                Lot {
                    ref area,
                    ref land_uses,
                    ..
                },
            occupancy,
            ..
        }) = prototype.kind
        {
            if occupancy == LotOccupancy::Vacant {
                for primitive in &area.primitives {
                    lot_vacant_outline_mesh += Mesh::from_band(
                        &Band::new(primitive.boundary.path().clone(), LOT_OUTLINE_WIDTH),
                        0.1,
                    );
                }
            } else {
                for primitive in &area.primitives {
                    lot_occupied_outline_mesh += Mesh::from_band(
                        &Band::new(primitive.boundary.path().clone(), LOT_OUTLINE_WIDTH),
                        0.1,
                    );
                }
            }

            for land_use in land_uses {
                if *land_use == LandUse::Residential {
                    lot_residential_mesh += Mesh::from_area(area);
                }
            }
        }
    }

    renderer_id.update_individual(
        RenderLayers::PlanningLotOccupiedOutline as u32,
        lot_occupied_outline_mesh,
        Instance::with_color(colors::LOT_OCCUPIED),
        true,
        world,
    );

    renderer_id.update_individual(
        RenderLayers::PlanningLotVacantOutline as u32,
        lot_vacant_outline_mesh,
        Instance::with_color(colors::LOT_VACANT),
        true,
        world,
    );

    renderer_id.update_individual(
        RenderLayers::PlanningLotResidentialArea as u32,
        lot_residential_mesh,
        Instance::with_color(colors::RESIDENTIAL),
        true,
        world,
    );
}
