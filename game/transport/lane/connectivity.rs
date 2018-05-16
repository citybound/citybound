use compact::CVec;
use descartes::N;
use super::LaneID;

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

use super::super::microtraffic::LaneLikeID;

#[derive(Compact, Clone, Default)]
pub struct TransferConnectivityInfo {
    pub left: Option<(LaneID, f32)>,
    pub right: Option<(LaneID, f32)>,
    pub left_distance_map: CVec<(N, N)>,
    pub right_distance_map: CVec<(N, N)>,
}

#[derive(Copy, Clone)]
pub struct Interaction {
    pub partner_lane: LaneLikeID,
    pub start: f32,
    pub partner_start: f32,
    pub kind: InteractionKind,
}

#[derive(Copy, Clone, Debug)]
pub enum InteractionKind {
    Overlap {
        end: f32,
        partner_end: f32,
        kind: OverlapKind,
    },
    Next { green: bool },
    Previous,
}

#[derive(Copy, Clone, Debug)]
pub enum OverlapKind {
    Parallel,
    Transfer,
    Conflicting,
}
