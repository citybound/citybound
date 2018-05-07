use kay::{ActorSystem, World, TypedID, Actor};
use simulation::{TimeOfDay, TimeOfDayRange, Duration, SimulationID, Ticks};
use economy::resources::Resource;
use economy::resources::Resource::*;
use economy::market::{Deal, EvaluationRequester, EvaluationRequesterID, EvaluatedSearchResult};
use land_use::buildings::BuildingID;

use super::{Household, HouseholdID, HouseholdCore, MemberIdx, Offer};


#[derive(Compact, Clone)]
pub struct Mill {
    id: MillID,
    site: BuildingID,
    core: HouseholdCore,
}

impl Mill {
    pub fn move_into(
        id: MillID,
        site: BuildingID,
        simulation: SimulationID,
        world: &mut World,
    ) -> Mill {
        simulation.wake_up_in(Ticks(0), id.into(), world);

        Mill {
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
                            vec![(Resource::Flour, 200.0), (Resource::Money, -200.0 * 0.3)],
                            Duration::from_minutes(10),
                        ),
                        4,
                        false
                    ),
                    Offer::new(
                        MemberIdx(0),
                        TimeOfDayRange::new(5, 0, 15, 0),
                        Deal::new(Some((Resource::Money, 40.0)), Duration::from_hours(4)),
                        3,
                        false
                    ),
                ].into(),
            ),
        }
    }
}

impl Household for Mill {
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
            Grain => Some([0, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0]),
            _ => None,
        };

        bihourly_importance
            .map(|lookup| lookup[hour / 2] as f32)
            .unwrap_or(0.0)
    }

    fn interesting_resources() -> &'static [Resource] {
        &[Resource::Money, Resource::Grain, Resource::Flour]
    }

    fn decay(&mut self, dt: Duration, _: &mut World) {
        {
            let flour = self.core.resources.mut_entry_or(Flour, 0.0);
            *flour += 800.0 * dt.as_days();
        }

        {
            let grain = self.core.resources.mut_entry_or(Grain, 0.0);
            *grain -= 800.0 * 1.0 * dt.as_days();
        }
    }

    fn on_destroy(&mut self, world: &mut World) {
        self.site.remove_household(self.id_as(), world);
    }

    fn household_name(&self) -> String {
        "Mill".to_owned()
    }

    fn member_name(&self, member: MemberIdx) -> String {
        format!("Miller {}", member.0 + 1)
    }
}

use simulation::{Simulatable, SimulatableID, Sleeper, SleeperID, Instant, TICKS_PER_SIM_SECOND};
const UPDATE_EVERY_N_SECS: usize = 4;

impl Simulatable for Mill {
    fn tick(&mut self, _dt: f32, current_instant: Instant, world: &mut World) {
        if (current_instant.ticks() + self.id.as_raw().instance_id as usize) %
            (UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND) == 0
        {
            self.decay(Duration(UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND), world);
        }
    }
}

impl Sleeper for Mill {
    fn wake(&mut self, current_instant: Instant, world: &mut World) {
        self.update_core(current_instant, world);
    }
}

impl EvaluationRequester for Mill {
    fn expect_n_results(&mut self, _r: Resource, _n: usize, _: &mut World) {}
    fn on_result(&mut self, _e: &EvaluatedSearchResult, _: &mut World) {}
}

use transport::pathfinding::{RoughLocationID, RoughLocation, RoughLocationResolve};

impl RoughLocation for Mill {
    fn resolve(&self) -> RoughLocationResolve {
        RoughLocationResolve::SameAs(self.site())
    }
}

use transport::pathfinding::trip::{TripListener, TripListenerID, TripID, TripResult};

impl TripListener for Mill {
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
    system.register::<Mill>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
