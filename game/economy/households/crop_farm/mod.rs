use kay::{ActorSystem, World, External, TypedID, Actor};
use imgui::Ui;
use core::simulation::{TimeOfDay, TimeOfDayRange, Duration};
use economy::resources::{Inventory, Resource};
use economy::market::{Deal, OfferID, EvaluationRequester, EvaluationRequesterID,
                      EvaluatedSearchResult};
use economy::buildings::BuildingID;
use economy::buildings::rendering::BuildingInspectorID;
use transport::pathfinding::RoughLocationID;

use super::{Household, HouseholdID, HouseholdCore, MemberIdx};


#[derive(Compact, Clone)]
pub struct CropFarm {
    id: CropFarmID,
    site: BuildingID,
    core: HouseholdCore,
    crops_offer: OfferID,
    job_offer: OfferID,
}

impl CropFarm {
    pub fn move_into(id: CropFarmID, site: BuildingID, world: &mut World) -> CropFarm {
        CropFarm {
            id,
            site,
            resources: Inventory::new(),
            crops_offer: OfferID::register(
                id.into(),
                MemberIdx(0),
                site.into(),
                TimeOfDayRange::new(7, 0, 20, 0),
                Deal::new(
                    vec![(Resource::Crops, 1000.0), (Resource::Money, -500.0)],
                    Duration::from_minutes(10),
                ),
                world,
            ),
            job_offer: OfferID::register(
                id.into(),
                MemberIdx(0),
                site.into(),
                TimeOfDayRange::new(5, 0, 15, 0),
                Deal::new(Some((Resource::Money, 60.0)), Duration::from_hours(7)),
                world,
            ),
        }
    }
}

impl Household for CropFarm {
    fn core(&self) -> &HouseholdCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut HouseholdCore {
        &mut self.core
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
        &[Resource::Money, Resource::Crops]
    }

    fn receive_deal(&mut self, deal: &Deal, _member: MemberIdx, _: &mut World) {
        deal.delta.give_to(&mut self.resources);
    }

    fn provide_deal(&mut self, deal: &Deal, _member: MemberIdx, _: &mut World) {
        deal.delta.take_from(&mut self.resources);
    }

    fn task_succeeded(&mut self, _member: MemberIdx, _: &mut World) {
        unimplemented!()
    }

    fn task_failed(&mut self, _member: MemberIdx, _location: RoughLocationID, _: &mut World) {
        unimplemented!()
    }

    fn reset_member_task(&mut self, _member: MemberIdx, _: &mut World) {
        unimplemented!()
    }

    fn stop_using(&mut self, _offer: OfferID, _: &mut World) {
        unimplemented!()
    }

    fn decay(&mut self, dt: Duration, _: &mut World) {
        let crops = self.resources.mut_entry_or(Resource::Crops, 0.0);
        *crops += 0.001 * dt.as_seconds();
    }

    fn destroy(&mut self, world: &mut World) {
        self.site.remove_household(self.id_as(), world);
        self.crops_offer.withdraw(world);
        self.job_offer.withdraw(world);
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
            ui.tree_node(im_str!("Crop Farm ID: {:?}", self.id.as_raw()))
                .build(|| for resource in Self::interesting_resources() {
                    if Self::is_shared(*resource) {
                        ui.text(im_str!("{}", resource));
                        ui.same_line(100.0);
                        let amount = self.resources.get(*resource).cloned().unwrap_or(0.0);
                        ui.text(im_str!("{:.2}", amount));
                    }
                });
        });

        return_to.ui_drawn(ui, world);
    }

    fn on_destroy(&mut self, _: &mut World) {}
}

use core::simulation::{Simulatable, SimulatableID, Sleeper, SleeperID, Instant,
                       TICKS_PER_SIM_SECOND};
const UPDATE_EVERY_N_SECS: usize = 4;

impl Simulatable for CropFarm {
    fn tick(&mut self, _dt: f32, current_instant: Instant, world: &mut World) {
        if (current_instant.ticks() + self.id.as_raw().instance_id as usize) %
            (UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND) == 0
        {
            self.decay(Duration(UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND), world);
        }
    }
}

impl Sleeper for CropFarm {
    fn wake(&mut self, current_instant: Instant, world: &mut World) {
        self.update_core(current_instant, world);
    }
}

impl EvaluationRequester for CropFarm {
    fn expect_n_results(&mut self, _r: Resource, _n: usize, _: &mut World) {}
    fn on_result(&mut self, _e: &EvaluatedSearchResult, _: &mut World) {}
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<CropFarm>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
