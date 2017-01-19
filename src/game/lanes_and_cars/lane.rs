use compact::CVec;
use kay::{ID, Actor};
use kay::swarm::Swarm;
use descartes::{N, FiniteCurve};
use ::core::geometry::CPath;

use super::connectivity::Interaction;
use super::microtraffic::{Obstacle, LaneCar, TransferringLaneCar};

#[derive(Compact, SubActor, Clone)]
pub struct Lane {
    _id: Option<ID>,
    pub length: f32,
    pub path: CPath,
    pub interactions: CVec<Interaction>,
    pub obstacles: CVec<(Obstacle, ID)>,
    pub cars: CVec<LaneCar>,
    pub in_construction: f32,
    pub on_intersection: bool,
    pub timings: CVec<bool>,
    pub green: bool,
    pub yellow_to_green: bool,
    pub yellow_to_red: bool,
    pub pathfinding_info: super::pathfinding::PathfindingInfo,
    pub hovered: bool,
    pub unbuilding_for: Option<ID>,
    pub disconnects_remaining: u8,
    pub last_spawn_position: N,
}

impl Lane {
    pub fn new(path: CPath, on_intersection: bool, timings: CVec<bool>) -> Self {
        Lane {
            _id: None,
            length: path.length(),
            last_spawn_position: path.length() / 2.0,
            path: path,
            interactions: CVec::new(),
            obstacles: CVec::new(),
            cars: CVec::new(),
            in_construction: 0.0,
            on_intersection: on_intersection,
            timings: timings,
            green: false,
            yellow_to_green: false,
            yellow_to_red: false,
            pathfinding_info: super::pathfinding::PathfindingInfo::default(),
            hovered: false,
            unbuilding_for: None,
            disconnects_remaining: 0,
        }
    }
}

#[derive(Compact, SubActor, Clone)]
pub struct TransferLane {
    _id: Option<ID>,
    pub length: f32,
    pub path: CPath,
    pub left: Option<(ID, f32)>,
    pub right: Option<(ID, f32)>,
    pub left_obstacles: CVec<Obstacle>,
    pub right_obstacles: CVec<Obstacle>,
    pub left_distance_map: CVec<(N, N)>,
    pub right_distance_map: CVec<(N, N)>,
    pub cars: CVec<TransferringLaneCar>,
    pub in_construction: f32,
    pub unbuilding_for: Option<ID>,
    pub disconnects_remaining: u8,
}

impl TransferLane {
    pub fn new(path: CPath) -> TransferLane {
        TransferLane {
            _id: None,
            length: path.length(),
            path: path,
            left: None,
            right: None,
            left_obstacles: CVec::new(),
            right_obstacles: CVec::new(),
            left_distance_map: CVec::new(),
            right_distance_map: CVec::new(),
            cars: CVec::new(),
            in_construction: 0.0,
            unbuilding_for: None,
            disconnects_remaining: 0,
        }
    }

    pub fn other_side(&self, side: ID) -> ID {
        if side == self.left.expect("should have a left lane").0 {
            self.right.expect("should have a right lane").0
        } else {
            self.left.expect("should have a left lane").0
        }
    }

    pub fn interaction_to_self_offset(&self,
                                      distance_on_interaction: N,
                                      came_from_left: bool)
                                      -> N {
        let map = if came_from_left {
            &self.left_distance_map
        } else {
            &self.right_distance_map
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
            &self.left_distance_map
        } else {
            &self.right_distance_map
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