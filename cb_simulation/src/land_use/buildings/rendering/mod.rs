use kay::{ World, TypedID};
use super::{Building, Lot, BuildingID, BuildingStyle};
use super::super::ui::{LandUseUIID};

impl Building {
    pub fn get_render_info(&mut self, ui: LandUseUIID, world: &mut World) {
        ui.on_building_constructed(self.id, self.lot.clone(), self.style, world)
    }
}

pub fn on_add(id: BuildingID, lot: &Lot, building_type: BuildingStyle, world: &mut World) {
    LandUseUIID::global_broadcast(world).on_building_constructed(
        id,
        lot.clone(),
        building_type,
        world,
    );
}

pub fn on_destroy(building_id: BuildingID, world: &mut World) {
    LandUseUIID::global_broadcast(world).on_building_destructed(building_id, world);
}

mod kay_auto;
pub use self::kay_auto::*;
