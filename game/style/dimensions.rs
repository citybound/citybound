use descartes::N;

pub const CONTROL_POINT_HANDLE_RADIUS: N = 1.5;

pub const LANE_WIDTH: N = 3.9;
pub const LANE_DISTANCE: N = 0.8 * LANE_WIDTH;
pub const CENTER_LANE_DISTANCE: N = LANE_DISTANCE * 1.1;
pub const LANE_MARKER_WIDTH: N = 0.3; // this is unrealistic, but increases visibility
pub const LANE_MARKER_DASH_GAP: N = 3.0;
pub const LANE_MARKER_DASH_LENGTH: N = 2.0;

pub const LOT_OUTLINE_WIDTH: N = 0.2;
