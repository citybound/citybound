use kay::{ID, ActorSystem, Fate};
use kay::swarm::{Swarm, Create, ToRandom};
use compact::CVec;
use descartes::P2;
use stagemaster::combo::{Bindings, Combo2};
use stagemaster::{UserInterface, AddInteractable, Focus};
use stagemaster::environment::Environment;
use stagemaster::combo::Button::*;
use stagemaster::Event3d;
use stagemaster::geometry::AnyShape;
use game::lanes_and_cars::lane::Lane;

#[derive(SubActor, Compact, Clone)]
pub struct Building {
    _id: Option<ID>,
    households: CVec<ID>,
    lot: Lot,
}

#[derive(Compact, Clone)]
pub struct Lot {
    pub position: P2,
    pub adjacent_lane: ID,
}

use game::lanes_and_cars::pathfinding::QueryAsDestination;

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

pub fn setup(system: &mut ActorSystem) {
    system.add(
        Swarm::<Building>::new(),
        Swarm::<Building>::subactors(|mut each_building| {
            each_building.on(|query: &QueryAsDestination, building, world| {
                world.send(building.lot.adjacent_lane, *query);
                Fate::Live
            });
        }),
    );

    let spawner = ::ENV.load_settings("Building Spawning");

    system.add::<BuildingSpawner, _>(spawner, |mut the_spawner| {
        the_spawner.on(|&FoundLot(ref lot), _, world| {
            world.send_to_id_of::<Swarm<Building>, _>(Create(Building {
                                                                 _id: None,
                                                                 households: CVec::new(),
                                                                 lot: lot.clone(),
                                                             }));
            println!("Created a building");
            Fate::Live
        });

        the_spawner.on(move |event, spawner, world| {
            if let Event3d::Combos(combos) = *event {
                spawner.bindings.do_rebinding(&combos.current);
                let bindings = &spawner.bindings;

                if bindings["Spawn Building"].is_freshly_in(&combos) {
                    let spawner_id = world.id::<BuildingSpawner>();
                    world.send_to_id_of::<Swarm<Lane>, _>(ToRandom {
                                                              message: FindLot {
                                                                  requester: spawner_id,
                                                              },
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