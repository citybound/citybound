use kay::{ActorSystem, World, External, TypedID, Actor};
use imgui::Ui;
use core::simulation::{TimeOfDay, TimeOfDayRange, Duration};
use economy::resources::Resource;
use economy::market::{Deal, OfferID, EvaluationRequester, EvaluationRequesterID,
                      EvaluatedSearchResult};
use economy::buildings::BuildingID;
use economy::buildings::rendering::BuildingInspectorID;

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
    pub fn move_into(id: GroceryShopID, site: BuildingID, world: &mut World) -> GroceryShop {
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
                    vec![(Resource::Groceries, 30.0), (Resource::Money, -60.0)],
                    Duration::from_minutes(30),
                ),
                world,
            ),
            job_offer: OfferID::register(
                id.into(),
                MemberIdx(0),
                site.into(),
                TimeOfDayRange::new(7, 0, 15, 0),
                Deal::new(Some((Resource::Money, 50.0)), Duration::from_hours(5)),
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

    fn importance(_: Resource, _: TimeOfDay) -> f32 {
        1.0
    }

    fn interesting_resources() -> &'static [Resource] {
        &[Resource::Money, Resource::Groceries]
    }

    fn decay(&mut self, dt: Duration, _: &mut World) {
        let groceries = self.core.resources.mut_entry_or(Resource::Groceries, 0.0);
        *groceries += 0.001 * dt.as_seconds();
    }

    #[allow(useless_format)]
    fn inspect(
        &mut self,
        imgui_ui: &External<Ui<'static>>,
        return_to: BuildingInspectorID,
        world: &mut World,
    ) {
        let ui = imgui_ui.steal();

        ui.window(im_str!("Building")).build(|| {
            ui.tree_node(im_str!("Grocery Shop RawID: {:?}", self.id.as_raw()))
                .build(|| for resource in Self::interesting_resources() {
                    if Self::is_shared(*resource) {
                        ui.text(im_str!("{}", resource));
                        ui.same_line(100.0);
                        let amount = self.core.resources.get(*resource).cloned().unwrap_or(0.0);
                        ui.text(im_str!("{:.2}", amount));
                    }
                });
        });

        return_to.ui_drawn(ui, world);
    }

    fn on_destroy(&mut self, world: &mut World) {
        self.site.remove_household(self.id_as(), world);
        self.grocery_offer.withdraw(world);
        self.job_offer.withdraw(world);
    }
}

impl EvaluationRequester for GroceryShop {
    fn expect_n_results(&mut self, _r: Resource, _n: usize, _: &mut World) {}
    fn on_result(&mut self, _e: &EvaluatedSearchResult, _: &mut World) {}
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
