use compact::CVec;
use descartes::N;
use super::{LaneID, SwitchLaneID};
use transport::microtraffic::LaneLikeID;

#[derive(Compact, Clone)]
pub struct ConnectivityInfo {
    pub interactions: CVec<Interaction>,
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
pub struct SwitchConnectivityInfo {
    pub left: Option<(LaneID, N, N)>,
    pub right: Option<(LaneID, N, N)>,
    pub left_distance_map: CVec<(N, N)>,
    pub right_distance_map: CVec<(N, N)>,
}

#[derive(Copy, Clone, Debug)]
pub enum Interaction {
    Previous {
        previous: LaneID,
        previous_length: N,
    },
    Next {
        next: LaneID,
        green: bool,
    },
    Conflicting {
        conflicting: LaneID,
        start: N,
        conflicting_start: N,
        end: N,
        conflicting_end: N,
        can_weave: bool,
    },
    Switch {
        via: SwitchLaneID,
        to: LaneID,
        is_left: bool,
        start: N,
        end: N,
    },
}

impl Interaction {
    pub fn direct_partner(&self) -> LaneLikeID {
        match self {
            Interaction::Previous { previous, .. } => (*previous).into(),
            Interaction::Next { next, .. } => (*next).into(),
            Interaction::Conflicting { conflicting, .. } => (*conflicting).into(),
            Interaction::Switch { via, .. } => (*via).into(),
        }
    }

    pub fn indirect_lane_partner(&self) -> LaneID {
        match self {
            Interaction::Previous { previous, .. } => *previous,
            Interaction::Next { next, .. } => *next,
            Interaction::Conflicting { conflicting, .. } => *conflicting,
            Interaction::Switch { to, .. } => *to,
        }
    }

    pub fn direct_lane_partner(&self) -> Option<LaneID> {
        match self {
            Interaction::Previous { previous, .. } => Some(*previous),
            Interaction::Next { next, .. } => Some(*next),
            Interaction::Conflicting { conflicting, .. } => Some(*conflicting),
            Interaction::Switch { .. } => None,
        }
    }

    pub fn direct_switch_partner(&self) -> Option<SwitchLaneID> {
        match self {
            Interaction::Switch { via, .. } => Some(*via),
            _ => None,
        }
    }
}
