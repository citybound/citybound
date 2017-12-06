use kay::{ActorSystem, World, TypedID, Actor};
use core::simulation::{TimeOfDay, TimeOfDayRange, Duration, SimulationID, Ticks};
use economy::resources::Resource;
use economy::market::{Deal, OfferID, EvaluationRequester, EvaluationRequesterID,
                      EvaluatedSearchResult};
use economy::buildings::BuildingID;

use super::{Household, HouseholdID, HouseholdCore, MemberIdx};


#[derive(Compact, Clone)]
pub struct GrainFarm {
    id: GrainFarmID,
    site: BuildingID,
    core: HouseholdCore,
    grain_offer: OfferID,
    job_offer: OfferID,
}

impl GrainFarm {
    pub fn move_into(
        id: GrainFarmID,
        site: BuildingID,
        simulation: SimulationID,
        world: &mut World,
    ) -> GrainFarm {
        simulation.wake_up_in(Ticks(0), id.into(), world);

        GrainFarm {
            id,
            site,
            core: HouseholdCore::new(1, site.into()),
            grain_offer: OfferID::register(
                id.into(),
                MemberIdx(0),
                site.into(),
                TimeOfDayRange::new(7, 0, 20, 0),
                Deal::new(
                    vec![(Resource::Grain, 1000.0), (Resource::Money, -500.0)],
                    Duration::from_minutes(10),
                ),
                3,
                world,
            ),
            job_offer: OfferID::register(
                id.into(),
                MemberIdx(0),
                site.into(),
                TimeOfDayRange::new(5, 0, 15, 0),
                Deal::new(Some((Resource::Money, 60.0)), Duration::from_hours(7)),
                3,
                world,
            ),
        }
    }
}

impl Household for GrainFarm {
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
        &[Resource::Money, Resource::Grain]
    }

    fn decay(&mut self, _dt: Duration, _: &mut World) {}

    fn on_destroy(&mut self, world: &mut World) {
        self.site.remove_household(self.id_as(), world);
        self.grain_offer.withdraw(world);
        self.job_offer.withdraw(world);
    }

    fn household_name(&self) -> String {
        "Grain Farm".to_owned()
    }

    fn member_name(&self, member: MemberIdx) -> String {
        format!("Farmer {}", member.0 + 1)
    }
}

use core::simulation::{Simulatable, SimulatableID, Sleeper, SleeperID, Instant,
                       TICKS_PER_SIM_SECOND};
const UPDATE_EVERY_N_SECS: usize = 4;

impl Simulatable for GrainFarm {
    fn tick(&mut self, _dt: f32, current_instant: Instant, world: &mut World) {
        if (current_instant.ticks() + self.id.as_raw().instance_id as usize) %
            (UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND) == 0
        {
            self.decay(Duration(UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND), world);
        }
    }
}

impl Sleeper for GrainFarm {
    fn wake(&mut self, current_instant: Instant, world: &mut World) {
        self.update_core(current_instant, world);
    }
}

impl EvaluationRequester for GrainFarm {
    fn expect_n_results(&mut self, _r: Resource, _n: usize, _: &mut World) {}
    fn on_result(&mut self, _e: &EvaluatedSearchResult, _: &mut World) {}
}

use transport::pathfinding::RoughLocationID;
use transport::pathfinding::trip::{TripListener, TripListenerID, TripID, TripResult};

impl TripListener for GrainFarm {
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
    system.register::<GrainFarm>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
