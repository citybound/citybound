use kay::{ActorSystem, World};
use core::simulation::{Duration, TimeOfDay};

use transport::pathfinding::RoughLocationID;

pub mod tasks;
pub mod family;
pub mod grocery_shop;
pub mod crop_farm;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct MemberIdx(usize);

use imgui::Ui;
use kay::External;

use super::market::{Deal, OfferID};
use super::buildings::rendering::BuildingInspectorID;
use super::resources::{Resource, ResourceAmount};

pub trait Household {
    fn is_shared(resource: Resource) -> bool;
    fn supplier_shared(resource: Resource) -> bool;
    fn importance(resource: Resource, time: TimeOfDay) -> f32;
    fn graveness(resource: Resource, amount: ResourceAmount, time: TimeOfDay) -> f32 {
        -amount * Self::importance(resource, time)
    }
    fn interesting_resources() -> &'static [Resource];
    fn receive_deal(&mut self, deal: &Deal, member: MemberIdx, world: &mut World);
    fn provide_deal(&mut self, deal: &Deal, member: MemberIdx, world: &mut World);
    fn decay(&mut self, dt: Duration, world: &mut World);
    fn task_succeeded(&mut self, member: MemberIdx, world: &mut World);
    fn task_failed(&mut self, member: MemberIdx, location: RoughLocationID, world: &mut World);
    fn reset_member_task(&mut self, member: MemberIdx, world: &mut World);
    fn stop_using(&mut self, offer: OfferID, world: &mut World);
    fn destroy(&mut self, world: &mut World);
    fn inspect(
        &mut self,
        imgui_ui: &External<Ui<'static>>,
        return_to: BuildingInspectorID,
        world: &mut World,
    );
}

pub fn setup(system: &mut ActorSystem) {
    auto_setup(system);
    tasks::setup(system);
    family::setup(system);
    grocery_shop::setup(system);
    crop_farm::setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
