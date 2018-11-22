use kay::{World, ActorSystem, TypedID};
use compact::COption;
use land_use::buildings::{UnitType, BuildingID, UnitIdx};
use time::{Sleeper, SleeperID, Instant, TimeID, Duration};
use util::random::{seed, Rng};

use economy::households::household_kinds;
use self::household_kinds::family::FamilyID;
use self::household_kinds::grocery_shop::GroceryShopID;
use self::household_kinds::cow_farm::CowFarmID;
use self::household_kinds::grain_farm::GrainFarmID;
use self::household_kinds::vegetable_farm::VegetableFarmID;
use self::household_kinds::mill::MillID;
use self::household_kinds::bakery::BakeryID;
use self::household_kinds::neighboring_town_trade::NeighboringTownTradeID;
use land_use::buildings::BuildingStyle;
use land_use::vacant_lots::VacantLotID;
use land_use::zone_planning::BuildingIntent;
use planning::{PlanManagerID, Proposal, PrototypeID, Plan, GestureID, Gesture, GestureIntent};

// TODO: somehow get rid of this horrible duplication by having something like
// a pointer to an abstract Household trait...

#[derive(Copy, Clone, Debug)]
pub enum HouseholdTypeToSpawn {
    Family,
    GroceryShop,
    GrainFarm,
    CowFarm,
    VegetableFarm,
    Mill,
    Bakery,
    NeighboringTownTrade,
}

pub fn unit_type_for(household_type: HouseholdTypeToSpawn) -> UnitType {
    match household_type {
        HouseholdTypeToSpawn::Family => UnitType::Dwelling,
        HouseholdTypeToSpawn::GroceryShop => UnitType::Retail,
        HouseholdTypeToSpawn::GrainFarm
        | HouseholdTypeToSpawn::CowFarm
        | HouseholdTypeToSpawn::VegetableFarm => UnitType::Agriculture,
        HouseholdTypeToSpawn::Mill => UnitType::Mill,
        HouseholdTypeToSpawn::Bakery => UnitType::Bakery,
        HouseholdTypeToSpawn::NeighboringTownTrade => UnitType::NeighboringTownTrade,
    }
}

pub fn building_style_for(household_type: HouseholdTypeToSpawn) -> BuildingStyle {
    match household_type {
        HouseholdTypeToSpawn::Family => BuildingStyle::FamilyHouse,
        HouseholdTypeToSpawn::GroceryShop => BuildingStyle::GroceryShop,
        HouseholdTypeToSpawn::GrainFarm
        | HouseholdTypeToSpawn::CowFarm
        | HouseholdTypeToSpawn::VegetableFarm => BuildingStyle::Field,
        HouseholdTypeToSpawn::Mill => BuildingStyle::Mill,
        HouseholdTypeToSpawn::Bakery => BuildingStyle::Bakery,
        HouseholdTypeToSpawn::NeighboringTownTrade => BuildingStyle::NeighboringTownConnection,
    }
}

#[derive(Compact, Clone)]
pub struct ImmigrationManager {
    id: ImmigrationManagerID,
    time: TimeID,
    development_manager: DevelopmentManagerID,
    state: ImmigrationManagerState,
}

impl ImmigrationManager {
    pub fn spawn(
        id: ImmigrationManagerID,
        time: TimeID,
        development_manager: DevelopmentManagerID,
        world: &mut World,
    ) -> ImmigrationManager {
        time.wake_up_in(IMMIGRATION_PACE.into(), id.into(), world);

        ImmigrationManager {
            id,
            time,
            development_manager,
            state: ImmigrationManagerState::Idle,
        }
    }
}

#[derive(Copy, Clone)]
pub enum ImmigrationManagerState {
    Idle,
    FindingBuilding(HouseholdTypeToSpawn),
}

const IMMIGRATION_PACE: Duration = Duration(10);

impl Sleeper for ImmigrationManager {
    fn wake(&mut self, current_instant: Instant, world: &mut World) {
        self.state = match self.state {
            ImmigrationManagerState::Idle => {
                let family_share = 1.0;
                let grocery_share = 0.2;
                let cow_farm_share = 0.3;
                let veg_farm_share = 0.26;
                let grain_farm_share = 0.2; //0.0016;
                let mill_share = 0.2; //0.001;
                let bakery_share = 0.2; //0.01;

                let total_share = family_share
                    + grocery_share
                    + cow_farm_share
                    + veg_farm_share
                    + grain_farm_share
                    + mill_share
                    + bakery_share;

                let dot = seed(current_instant).gen_range(0.0, total_share);

                let household_type_to_spawn = if dot < family_share {
                    HouseholdTypeToSpawn::Family
                } else if dot < family_share + grocery_share {
                    HouseholdTypeToSpawn::GroceryShop
                } else if dot < family_share + grocery_share + cow_farm_share {
                    HouseholdTypeToSpawn::CowFarm
                } else if dot < family_share + grocery_share + cow_farm_share + veg_farm_share {
                    HouseholdTypeToSpawn::VegetableFarm
                } else if dot
                    < family_share
                        + grocery_share
                        + cow_farm_share
                        + veg_farm_share
                        + grain_farm_share
                {
                    HouseholdTypeToSpawn::GrainFarm
                } else if dot
                    < family_share
                        + grocery_share
                        + cow_farm_share
                        + veg_farm_share
                        + grain_farm_share
                        + mill_share
                {
                    HouseholdTypeToSpawn::Mill
                } else {
                    HouseholdTypeToSpawn::Bakery
                };

                println!("Trying to spawn {:?}", household_type_to_spawn);

                let required_unit_type = unit_type_for(household_type_to_spawn);

                BuildingID::global_broadcast(world).try_offer_unit(
                    required_unit_type,
                    self.id,
                    world,
                );

                ImmigrationManagerState::FindingBuilding(household_type_to_spawn)
            }
            ImmigrationManagerState::FindingBuilding(household_type_to_spawn) => {
                // didn't find a building in time
                self.development_manager
                    .try_develop(building_style_for(household_type_to_spawn), world);

                ImmigrationManagerState::Idle
            }
        };

        self.time
            .wake_up_in(IMMIGRATION_PACE.into(), self.id.into(), world);
    }
}

