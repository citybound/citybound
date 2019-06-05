use transport::transport_planning::{RoadIntent, RoadPrototype};
use land_use::zone_planning::{ZoneIntent, BuildingIntent, LotPrototype};
use environment::vegetation::{PlantIntent, PlantPrototype};

#[derive(Copy, Clone)]
pub struct CBPlanning {}

pub type PlanManager = ();

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub enum CBGestureIntent {
    Road(RoadIntent),
    Zone(ZoneIntent),
    Building(BuildingIntent),
    Plant(PlantIntent),
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub enum CBPrototypeKind {
    Road(RoadPrototype),
    Lot(LotPrototype),
    Plant(PlantPrototype),
}

impl PrototypeKind for CBPrototypeKind {
    fn construct(&self, report_to: ConstructionID, world: &mut World) -> CVec<ConstructableID> {
        match self {
            CBPrototypeKind::Road(ref road_prototype) => road_prototype.construct(report_to, world),
            CBPrototypeKind::Lot(ref lot_prototype) => {
                lot_prototype.construct(self.id, report_to, world)
            }
            CBPrototypeKind::Plant(ref plant_prototype) => {
                plant_prototype.construct(self.id, report_to, world)
            }
        }
    }

    pub fn morphable_from(&self, other: &Self) -> bool {
        match (&self, &other) {
            (&CBPrototypeKind::Road(ref self_road), &CBPrototypeKind::Road(ref other_road)) => {
                self_road.morphable_from(other_road)
            }
            (&CBPrototypeKind::Lot(ref self_lot), &CBPrototypeKind::Lot(ref other_lot)) => {
                self_lot.morphable_from(other_lot)
            }
            (&CBPrototypeKind::Plant(ref self_plant), &CBPrototypeKind::Plant(ref other_plant)) => {
                self_plant.morphable_from(other_plant)
            }
            _ => false,
        }
    }
}