use compact::CVec;
use kay::{ID, Actor};
use kay::swarm::Swarm;
use descartes::{N, FiniteCurve};
use stagemaster::geometry::CPath;

use super::construction::ConstructionInfo;
use super::connectivity::{ConnectivityInfo, TransferConnectivityInfo};
use super::microtraffic::{Microtraffic, TransferringMicrotraffic};
use super::pathfinding::PathfindingInfo;

#[derive(Compact, SubActor, Clone)]
pub struct Lane {
    _id: Option<ID>,
    pub construction: ConstructionInfo,
    pub connectivity: ConnectivityInfo,
    pub microtraffic: Microtraffic,
    pub pathfinding: PathfindingInfo,
    pub hovered: bool,
    pub last_spawn_position: N,
}

impl Lane {
    pub fn new(path: CPath, on_intersection: bool, timings: CVec<bool>) -> Self {
        Lane {
            _id: None,
            last_spawn_position: path.length() / 2.0,
            construction: ConstructionInfo::from_path(path),
            connectivity: ConnectivityInfo::new(on_intersection),
            microtraffic: Microtraffic::new(timings),
            pathfinding: PathfindingInfo::default(),
            hovered: false,
        }
    }
}

#[derive(Compact, SubActor, Clone)]
pub struct TransferLane {
    _id: Option<ID>,
    pub construction: ConstructionInfo,
    pub connectivity: TransferConnectivityInfo,
    pub microtraffic: TransferringMicrotraffic,
}

impl TransferLane {
    pub fn new(path: CPath) -> TransferLane {
        TransferLane {
            _id: None,
            construction: ConstructionInfo::from_path(path),
            connectivity: TransferConnectivityInfo::default(),
            microtraffic: TransferringMicrotraffic::default(),
        }
    }

    pub fn other_side(&self, side: ID) -> ID {
        if side == self.connectivity.left.expect("should have a left lane").0 {
            self.connectivity.right.expect("should have a right lane").0
        } else {
            self.connectivity.left.expect("should have a left lane").0
        }
    }

    pub fn interaction_to_self_offset(&self,
                                      distance_on_interaction: N,
                                      came_from_left: bool)
                                      -> N {
        let map = if came_from_left {
            &self.connectivity.left_distance_map
        } else {
            &self.connectivity.right_distance_map
        };
        #[allow(needless_range_loop)]
        for i in 0..map.len() {
            let (next_self, next_other) = map[i];
            let &(prev_self, prev_other) = map.get(i - 1).unwrap_or(&(0.0, 0.0));
            if prev_other <= distance_on_interaction && next_other >= distance_on_interaction {
                let amount_of_segment = (distance_on_interaction - prev_other) /
                                        (next_other - prev_other);
                let distance_on_self = prev_self + amount_of_segment * (next_self - prev_self);
                return distance_on_self - distance_on_interaction;
            }
        }
        map.last().unwrap().0 - map.last().unwrap().1
    }

    pub fn self_to_interaction_offset(&self, distance_on_self: N, going_to_left: bool) -> N {
        let map = if going_to_left {
            &self.connectivity.left_distance_map
        } else {
            &self.connectivity.right_distance_map
        };
        #[allow(needless_range_loop)]
        for i in 0..map.len() {
            let (next_self, next_other) = map[i];
            let &(prev_self, prev_other) = map.get(i - 1).unwrap_or(&(0.0, 0.0));
            if prev_self <= distance_on_self && next_self >= distance_on_self {
                let amount_of_segment = (distance_on_self - prev_self) / (next_self - prev_self);
                let distance_on_other = prev_other + amount_of_segment * (next_other - prev_other);
                return distance_on_other - distance_on_self;
            }
        }
        map.last().unwrap().1 - map.last().unwrap().0
    }
}

pub fn setup() {
    Swarm::<Lane>::register_default();
    Swarm::<TransferLane>::register_default();
}
