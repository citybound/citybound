use kay::{ID, ActorSystem, Fate, World};
use kay::swarm::{Swarm, ToRandom};
use compact::CVec;
use descartes::P2;
use stagemaster::combo::{Bindings, Combo2};
use stagemaster::{UserInterface, AddInteractable, Focus};
use stagemaster::combo::Button::*;
use stagemaster::Event3d;
use stagemaster::geometry::AnyShape;
use transport::lane::Lane;

mod rendering;

use super::households::HouseholdID;

#[derive(Compact, Clone)]
pub struct Building {
    id: BuildingID,
    households: CVec<HouseholdID>,
    lot: Lot,
}

impl Building {
    pub fn spawn(
        id: BuildingID,
        households: &CVec<HouseholdID>,
        lot: &Lot,
        world: &mut World,
    ) -> Building {
        rendering::on_add(id, lot.position, world);
        Building {
            id,
            households: households.clone(),
            lot: lot.clone(),
        }
    }

    pub fn add_household(&mut self, household: HouseholdID, _: &mut World) {
        self.households.push(household);
    }
}

use transport::pathfinding::{RoughDestination, AsDestinationRequesterID, RoughDestinationID,
                             MSG_RoughDestination_query_as_destination};
use core::simulation::Timestamp;

impl RoughDestination for Building {
    fn query_as_destination(
        &mut self,
        requester: AsDestinationRequesterID,
        rough_destination: RoughDestinationID,
        tick: Timestamp,
        world: &mut World,
    ) {
        let adjacent_lane = RoughDestinationID { _raw_id: self.lot.adjacent_lane };
        adjacent_lane.query_as_destination(requester, rough_destination, tick, world)
    }
}

#[derive(Compact, Clone)]
pub struct Lot {
    pub position: P2,
    pub adjacent_lane: ID,
}

#[derive(Serialize, Deserialize)]
pub struct BuildingSpawner {
    bindings: Bindings,
}

impl Default for BuildingSpawner {
    fn default() -> Self {
        BuildingSpawner {
            bindings: Bindings::new(vec![("Spawn Building", Combo2::new(&[B], &[]))]),
        }
    }
}

#[derive(Copy, Clone)]
pub struct FindLot {
    pub requester: ID,
}
#[derive(Compact, Clone)]
pub struct FoundLot(pub Lot);

#[derive(Copy, Clone)]
pub struct InitializeUI;

use super::households::FamilyID;
use super::households::GroceryShopID;

pub fn setup(system: &mut ActorSystem) {
    system.add(Swarm::<Building>::new(), |_| {});

    kay_auto::auto_setup(system);

    let spawner = ::ENV.load_settings("Building Spawning");

    system.add::<BuildingSpawner, _>(spawner, |mut the_spawner| {
        the_spawner.on(|&FoundLot(ref lot), _, world| {
            let building_id = BuildingID::spawn(CVec::new(), lot.clone(), world);

            if building_id._raw_id.sub_actor_id % 6 == 0 {
                let shop_id = GroceryShopID::move_into(building_id, world);
                building_id.add_household(shop_id.into(), world);
            } else {
                let family_id = FamilyID::move_into(3, building_id, world);
                building_id.add_household(family_id.into(), world);
            }

            Fate::Live
        });

        the_spawner.on(move |event, spawner, world| {
            if let Event3d::Combos(combos) = *event {
                spawner.bindings.do_rebinding(&combos.current);
                let bindings = &spawner.bindings;

                if bindings["Spawn Building"].is_freshly_in(&combos) {
                    let spawner_id = world.id::<BuildingSpawner>();
                    world.send_to_id_of::<Swarm<Lane>, _>(ToRandom {
                        message: FindLot { requester: spawner_id },
                        n_recipients: 50,
                    })
                }
            };

            Fate::Live
        });

        let ui_id = the_spawner.world().id::<UserInterface>();
        let spawner_id = the_spawner.world().id::<BuildingSpawner>();

        the_spawner.on(move |_: &InitializeUI, _, world| {
            world.send(ui_id, AddInteractable(spawner_id, AnyShape::Everywhere, 0));
            world.send(ui_id, Focus(spawner_id));
            Fate::Live
        });

        the_spawner.world().send(spawner_id, InitializeUI);
    });

}

pub fn setup_ui(system: &mut ActorSystem) {
    rendering::setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
