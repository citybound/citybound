use compact::CVec;
use descartes::N;
use super::{CarLaneID, CarSwitchLaneID, SidewalkID};
use transport::microtraffic::{ObstacleContainerID};

#[derive(Compact, Clone)]
pub struct ConnectivityInfo {
    pub interactions: CVec<CarLaneInteraction>,
    pub on_intersection: bool,
}

impl ConnectivityInfo {
    pub fn new(on_intersection: bool) -> Self {
        ConnectivityInfo {
            interactions: CVec::new(),
            on_intersection,
        }
    }
}

#[derive(Compact, Clone, Default)]
pub struct CarSwitchConnectivityInfo {
    pub left: Option<(CarLaneID, N, N)>,
    pub right: Option<(CarLaneID, N, N)>,
    pub left_distance_map: CVec<(N, N)>,
    pub right_distance_map: CVec<(N, N)>,
}

#[derive(Copy, Clone, Debug)]
pub enum CarLaneInteraction {
    Previous {
        previous: CarLaneID,
        previous_length: N,
    },
    Next {
        next: CarLaneID,
        green: bool,
    },
    Conflicting {
        conflicting: ObstacleContainerID,
        start: N,
        conflicting_start: N,
        end: N,
        conflicting_end: N,
        can_weave: bool,
    },
    Switch {
        via: CarSwitchLaneID,
        to: CarLaneID,
        is_left: bool,
        start: N,
        end: N,
    },
}

impl CarLaneInteraction {
    pub fn direct_partner(&self) -> ObstacleContainerID {
        match self {
            CarLaneInteraction::Previous { previous, .. } => (*previous).into(),
            CarLaneInteraction::Next { next, .. } => (*next).into(),
            CarLaneInteraction::Conflicting { conflicting, .. } => (*conflicting).into(),
            CarLaneInteraction::Switch { via, .. } => (*via).into(),
        }
    }

    pub fn indirect_lane_partner(&self) -> ObstacleContainerID {
        match self {
            CarLaneInteraction::Previous { previous, .. } => (*previous).into(),
            CarLaneInteraction::Next { next, .. } => (*next).into(),
            CarLaneInteraction::Conflicting { conflicting, .. } => *conflicting,
            CarLaneInteraction::Switch { to, .. } => (*to).into(),
        }
    }

    pub fn direct_lane_partner(&self) -> Option<ObstacleContainerID> {
        match self {
            CarLaneInteraction::Previous { previous, .. } => Some((*previous).into()),
            CarLaneInteraction::Next { next, .. } => Some((*next).into()),
            CarLaneInteraction::Conflicting { conflicting, .. } => Some(*conflicting),
            CarLaneInteraction::Switch { .. } => None,
        }
    }

    pub fn direct_switch_partner(&self) -> Option<CarSwitchLaneID> {
        match self {
            CarLaneInteraction::Switch { via, .. } => Some(*via),
            _ => None,
        }
    }
}

#[derive(Compact, Clone)]
pub struct SidewalkConnectivityInfo {
    pub interactions: CVec<SidewalkInteraction>,
    pub on_intersection: bool,
}

impl SidewalkConnectivityInfo {
    pub fn new(on_intersection: bool) -> Self {
        SidewalkConnectivityInfo {
            interactions: CVec::new(),
            on_intersection,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SidewalkInteraction {
    Previous {
        previous: SidewalkID,
    },
    Next {
        next: SidewalkID,
        green: bool,
    },
    Conflicting {
        conflicting: CarLaneID,
        start: N,
        conflicting_start: N,
        end: N,
        conflicting_end: N,
    },
}
