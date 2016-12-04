use kay::{Actor, CDict, ID, Individual, Recipient, Fate, ActorSystem, Swarm};
use core::geometry::AnyShape;
use descartes::Band;
use super::{Lane, TransferLane, Interaction, InteractionKind, OverlapKind};

pub mod trip;

#[derive(Compact, Clone, Default)]
pub struct PathfindingInfo{
    pub as_destination: Option<Destination>,
    pub hops_from_landmark: u8,
    pub incoming_idx_from_landmark: u8,
    pub routes: CDict<Destination, RoutingInfo>
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Destination{
    pub landmark: ID,
    pub outgoing_idx_at_landmark: u8,
    pub node: ID
}

#[derive(Copy, Clone, Debug)]
pub struct RoutingInfo{
    pub outgoing_idx: u8,
    pub distance: f32
}

use ::core::ui::Add;

pub fn on_build(lane: &mut Lane) {
    lane.pathfinding_info.as_destination = None;
    ::core::ui::UserInterface::id() << Add::Interactable3d(lane.id(), AnyShape::Band(Band::new(lane.path.clone(), 3.0)), 5);
}

const MIN_LANDMARK_INCOMING : usize = 3;

pub fn tick(lane: &mut Lane) {
    if let Some(as_destination) = lane.pathfinding_info.as_destination {
        for (interaction_idx, successor) in successors(lane) {
            successor << JoinLandmark{
                from: lane.id(),
                join_as: Destination{
                    landmark: as_destination.landmark,
                    outgoing_idx_at_landmark: if as_destination.landmark == as_destination.node {
                        interaction_idx
                    } else {
                        as_destination.outgoing_idx_at_landmark
                    },
                    node: successor
                },
                hops_from_landmark: lane.pathfinding_info.hops_from_landmark + 1
            }
        }
    } else if predecessors(lane).count() >= MIN_LANDMARK_INCOMING {
        lane.pathfinding_info = PathfindingInfo{
            as_destination: Some(Destination{
                landmark: lane.id(),
                outgoing_idx_at_landmark: 0,
                node: lane.id()
            }),
            hops_from_landmark: 0,
            incoming_idx_from_landmark: 0,
            routes: CDict::new()
        }
    }

    for (_, predecessor, is_transfer) in predecessors(lane) {
        let self_cost = if is_transfer {0.0} else {lane.length};
        predecessor << ShareRoutes{
            new_routes: lane.pathfinding_info.routes.pairs().map(|(&destination, &RoutingInfo{distance, ..})|
                (destination, distance + self_cost)
            ).chain(lane.pathfinding_info.as_destination.map(|destination|
                (destination, self_cost)
            )).collect(),
            from: lane.id()
        };
    }
}

#[allow(needless_lifetimes)]
fn successors<'a>(lane: &'a Lane) -> impl Iterator<Item=(u8, ID)> + 'a{
    lane.interactions.iter().enumerate().filter_map(|(i, interaction)|
        match *interaction {
            Interaction{partner_lane, kind: InteractionKind::Overlap{kind: OverlapKind::Transfer, ..}, ..} 
            | Interaction{partner_lane, kind: InteractionKind::Next{..}, ..} => {
                Some((i as u8, partner_lane))
            },
            _ => None
        }
    )
}

#[allow(needless_lifetimes)]
fn predecessors<'a>(lane: &'a Lane) -> impl Iterator<Item=(u8, ID, bool)> + 'a{
    lane.interactions.iter().enumerate().filter_map(|(i, interaction)|
        match *interaction {
            Interaction{partner_lane, kind: InteractionKind::Overlap{kind: OverlapKind::Transfer, ..}, ..} => {
                Some((i as u8, partner_lane, true))
            },
            Interaction{partner_lane, kind: InteractionKind::Previous{..}, ..} => {
                Some((i as u8, partner_lane, false))
            },
            _ => None
        }
    )
}

#[derive(Copy, Clone)]
pub struct JoinLandmark{
    from: ID,
    join_as: Destination,
    hops_from_landmark: u8
}

const IDEAL_LANDMARK_RADIUS : u8 = 6;

