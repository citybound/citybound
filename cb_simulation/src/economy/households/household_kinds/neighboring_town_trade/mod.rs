use kay::{ActorSystem, World, Actor};
use cb_time::actors::{Temporal, TemporalID, TimeID};
use cb_time::units::{TimeOfDay, TimeOfDayRange, Duration, Instant, Ticks};
use economy::resources::Resource;
use economy::resources::Resource::*;
use economy::market::{Deal, EvaluationRequester, EvaluationRequesterID, EvaluatedSearchResult};
use land_use::buildings::BuildingID;
use transport::pathfinding::RoughLocationID;
use transport::pathfinding::trip::{TripListener, TripListenerID, TripID, TripResult};

use economy::households::{Household, HouseholdID, HouseholdCore, MemberIdx, Offer};

#[derive(Compact, Clone)]
pub struct NeighboringTownTrade {
    id: NeighboringTownTradeID,
    town: BuildingID,
    core: HouseholdCore,
}

impl NeighboringTownTrade {
    pub fn move_into(
        id: NeighboringTownTradeID,
        town: BuildingID,
        time: TimeID,
        world: &mut World,
    ) -> Self {
        time.wake_up_in(Ticks(0), id.into(), world);

        let offers = vec![
            Offer::new(
                MemberIdx(0),
                TimeOfDayRange::new(5, 0, 15, 0),
                Deal::new(Some((Resource::Money, 50.0)), Duration::from_hours(5)),
                300,
                false,
            ),
            // Offer::new(
            //     MemberIdx(0),
            //     TimeOfDayRange::new(7, 0, 20, 0),
            //     Deal::new(
            //         vec![(Entertainment, 5.0), (Money, -10.0)],
            //         Duration::from_minutes(30),
            //     ),
            //     10,
            //     false
            // ),
            // Offer::new(
            //     MemberIdx(0),
            //     TimeOfDayRange::new(7, 0, 20, 0),
            //     Deal::new(
            //         vec![(Services, 5.0), (Money, -10.0)],
            //         Duration::from_minutes(30),
            //     ),
            //     10,
            //     false
            // ),
            Offer::new(
                MemberIdx(0),
                TimeOfDayRange::new(7, 0, 20, 0),
                Deal::new(
                    vec![(Groceries, 30.0), (Money, -30.0 * 2.7)],
                    Duration::from_minutes(30),
                ),
                32,
                false,
            ),
            Offer::new(
                MemberIdx(0),
                TimeOfDayRange::new(7, 0, 20, 0),
                Deal::new(
                    vec![(Resource::Produce, 20.0), (Resource::Money, -20.0 * 1.3)],
                    Duration::from_minutes(10),
                ),
                8,
                false,
            ),
            Offer::new(
                MemberIdx(0),
                TimeOfDayRange::new(7, 0, 20, 0),
                Deal::new(
                    vec![(Resource::Grain, 200.0), (Resource::Money, -200.0 * 0.13)],
                    Duration::from_minutes(10),
                ),
                8,
                false,
            ),
            Offer::new(
                MemberIdx(0),
                TimeOfDayRange::new(7, 0, 20, 0),
                Deal::new(
                    vec![(Resource::Flour, 200.0), (Resource::Money, -200.0 * 0.3)],
                    Duration::from_minutes(10),
                ),
                8,
                false,
            ),
            Offer::new(
                MemberIdx(0),
                TimeOfDayRange::new(7, 0, 20, 0),
                Deal::new(
                    vec![
                        (Resource::BakedGoods, 100.0),
                        (Resource::Money, -100.0 * 2.5),
                    ],
                    Duration::from_minutes(10),
                ),
                60,
                false,
            ),
            Offer::new(
                MemberIdx(0),
                TimeOfDayRange::new(7, 0, 20, 0),
                Deal::new(
                    vec![(Resource::Meat, 5.0), (Resource::Money, -5.0 * 3.0)],
                    Duration::from_minutes(10),
                ),
                8,
                false,
            ),
            Offer::new(
                MemberIdx(0),
                TimeOfDayRange::new(7, 0, 20, 0),
                Deal::new(
                    vec![
                        (Resource::DairyGoods, 10.0),
                        (Resource::Money, -10.0 * 1.31),
                    ],
                    Duration::from_minutes(10),
                ),
                8,
                false,
            ),
            /* Offer::new(
             *     MemberIdx(0),
             *     TimeOfDayRange::new(7, 0, 20, 0),
             *     Deal::new(
             *         vec![(Wood, 30.0), (Money, -10.0)],
             *         Duration::from_minutes(10),
             *     ),
             *     10,
             *     false
             * ),
             * Offer::new(
             *     MemberIdx(0),
             *     TimeOfDayRange::new(7, 0, 20, 0),
             *     Deal::new(
             *         vec![(Furniture, 5.0), (Money, -100.0)],
             *         Duration::from_minutes(10),
             *     ),
             *     10,
             *     false
             * ),
             * Offer::new(
             *     MemberIdx(0),
             *     TimeOfDayRange::new(7, 0, 20, 0),
             *     Deal::new(
             *         vec![(TextileGoods, 30.0), (Money, -30.0)],
             *         Duration::from_minutes(10),
             *     ),
             *     10,
             *     false
             * ),
             * Offer::new(
             *     MemberIdx(0),
             *     TimeOfDayRange::new(7, 0, 20, 0),
             *     Deal::new(
             *         vec![(Clothes, 5.0), (Money, -50.0)],
             *         Duration::from_minutes(10),
             *     ),
             *     10,
             *     false
             * ),
             * Offer::new(
             *     MemberIdx(0),
             *     TimeOfDayRange::new(7, 0, 20, 0),
             *     Deal::new(
             *         vec![(Devices, 5.0), (Money, -100.0)],
             *         Duration::from_minutes(10),
             *     ),
             *     10,
             *     false
             * ), */
        ];

        NeighboringTownTrade {
            id,
            town,
            core: HouseholdCore::new(id.into(), world, 10, town.into(), offers.into()),
        }
    }
}

