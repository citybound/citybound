use kay::{ActorSystem, World, External};
use imgui::Ui;
use core::simulation::{TimeOfDayRange, Duration};
use economy::resources::{ResourceAmount, ResourceMap, r_id, r_properties, r_info, all_resource_ids};
use economy::market::{Deal, OfferID};
use economy::buildings::BuildingID;
use economy::buildings::rendering::BuildingInspectorID;
use transport::pathfinding::RoughLocationID;

use super::{Household, HouseholdID, MemberIdx, MSG_Household_decay, MSG_Household_inspect,
            MSG_Household_provide_deal, MSG_Household_receive_deal, MSG_Household_task_succeeded,
            MSG_Household_task_failed};

#[derive(Compact, Clone)]
pub struct GroceryShop {
    id: GroceryShopID,
    site: BuildingID,
    resources: ResourceMap<ResourceAmount>,
    grocery_offer: OfferID,
    job_offer: OfferID,
}

impl GroceryShop {
    pub fn move_into(id: GroceryShopID, site: BuildingID, world: &mut World) -> GroceryShop {
        GroceryShop {
            id,
            site,
            resources: ResourceMap::new(),
            grocery_offer: OfferID::register(
                id.into(),
                MemberIdx(0),
                site.into(),
                TimeOfDayRange::new(7, 0, 20, 0),
                Deal::new(
                    vec![(r_id("groceries"), 30.0), (r_id("money"), -40.0)],
                    Duration(5 * 60),
                ),
                world,
            ),
            job_offer: OfferID::register(
                id.into(),
                MemberIdx(0),
                site.into(),
                TimeOfDayRange::new(7, 0, 20, 0),
                Deal::new(Some((r_id("money"), 50.0)), Duration(5 * 60 * 60)),
                world,
            ),
        }
    }
}

impl Household for GroceryShop {
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

    fn decay(&mut self, dt: Duration, _: &mut World) {
        let groceries = self.resources.mut_entry_or(r_id("groceries"), 0.0);
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
            ui.tree_node(im_str!("Grocery Shop ID: {:?}", self.id._raw_id))
                .build(|| {
                    ui.tree_node(im_str!("Resources")).build(|| for resource in
                        all_resource_ids()
                    {
                        if r_properties(resource).ownership_shared {
                            ui.text(im_str!("{}", r_info(resource).0));
                            ui.same_line(250.0);
                            let amount = self.resources.get(resource).cloned().unwrap_or(0.0);
                            ui.text(im_str!("{}", amount));
                        }
                    });
                });
        });

        return_to.ui_drawn(ui, world);
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<GroceryShop>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
