use kay::{ActorSystem, World, TypedID, Actor};
use simulation::{TimeOfDay, TimeOfDayRange, Duration, SimulationID, Ticks};
use economy::resources::Resource;
use economy::resources::Resource::*;
use economy::market::{Deal, EvaluationRequester, EvaluationRequesterID, EvaluatedSearchResult};
use land_use::buildings::BuildingID;

use super::{Household, HouseholdID, HouseholdCore, MemberIdx, Offer};

#[derive(Compact, Clone)]
pub struct VegetableFarm {
    id: VegetableFarmID,
    site: BuildingID,
    core: HouseholdCore,
}

impl VegetableFarm {
    pub fn move_into(
        id: VegetableFarmID,
        site: BuildingID,
        simulation: SimulationID,
        world: &mut World,
    ) -> VegetableFarm {
        simulation.wake_up_in(Ticks(0), id.into(), world);

        VegetableFarm {
            id,
            site,
            core: HouseholdCore::new(
                id.into(),
                world,
                1,
                site.into(),
                vec![
                    Offer::new(
                        MemberIdx(0),
                        TimeOfDayRange::new(7, 0, 20, 0),
                        Deal::new(
                            vec![(Resource::Produce, 20.0), (Resource::Money, -20.0 * 1.3)],
                            Duration::from_minutes(10),
                        ),
                        4,
                        false,
                    ),
                    Offer::new(
                        MemberIdx(0),
                        TimeOfDayRange::new(5, 0, 15, 0),
                        Deal::new(Some((Resource::Money, 40.0)), Duration::from_hours(4)),
                        2,
                        false,
                    ),
                ].into(),
            ),
        }
    }
}

impl Household for VegetableFarm {
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

    fn importance(_: Resource, _: TimeOfDay) -> f32 {
        0.0
    }

    fn interesting_resources() -> &'static [Resource] {
        &[Resource::Money, Resource::Produce]
    }

    fn decay(&mut self, dt: Duration, _: &mut World) {
        {
            let produce = self.core.resources.mut_entry_or(Produce, 0.0);
            *produce += 80.0 * dt.as_days();
        }
    }

    fn on_destroy(&mut self, world: &mut World) {
        self.site.remove_household(self.id_as(), world);
    }

    fn household_name(&self) -> String {
        "Vegetable Farm".to_owned()
    }

    fn member_name(&self, member: MemberIdx) -> String {
        format!("Farmer {}", member.0 + 1)
    }
}

use simulation::{Simulatable, SimulatableID, Sleeper, SleeperID, Instant, TICKS_PER_SIM_SECOND};
const UPDATE_EVERY_N_SECS: u32 = 4;

impl Simulatable for VegetableFarm {
    fn tick(&mut self, _dt: f32, current_instant: Instant, world: &mut World) {
        if (current_instant.ticks() + self.id.as_raw().instance_id as usize)
            % (UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND) as usize
            == 0
        {
            self.decay(Duration(UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND), world);
        }
    }
}

impl Sleeper for VegetableFarm {
    fn wake(&mut self, current_instant: Instant, world: &mut World) {
        self.update_core(current_instant, world);
    }
}

impl EvaluationRequester for VegetableFarm {
    fn expect_n_results(&mut self, _r: Resource, _n: u32, _: &mut World) {}
    fn on_result(&mut self, _e: &EvaluatedSearchResult, _: &mut World) {}
}

use transport::pathfinding::{RoughLocationID, RoughLocation, RoughLocationResolve};

impl RoughLocation for VegetableFarm {
    fn resolve(&self) -> RoughLocationResolve {
        RoughLocationResolve::SameAs(self.site())
    }
}

use transport::pathfinding::trip::{TripListener, TripListenerID, TripID, TripResult};

impl TripListener for VegetableFarm {
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
    system.register::<VegetableFarm>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
