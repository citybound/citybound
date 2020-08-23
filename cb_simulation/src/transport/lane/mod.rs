use compact::{CVec};
use kay::{ActorSystem, World};
use descartes::{N, LinePath};

use super::construction::ConstructionInfo;
pub mod connectivity;
use self::connectivity::{ConnectivityInfo, CarSwitchConnectivityInfo, SidewalkConnectivityInfo};
use super::microtraffic::{CarMicrotraffic, CarSwitchMicrotraffic, SidewalkMicrotraffic};
use super::pathfinding::PathfindingCore;

#[derive(Compact, Clone)]
pub struct CarLane {
    pub id: CarLaneID,
    pub construction: ConstructionInfo,
    pub connectivity: ConnectivityInfo,
    pub microtraffic: CarMicrotraffic,
    pub pathfinding: PathfindingCore,
}

impl CarLane {
    pub fn spawn(
        id: CarLaneID,
        path: &LinePath,
        on_intersection: bool,
        timings: &CVec<bool>,
        world: &mut World,
    ) -> Self {
        let lane = CarLane {
            id,
            construction: ConstructionInfo::from_path(path.clone()),
            connectivity: ConnectivityInfo::new(on_intersection),
            microtraffic: CarMicrotraffic::new(timings.clone()),
            pathfinding: PathfindingCore::default(),
        };

        super::ui::on_build(&lane, world);

        lane
    }
}

#[derive(Compact, Clone)]
pub struct CarSwitchLane {
    pub id: CarSwitchLaneID,
    pub construction: ConstructionInfo,
    pub connectivity: CarSwitchConnectivityInfo,
    pub microtraffic: CarSwitchMicrotraffic,
}

impl CarSwitchLane {
    pub fn spawn(id: CarSwitchLaneID, path: &LinePath, _: &mut World) -> CarSwitchLane {
        CarSwitchLane {
            id,
            construction: ConstructionInfo::from_path(path.clone()),
            connectivity: CarSwitchConnectivityInfo::default(),
            microtraffic: CarSwitchMicrotraffic::default(),
        }
    }

    pub fn other_side(&self, side: CarLaneID) -> Option<CarLaneID> {
        if let Some((left, ..)) = self.connectivity.left {
            if side == left {
                return self.connectivity.right.map(|(right, ..)| right);
            }
        };
        if let Some((right, ..)) = self.connectivity.right {
            if side == right {
                return self.connectivity.left.map(|(left, ..)| left);
            }
        };
        None
    }

    #[allow(clippy::needless_range_loop)]
    pub fn interaction_to_self_offset(
        &self,
        distance_on_interaction: N,
        came_from_left: bool,
    ) -> N {
        let map = if came_from_left {
            &self.connectivity.left_distance_map
        } else {
            &self.connectivity.right_distance_map
        };

        for i in 0..map.len() {
            let (next_self, next_other) = map[i];
            let &(prev_self, prev_other) = i
                .checked_sub(1)
                .and_then(|p| map.get(p))
                .unwrap_or(&(0.0, 0.0));
            if prev_other <= distance_on_interaction && next_other >= distance_on_interaction {
                let amount_of_segment =
                    (distance_on_interaction - prev_other) / (next_other - prev_other);
                let distance_on_self = prev_self + amount_of_segment * (next_self - prev_self);
                return distance_on_self - distance_on_interaction;
            }
        }
        map.last().unwrap().0 - map.last().unwrap().1
    }

    #[allow(clippy::needless_range_loop)]
    pub fn self_to_interaction_offset(&self, distance_on_self: N, going_to_left: bool) -> N {
        let map = if going_to_left {
            &self.connectivity.left_distance_map
        } else {
            &self.connectivity.right_distance_map
        };

        for i in 0..map.len() {
            let (next_self, next_other) = map[i];
            let &(prev_self, prev_other) = i
                .checked_sub(1)
                .and_then(|p| map.get(p))
                .unwrap_or(&(0.0, 0.0));
            if prev_self <= distance_on_self && next_self >= distance_on_self {
                let amount_of_segment = (distance_on_self - prev_self) / (next_self - prev_self);
                let distance_on_other = prev_other + amount_of_segment * (next_other - prev_other);
                return distance_on_other - distance_on_self;
            }
        }
        map.last().unwrap().1 - map.last().unwrap().0
    }
}

#[derive(Compact, Clone)]
pub struct Sidewalk {
    pub id: SidewalkID,
    pub construction: ConstructionInfo,
    pub connectivity: SidewalkConnectivityInfo,
    pub microtraffic: SidewalkMicrotraffic,
    pub pathfinding: PathfindingCore,
}

impl Sidewalk {
    pub fn spawn(
        id: SidewalkID,
        path: &LinePath,
        on_intersection: bool,
        timings: &CVec<bool>,
        world: &mut World,
    ) -> Self {
        let sidewalk = Sidewalk {
            id,
            construction: ConstructionInfo::from_path(path.clone()),
            connectivity: SidewalkConnectivityInfo::new(on_intersection),
            microtraffic: SidewalkMicrotraffic::new(timings.clone()),
            pathfinding: PathfindingCore::default(),
        };

        super::ui::on_build_sidewalk(&sidewalk, world);

        sidewalk
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<CarLane>();
    system.register::<CarSwitchLane>();
    system.register::<Sidewalk>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
