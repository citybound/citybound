use kay::{World, Fate, ActorSystem};
use compact::CVec;
use stagemaster::geometry::CShape;
use land_use::zone_planning_new::{LandUse, Lot, BuildingIntent};
use land_use::buildings::BuildingStyle;
use economy::immigration_and_development::DevelopmentManagerID;

use construction::{ConstructionID, Constructable, ConstructableID};
use planning_new::Prototype;

#[derive(Compact, Clone)]
pub struct VacantLot {
    pub id: VacantLotID,
    pub lot: Lot,
}

impl VacantLot {
    pub fn spawn(id: VacantLotID, lot: &Lot, world: &mut World) -> VacantLot {
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
    fn morph(&mut self, _: &Prototype, report_to: ConstructionID, world: &mut World) {
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