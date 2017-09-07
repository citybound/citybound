use kay::{ActorSystem, World};
use core::simulation::Seconds;

use transport::pathfinding::RoughLocationID;

pub mod tasks;
pub mod family;
pub mod grocery_shop;

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct MemberIdx(usize);

use imgui::Ui;
use kay::{External, ID};

use super::market::Deal;

pub trait Household {
    fn receive_deal(&mut self, deal: &Deal, member: MemberIdx, world: &mut World);
    fn provide_deal(&mut self, deal: &Deal, world: &mut World);
    fn decay(&mut self, dt: Seconds, world: &mut World);
    fn task_succeeded(&mut self, member: MemberIdx, world: &mut World);
    fn task_failed(&mut self, member: MemberIdx, location: RoughLocationID, world: &mut World);
    fn inspect(&mut self, imgui_ui: &External<Ui<'static>>, return_to: ID, world: &mut World);
}

pub fn setup(system: &mut ActorSystem) {
    auto_setup(system);
    tasks::setup(system);
    family::setup(system);
    grocery_shop::setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