impl ImmigrationManager {
    pub fn on_unit_offer(&mut self, building_id: BuildingID, unit_idx: UnitIdx, world: &mut World) {
        println!("Got offer");
        self.state = match self.state {
            ImmigrationManagerState::FindingBuilding(household_type_to_spawn) => {
                println!("Moving in");

                let household_id = match household_type_to_spawn {
                    HouseholdTypeToSpawn::Family => {
                        FamilyID::move_into(3, building_id, self.time, world).into()
                    }
                    HouseholdTypeToSpawn::GroceryShop => {
                        GroceryShopID::move_into(building_id, self.time, world).into()
                    }
                    HouseholdTypeToSpawn::GrainFarm => {
                        GrainFarmID::move_into(building_id, self.time, world).into()
                    }
                    HouseholdTypeToSpawn::CowFarm => {
                        CowFarmID::move_into(building_id, self.time, world).into()
                    }
                    HouseholdTypeToSpawn::VegetableFarm => {
                        VegetableFarmID::move_into(building_id, self.time, world).into()
                    }
                    HouseholdTypeToSpawn::Mill => {
                        MillID::move_into(building_id, self.time, world).into()
                    }
                    HouseholdTypeToSpawn::Bakery => {
                        BakeryID::move_into(building_id, self.time, world).into()
                    }
                    HouseholdTypeToSpawn::NeighboringTownTrade => {
                        NeighboringTownTradeID::move_into(building_id, self.time, world).into()
                    }
                };

                building_id.add_household(household_id, unit_idx, world);

                ImmigrationManagerState::Idle
            }
            ImmigrationManagerState::Idle => ImmigrationManagerState::Idle,
        }
    }
}

#[derive(Compact, Clone)]
pub struct DevelopmentManager {
    id: DevelopmentManagerID,
    time: TimeID,
    plan_manager: PlanManagerID,
    building_to_develop: COption<BuildingStyle>,
}

impl DevelopmentManager {
    pub fn spawn(
        id: DevelopmentManagerID,
        time: TimeID,
        plan_manager: PlanManagerID,
        _world: &mut World,
    ) -> DevelopmentManager {
        DevelopmentManager {
            id,
            time,
            plan_manager,
            building_to_develop: COption(None),
        }
    }

    pub fn try_develop(&mut self, building_style: BuildingStyle, world: &mut World) {
        if self.building_to_develop.is_none() {
            println!("Trying to develop {:?}", building_style);
            self.building_to_develop = COption(Some(building_style));
            VacantLotID::global_broadcast(world).suggest_lot(building_style, self.id, world);
            self.time
                .wake_up_in(IMMIGRATION_PACE.into(), self.id.into(), world);
        }
    }

    pub fn on_suggested_lot(
        &mut self,
        building_intent: &BuildingIntent,
        based_on: PrototypeID,
        world: &mut World,
    ) {
        if let Some(building_to_develop) = *self.building_to_develop {
            if building_to_develop == building_intent.building_style {
                println!("Adding to plan {:?}", building_intent.building_style);
                self.plan_manager.implement_artificial_proposal(
                    Proposal::from_plan(Plan::from_gestures(Some((
                        GestureID::new(),
                        Gesture::new(
                            vec![building_intent.lot.center_point()].into(),
                            GestureIntent::Building(building_intent.clone()),
                        ),
                    )))),
                    vec![based_on].into(),
                    world,
                );
                self.building_to_develop = COption(None);
            }
        }
    }
}

impl Sleeper for DevelopmentManager {
    fn wake(&mut self, _: Instant, _world: &mut World) {
        self.building_to_develop = COption(None);
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<ImmigrationManager>();
    system.register::<DevelopmentManager>();
    auto_setup(system);
}

pub fn spawn(world: &mut World, time: TimeID, plan_manager: PlanManagerID) {
    let development_manager = DevelopmentManagerID::spawn(time, plan_manager, world);
    ImmigrationManagerID::spawn(time, development_manager, world);
}

mod kay_auto;
pub use self::kay_auto::*;
