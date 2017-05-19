use kay::{ID, ActorSystem, Fate};
use kay::swarm::Swarm;
use compact::CVec;

#[derive(SubActor, Compact, Clone)]
pub struct Building {
    _id: Option<ID>,
    adjacent_lane: ID,
    households: CVec<ID>,
}

use game::lanes_and_cars::pathfinding::QueryAsDestination;

pub fn setup(system: &mut ActorSystem) {
    system.add(Swarm::<Building>::new(),
               Swarm::<Building>::subactors(|mut each_building| {
        each_building.on(|&QueryAsDestination { rough_destination, requester },
                          building,
                          world| {
            world.send(building.adjacent_lane,
                       QueryAsDestination { rough_destination, requester });
            Fate::Live
        });
    }));
}