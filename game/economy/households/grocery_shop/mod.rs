use kay::{ActorSystem, World, TypedID, Actor};
use core::simulation::{TimeOfDay, TimeOfDayRange, Duration, SimulationID, Ticks};
use economy::resources::Resource;
use economy::resources::Resource::*;
use economy::market::{Deal, OfferID, EvaluationRequester, EvaluationRequesterID,
                      EvaluatedSearchResult};
use economy::buildings::BuildingID;

use super::{Household, HouseholdID, HouseholdCore, MemberIdx};

#[derive(Compact, Clone)]
pub struct GroceryShop {
    id: GroceryShopID,
    site: BuildingID,
    core: HouseholdCore,
    grocery_offer: OfferID,
    job_offer: OfferID,
}

impl GroceryShop {
    pub fn move_into(
        id: GroceryShopID,
        site: BuildingID,
        simulation: SimulationID,
        world: &mut World,
    ) -> GroceryShop {
        simulation.wake_up_in(Ticks(0), id.into(), world);

        GroceryShop {
            id,
            site,
            core: HouseholdCore::new(1, site.into()),
            grocery_offer: OfferID::register(
                id.into(),
                MemberIdx(0),
                site.into(),
                TimeOfDayRange::new(7, 0, 20, 0),
                Deal::new(
                    vec![(Groceries, 30.0), (Money, -60.0)],
                    Duration::from_minutes(30),
                ),
                30,
                world,
            ),
            job_offer: OfferID::register(
                id.into(),
                MemberIdx(0),
                site.into(),
                TimeOfDayRange::new(7, 0, 15, 0),
                Deal::new(Some((Money, 50.0)), Duration::from_hours(5)),
                5,
                world,
            ),
        }
    }
}

impl Household for GroceryShop {
    fn core(&self) -> &HouseholdCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut HouseholdCore {
        &mut self.core
    }

    fn site(&self) -> RoughLocationID {
        self.site.into()
    }

    fn is_shared(_: Resource) -> bool {
        true
    }

    fn supplier_shared(_: Resource) -> bool {
        true
    }

    fn importance(resource: Resource, time: TimeOfDay) -> f32 {
        let hour = time.hours_minutes().0;

        let bihourly_importance = match resource {
            BakedGoods | Produce | Grain | Flour | Meat | DairyGoods => Some(
                [
                    0,
                    0,
                    0,
                    1,
                    1,
                    1,
                    1,
                    1,
                    1,
                    0,
                    0,
                    0,
                ],
            ),
            _ => None,
        };

        bihourly_importance
            .map(|lookup| lookup[hour / 2] as f32)
            .unwrap_or(0.0)
    }

    fn interesting_resources() -> &'static [Resource] {
        &[
            Money,
            Groceries,
            Produce,
            Grain,
            Flour,
            BakedGoods,
            Meat,
            DairyGoods,
        ]
    }

    fn decay(&mut self, dt: Duration, _: &mut World) {
        {
            let groceries = self.core.resources.mut_entry_or(Groceries, 0.0);
            *groceries += 0.001 * dt.as_seconds();
        }

        for raw_resource in &[Produce, Grain, Flour, BakedGoods, Meat, DairyGoods] {
            let amount = self.core.resources.mut_entry_or(*raw_resource, 0.0);
            *amount -= 0.001 * dt.as_seconds();
        }
    }

    fn household_name(&self) -> String {
        "Grocery Shop".to_owned()
    }

    fn member_name(&self, member: MemberIdx) -> String {
        format!("Retail Worker {}", member.0 + 1)
    }

    fn on_destroy(&mut self, world: &mut World) {
        self.site.remove_household(self.id_as(), world);
        self.grocery_offer.withdraw(world);
        self.job_offer.withdraw(world);
    }
}

use super::ResultAspect;

impl EvaluationRequester for GroceryShop {
    fn expect_n_results(&mut self, resource: Resource, n: usize, world: &mut World) {
        self.update_results(resource, &ResultAspect::SetTarget(n), world);
    }

    fn on_result(&mut self, result: &EvaluatedSearchResult, world: &mut World) {
        let &EvaluatedSearchResult { resource, ref evaluated_deals, .. } = result;
        self.update_results(
            resource,
            &ResultAspect::AddDeals(evaluated_deals.clone()),
            world,
        );
    }
}

use core::simulation::{Simulatable, SimulatableID, Sleeper, SleeperID, Instant,
                       TICKS_PER_SIM_SECOND};
const UPDATE_EVERY_N_SECS: usize = 4;

impl Simulatable for GroceryShop {
    fn tick(&mut self, _dt: f32, current_instant: Instant, world: &mut World) {
        if (current_instant.ticks() + self.id.as_raw().instance_id as usize) %
            (UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND) == 0
        {
            self.decay(Duration(UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND), world);
        }
    }
}

impl Sleeper for GroceryShop {
    fn wake(&mut self, current_instant: Instant, world: &mut World) {
        self.update_core(current_instant, world);
    }
}

use transport::pathfinding::RoughLocationID;
use transport::pathfinding::trip::{TripListener, TripListenerID, TripID, TripResult};

impl TripListener for GroceryShop {
    fn trip_created(&mut self, trip: TripID, world: &mut World) {
        self.on_trip_created(trip, world);
    }

    fn trip_result(
        &mut self,
        trip: TripID,
        result: TripResult,
        rough_source: RoughLocationID,
        rough_destination: RoughLocationID,
        world: &mut World,
    ) {
        self.on_trip_result(trip, result, rough_source, rough_destination, world);
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<GroceryShop>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