impl Recipient<JoinLandmark> for Lane {
     fn receive(&mut self, msg: &JoinLandmark) -> Fate {match *msg{
         JoinLandmark{join_as, hops_from_landmark, from} => {
            if let Some(from_interaction_idx) = self.interactions.iter().position(|interaction| interaction.partner_lane == from) {
                let join = if let Some(as_destination) = self.pathfinding_info.as_destination {
                    if as_destination.landmark == as_destination.node {
                        join_as.landmark.instance_id > self.id().instance_id
                    } else if join_as.landmark == as_destination.landmark {
                        hops_from_landmark < self.pathfinding_info.hops_from_landmark
                    } else {
                        from_interaction_idx == self.pathfinding_info.incoming_idx_from_landmark as usize
                        && join_as != as_destination
                    }
                } else {
                    true
                };
                if join {
                    self.pathfinding_info = PathfindingInfo{
                        as_destination: Some(join_as),
                        hops_from_landmark: hops_from_landmark,
                        incoming_idx_from_landmark: from_interaction_idx as u8,
                        routes: CDict::new()
                    };
                }
            } else {
                println!("{:?} not yet connected to {:?}", self.id(), from);
            }
            Fate::Live
         }
     }}
}

use core::geometry::add_debug_path;
use descartes::FiniteCurve;

impl Recipient<JoinLandmark> for TransferLane {
     fn receive(&mut self, msg: &JoinLandmark) -> Fate {match *msg{
         JoinLandmark{join_as, hops_from_landmark, from} => {
             if self.left.is_none() {
                 add_debug_path(self.path.shift_orthogonally(-1.5).unwrap_or(self.path.clone()), [1.0, 0.0, 0.0], 0.6);
             }
             if self.right.is_none() {
                 add_debug_path(self.path.shift_orthogonally(1.5).unwrap_or(self.path.clone()), [1.0, 0.0, 0.0], 0.6);
             }
             let left = self.left.expect("should have a left lane").0;
             if from == left {
                 let right = self.right.expect("should have a right lane").0;
                 right << JoinLandmark{
                     join_as: Destination{
                         landmark: join_as.landmark,
                         outgoing_idx_at_landmark: join_as.outgoing_idx_at_landmark,
                         node: right
                     },
                     hops_from_landmark: hops_from_landmark + 1,
                     from: self.id()
                 }
             } else {
                 left << JoinLandmark{
                     join_as: Destination{
                         landmark: join_as.landmark,
                         outgoing_idx_at_landmark: join_as.outgoing_idx_at_landmark,
                         node: left
                     },
                     hops_from_landmark: hops_from_landmark + 1,
                     from: self.id()
                 }
             }
             Fate::Live
         }
     }}
}

#[derive(Compact, Clone)]
pub struct ShareRoutes{
    new_routes: CDict<Destination, f32>,
    from: ID
}

impl Recipient<ShareRoutes> for Lane {
    fn receive(&mut self, msg: &ShareRoutes) -> Fate {match *msg{
        ShareRoutes{ref new_routes, from} => {
            if let Some(from_interaction_idx) = self.interactions.iter().position(|interaction| interaction.partner_lane == from) {
                for (&destination, &new_distance) in new_routes.pairs() {
                    let insert = self.pathfinding_info.routes.get(destination).map(
                        |&RoutingInfo{distance, ..}| new_distance < distance
                    ).unwrap_or(true);
                    if insert {
                        self.pathfinding_info.routes.insert(destination, RoutingInfo{
                            distance: new_distance,
                            outgoing_idx: from_interaction_idx as u8
                        });
                    }
                }
            } else {
                println!("{:?} not yet connected to {:?}", self.id(), from);
            }
            Fate::Live
        }
    }}
}

impl Recipient<ShareRoutes> for TransferLane {
    fn receive(&mut self, msg: &ShareRoutes) -> Fate {match *msg{
        ShareRoutes{ref new_routes, from} => {
            let other_side = if from == self.left.expect("should have a left lane").0 {
                self.right.expect("should have a right lane").0
            } else {
                self.left.expect("should have a left lane").0
            };
            other_side << ShareRoutes{
                new_routes: new_routes.clone(),
                from: self.id()
            };
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
pub struct QueryAsDestination{requester: ID}
#[derive(Copy, Clone)]
pub struct TellAsDestination{id: ID, as_destination: Option<Destination>}

impl Recipient<QueryAsDestination> for Lane {
    fn receive(&mut self, msg: &QueryAsDestination) -> Fate {match *msg{
        QueryAsDestination{requester} => {
            requester << TellAsDestination{
                id: self.id(),
                as_destination: self.pathfinding_info.as_destination
            };
            Fate::Live
        }
    }}
}

pub fn setup(system: &mut ActorSystem) {
    system.add_inbox::<JoinLandmark, Swarm<Lane>>();
    system.add_inbox::<JoinLandmark, Swarm<TransferLane>>();
    system.add_inbox::<ShareRoutes, Swarm<Lane>>();
    system.add_inbox::<ShareRoutes, Swarm<TransferLane>>();
    system.add_inbox::<QueryAsDestination, Swarm<Lane>>();

    trip::setup(system);
}