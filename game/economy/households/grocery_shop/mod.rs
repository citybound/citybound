use kay::{ActorSystem, World, Fate, External, ID};
use kay::swarm::Swarm;
use imgui::Ui;
use core::simulation::{TimeOfDay, Seconds};
use economy::resources::{ResourceAmount, ResourceMap, Entry, r_id, r_properties, r_info,
                         all_resource_ids};
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
                site.into(),
                TimeOfDay::new(7, 0),
                TimeOfDay::new(20, 0),
                Deal::new(
                    (r_id("groceries"), 30.0),
                    vec![(r_id("money"), 40.0)],
                    Seconds(5 * 60),
                ),
                world,
            ),
            job_offer: OfferID::register(
                id.into(),
                site.into(),
                TimeOfDay::new(7, 0),
                TimeOfDay::new(20, 0),
                Deal::new((r_id("money"), 50.0), None, Seconds(5 * 60 * 60)),
                world,
            ),
        }
    }
}

impl Household for GroceryShop {
    fn receive_deal(&mut self, _deal: &Deal, _member: MemberIdx, _: &mut World) {
        unimplemented!()
    }

    fn provide_deal(&mut self, deal: &Deal, _: &mut World) {
        let (resource, amount) = deal.give;
        *self.resources.mut_entry_or(resource, 0.0) -= amount;

        for &Entry(resource, amount) in &*deal.take {
            *self.resources.mut_entry_or(resource, 0.0) += amount;
        }
    }

    fn task_succeeded(&mut self, _member: MemberIdx, _: &mut World) {
        unimplemented!()
    }

    fn task_failed(&mut self, _member: MemberIdx, _location: RoughLocationID, _: &mut World) {
        unimplemented!()
    }

    fn decay(&mut self, dt: Seconds, _: &mut World) {
        let groceries = self.resources.mut_entry_or(r_id("groceries"), 0.0);
        *groceries += 0.001 * dt.seconds() as f32;
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
                            ui.same_line(150.0);
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
    system.add(Swarm::<GroceryShop>::new(), |_| {});
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
