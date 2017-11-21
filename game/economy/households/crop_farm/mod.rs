use kay::{ActorSystem, World, External};
use imgui::Ui;
use core::simulation::{TimeOfDayRange, Duration};
use economy::resources::{Inventory, r_id, r_properties, r_info, all_resource_ids};
use economy::market::{Deal, OfferID};
use economy::buildings::BuildingID;
use economy::buildings::rendering::BuildingInspectorID;
use transport::pathfinding::RoughLocationID;

use super::{Household, HouseholdID, MemberIdx, MSG_Household_decay, MSG_Household_inspect,
            MSG_Household_provide_deal, MSG_Household_receive_deal, MSG_Household_task_succeeded,
            MSG_Household_task_failed, MSG_Household_destroy, MSG_Household_stop_using,
            MSG_Household_reset_member_task};


#[derive(Compact, Clone)]
pub struct CropFarm {
    id: CropFarmID,
    site: BuildingID,
    resources: Inventory,
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
                    vec![(r_id("crops"), 1000.0), (r_id("money"), -500.0)],
                    Duration::from_minutes(10),
                ),
                world,
            ),
            job_offer: OfferID::register(
                id.into(),
                MemberIdx(0),
                site.into(),
                TimeOfDayRange::new(5, 0, 15, 0),
                Deal::new(Some((r_id("money"), 60.0)), Duration::from_hours(7)),
                world,
            ),
        }
    }
}

impl Household for CropFarm {
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
        let crops = self.resources.mut_entry_or(r_id("crops"), 0.0);
        *crops += 0.001 * dt.as_seconds();
    }

    fn destroy(&mut self, world: &mut World) {
        self.site.remove_household(self.id.into(), world);
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
            ui.tree_node(im_str!("Crop Farm ID: {:?}", self.id._raw_id))
                .build(|| for resource in all_resource_ids() {
                    if r_properties(resource).ownership_shared {
                        ui.text(im_str!("{}", r_info(resource).0));
                        ui.same_line(100.0);
                        let amount = self.resources.get(resource).cloned().unwrap_or(0.0);
                        ui.text(im_str!("{:.2}", amount));
                    }
                });
        });

        return_to.ui_drawn(ui, world);
    }
}

use core::simulation::{Simulatable, SimulatableID, Instant, TICKS_PER_SIM_SECOND,
                       MSG_Simulatable_tick};
const UPDATE_EVERY_N_SECS: usize = 4;

impl Simulatable for CropFarm {
    fn tick(&mut self, _dt: f32, current_instant: Instant, world: &mut World) {
        if (current_instant.ticks() + self.id._raw_id.instance_id as usize) %
            (UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND) == 0
        {
            self.decay(Duration(UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND), world);
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<CropFarm>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