impl Household for NeighboringTownTrade {
    fn core(&self) -> &HouseholdCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut HouseholdCore {
        &mut self.core
    }

    fn site(&self) -> RoughLocationID {
        self.town.into()
    }

    fn is_shared(_: Resource) -> bool {
        true
    }

    fn supplier_shared(_: Resource) -> bool {
        true
    }

    fn importance(_: Resource, _: TimeOfDay) -> f32 {
        1.0
    }

    fn interesting_resources() -> &'static [Resource] {
        &[
            //Entertainment,
            //Services,
            Groceries, Produce, Grain, Flour, BakedGoods, Meat,
            DairyGoods,
            /* Wood,
             *Furniture,
             *TextileGoods,
             *Clothes,
             *Devices, */
        ]
    }

    fn decay(&mut self, _dt: Duration, _: &mut World) {}

    fn household_name(&self) -> String {
        "Neighboring Town".to_owned()
    }

    fn member_name(&self, member: MemberIdx) -> String {
        format!("Neighboring Town Worker {}", member.0 + 1)
    }

    fn on_destroy(&mut self, world: &mut World) {
        self.town.remove_household(self.id_as(), world);
    }
}

use cb_time::actors::{Sleeper, SleeperID};

impl Sleeper for NeighboringTownTrade {
    fn wake(&mut self, current_instant: Instant, world: &mut World) {
        self.update_core(current_instant, world);
    }
}

use economy::households::ResultAspect;

impl EvaluationRequester for NeighboringTownTrade {
    fn expect_n_results(&mut self, resource: Resource, n: u32, world: &mut World) {
        self.update_results(resource, &ResultAspect::SetTarget(n), world);
    }

    fn on_result(&mut self, result: &EvaluatedSearchResult, world: &mut World) {
        let &EvaluatedSearchResult {
            resource,
            ref evaluated_deals,
            ..
        } = result;
        self.update_results(
            resource,
            &ResultAspect::AddDeals(evaluated_deals.clone()),
            world,
        );
    }
}

impl TripListener for NeighboringTownTrade {
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

impl Temporal for NeighboringTownTrade {
    fn tick(&mut self, _dt: f32, current_instant: Instant, world: &mut World) {
        self.on_tick(current_instant, world);
    }
}

use transport::pathfinding::{RoughLocation, RoughLocationResolve};

impl RoughLocation for NeighboringTownTrade {
    fn resolve(&self) -> RoughLocationResolve {
        RoughLocationResolve::SameAs(self.site())
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<NeighboringTownTrade>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
