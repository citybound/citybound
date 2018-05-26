use kay::World;
use compact::CVec;
use descartes::PointContainer;
use land_use::zone_planning::{LotPrototype, LotOccupancy};
use land_use::vacant_lots::VacantLotID;
use land_use::buildings::BuildingID;
use construction::{ConstructionID, ConstructableID};

impl LotPrototype {
    pub fn construct(&self, report_to: ConstructionID, world: &mut World) -> CVec<ConstructableID> {
        let id = match self.occupancy {
            LotOccupancy::Vacant => {
                VacantLotID::spawn(self.lot.clone(), self.based_on, world).into()
            }
            LotOccupancy::Occupied(building_style) => {
                BuildingID::spawn(building_style, self.lot.clone(), world).into()
            }
        };
        report_to.action_done(id, world);
        vec![id].into()
    }

    pub fn morphable_from(&self, other: &LotPrototype) -> bool {
        // TODO: improve this
        (self.occupancy != LotOccupancy::Vacant) && (other.occupancy != LotOccupancy::Vacant) &&
            other.lot.area.contains(self.lot.center_point())
    }
}
