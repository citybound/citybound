use kay::{ID, ActorSystem, Fate};
use kay::swarm::Swarm;
use compact::CVec;

#[derive(SubActor, Compact, Clone)]
pub struct Building {
    _id: Option<ID>,
    adjacent_lane: ID,
    households: CVec<ID>,
}

use stagemaster::geometry::CPath;

#[derive(Compact, Clone)]
pub struct Lot {
    footprint: CPath,
}

use game::lanes_and_cars::pathfinding::QueryAsDestination;

pub fn setup(system: &mut ActorSystem) {
    system.add(Swarm::<Building>::new(),
               Swarm::<Building>::subactors(|mut each_building| {
        each_building.on(|query: &QueryAsDestination, building, world| {
            world.send(building.adjacent_lane, *query);
            Fate::Live
        });
    }));
}

pub struct FindLot{pub requester: ID};
pub struct FoundLot(pub Lot);
pub struct CheckLot{pub lot: Lot, pub requester: ID};
pub struct LotResult{pub from: ID, pub obstructed: bool};