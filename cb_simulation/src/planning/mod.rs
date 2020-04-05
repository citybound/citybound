use kay::World;
use compact::CVec;
use transport::transport_planning::{RoadIntent, RoadPrototype};
use land_use::zone_planning::{ZoneIntent, BuildingIntent, LotPrototype};
use environment::vegetation::{PlantIntent, PlantPrototype};
use cb_planning::{PlanningLogic, PrototypeID, PlanningStepFn};
use cb_planning::plan_manager::{PlanManager, PlanManagerID};
use cb_planning::construction::{
    Construction, ConstructionID, PrototypeKind, GestureIntent, ConstructableID,
};

#[derive(Copy, Clone)]
pub struct CBPlanningLogic {}

impl PlanningLogic for CBPlanningLogic {
    type GestureIntent = CBGestureIntent;
    type PrototypeKind = CBPrototypeKind;

    fn planning_step_functions() -> &'static [PlanningStepFn<Self>] {
        &[
            ::transport::transport_planning::calculate_prototypes,
            ::land_use::zone_planning::calculate_prototypes,
            ::environment::vegetation::calculate_prototypes,
        ]
    }
}

pub type CBPlanManager = PlanManager<CBPlanningLogic>;
pub type CBPlanManagerID = PlanManagerID<CBPlanningLogic>;
pub type CBConstruction = Construction<CBPrototypeKind>;
pub type CBConstructionID = ConstructionID<CBPrototypeKind>;

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub enum CBGestureIntent {
    Road(RoadIntent),
    Zone(ZoneIntent),
    Building(BuildingIntent),
    Plant(PlantIntent),
}

impl GestureIntent for CBGestureIntent {}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub enum CBPrototypeKind {
    Road(RoadPrototype),
    Lot(LotPrototype),
    Plant(PlantPrototype),
}

impl PrototypeKind for CBPrototypeKind {
    fn construct(
        &self,
        prototype_id: PrototypeID,
        report_to: CBConstructionID,
        world: &mut World,
    ) -> CVec<ConstructableID<CBPrototypeKind>> {
        match self {
            CBPrototypeKind::Road(ref road_prototype) => road_prototype.construct(report_to, world),
            CBPrototypeKind::Lot(ref lot_prototype) => {
                lot_prototype.construct(prototype_id, report_to, world)
            }
            CBPrototypeKind::Plant(ref plant_prototype) => {
                plant_prototype.construct(prototype_id, report_to, world)
            }
        }
    }

    fn morphable_from(&self, other: &Self) -> bool {
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
