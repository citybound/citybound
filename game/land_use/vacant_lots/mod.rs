use kay::{World, Fate, ActorSystem};
use land_use::zone_planning::{Lot, BuildingIntent};
use land_use::buildings::BuildingStyle;
use economy::immigration_and_development::DevelopmentManagerID;

use construction::{ConstructionID, Constructable, ConstructableID};
use planning::Prototype;

#[derive(Compact, Clone)]
pub struct VacantLot {
    pub id: VacantLotID,
    pub lot: Lot,
}

impl VacantLot {
    pub fn spawn(id: VacantLotID, lot: &Lot, _world: &mut World) -> VacantLot {
        VacantLot { id, lot: lot.clone() }
    }

    pub fn suggest_lot(
        &mut self,
        building_style: BuildingStyle,
        requester: DevelopmentManagerID,
        world: &mut World,
    ) {
        // TODO: actually implement
        requester.on_suggested_lot(
            BuildingIntent { lot: self.lot.clone(), building_style },
            world,
        )
    }
}

impl Constructable for VacantLot {
    fn morph(&mut self, _: &Prototype, _report_to: ConstructionID, _world: &mut World) {
        unreachable!()
    }

    fn destruct(&mut self, report_to: ConstructionID, world: &mut World) -> Fate {
        report_to.action_done(self.id.into(), world);
        Fate::Die
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<VacantLot>();

    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
