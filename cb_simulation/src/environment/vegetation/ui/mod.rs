use kay::World;
use super::{Plant, PlantID, PlantPrototype};

impl Plant {
    pub fn get_render_info(&mut self, requester: VegetationUIID, world: &mut World) {
        requester.on_plant_spawned(self.id, self.proto, world);
    }
}

pub trait VegetationUI {
    fn on_plant_spawned(&mut self, id: PlantID, proto: &PlantPrototype, world: &mut World);
    fn on_plant_destroyed(&mut self, id: PlantID, world: &mut World);
}

mod kay_auto;
pub use self::kay_auto::*;
