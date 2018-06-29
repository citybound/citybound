use compact::{CVec, COption};
use kay::{ActorSystem, World, Actor};
use descartes::{N, Band, LinePath};

use super::construction::ConstructionInfo;
pub mod connectivity;
use self::connectivity::{ConnectivityInfo, TransferConnectivityInfo};
use super::microtraffic::{Microtraffic, TransferringMicrotraffic};
use super::pathfinding::PathfindingInfo;
use stagemaster::{UserInterface, Event3d, Interactable3d, Interactable3dID};

#[derive(Compact, Clone)]
pub struct Lane {
    pub id: LaneID,
    pub construction: ConstructionInfo,
    pub connectivity: ConnectivityInfo,
    pub microtraffic: Microtraffic,
    pub pathfinding: PathfindingInfo,
}

impl Lane {
    #[cfg_attr(feature = "cargo-clippy", allow(eq_op))]
    pub fn spawn(
        id: LaneID,
        path: &LinePath,
        on_intersection: bool,
        timings: &CVec<bool>,
        world: &mut World,
    ) -> Self {
        let lane = Lane {
            id,
            construction: ConstructionInfo::from_path(path.clone()),
            connectivity: ConnectivityInfo::new(on_intersection),
            microtraffic: Microtraffic::new(timings.clone()),
            pathfinding: PathfindingInfo::default(),
        };

        super::rendering::on_build(&lane, world);

        if super::pathfinding::DEBUG_VIEW_CONNECTIVITY
            || super::pathfinding::trip::DEBUG_MANUALLY_SPAWN_CARS
        {
            UserInterface::local_first(world).add(
                ::ui_layers::UILayer::Debug as usize,
                id.into(),
                COption(Some(Band::new(path.clone(), 3.0).as_area())),
                5,
                world,
            );
        }

        lane
    }
}

#[derive(Compact, Clone)]
pub struct SwitchLane {
    pub id: SwitchLaneID,
    pub construction: ConstructionInfo,
    pub connectivity: TransferConnectivityInfo,
    pub microtraffic: TransferringMicrotraffic,
}

impl SwitchLane {
    pub fn spawn(id: SwitchLaneID, path: &LinePath, _: &mut World) -> SwitchLane {
        SwitchLane {
            id,
            construction: ConstructionInfo::from_path(path.clone()),
            connectivity: TransferConnectivityInfo::default(),
            microtraffic: TransferringMicrotraffic::default(),
        }
    }

    pub fn other_side(&self, side: LaneID) -> Option<LaneID> {
        if let Some((left, _)) = self.connectivity.left {
            if side == left {
                return self.connectivity.right.map(|(right, _)| right);
            }
        };
        if let Some((right, _)) = self.connectivity.right {
            if side == right {
                return self.connectivity.left.map(|(left, _)| left);
            }
        };
        None
    }

    #[cfg_attr(feature = "cargo-clippy", allow(needless_range_loop))]
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

    #[cfg_attr(feature = "cargo-clippy", allow(needless_range_loop))]
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

impl Interactable3d for Lane {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        match event {
            Event3d::HoverStarted { .. } => self.start_debug_connectivity(world),
            Event3d::HoverStopped { .. } => self.stop_debug_connectivity(world),
            Event3d::DragFinished { .. } => self.manually_spawn_car_add_lane(world),
            _ => {}
        };
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Lane>();
    system.register::<SwitchLane>();

    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
