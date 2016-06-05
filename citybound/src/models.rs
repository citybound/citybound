use world_record::{ID};
// use std::ops::Range;

#[derive(Default)]
pub struct Core {
    pub ticks: u64,
    pub time: f64
}

// TODO: make this work
pub struct MyRange<T> {
    pub start: T,
    pub end: T
}

// TODO: move this to world_record
type LinkedList<T> = Option<ID<T>>;

pub struct Car {
    location: LanePosition,
    velocity: f32,
    acceleration: f32,
    destination: LanePosition
}

pub struct Lane {
    start: [f32; 2],
    direction: [f32; 2],
    end: [f32; 2],
    next: LinkedList<LaneConnection>,
    previous: LinkedList<LaneConnection>,
    first_overlap: LinkedList<LaneOverlap>
}

pub struct LaneConnection {
    lane: ID<Lane>,
    offset: f32,
    next_lane_connection: LinkedList<LaneConnection>
}

enum LaneOverlapRole {
    Parallel,
    ParallelMerging,
    ParallelOpposing,
    Crossing,
    Opposing
}

// note: one overlap per segment/segment for easy linear mapping!
pub struct LaneOverlap {
    role: LaneOverlapRole,
    range: MyRange<f32>,
    range_there: MyRange<f32>,
    next_overlap_of_lane: LinkedList<LaneOverlap>,
    next_overlap_of_group: LinkedList<LaneOverlap>
}

pub struct LaneOverlapGroup {
    first_overlap: LinkedList<LaneOverlap>,
    next_group_of_intersection: LinkedList<LaneOverlapGroup>
}

pub struct Intersection {
    nonconflicting_groups: LinkedList<LaneOverlapGroup>
}

pub struct LanePosition {
    lane: ID<Lane>,
    position: f32
}

pub struct LanePlanEntry {
    lane: ID<Lane>,
    next_of_plan: ID<LanePlanEntry>
}

pub struct Plan {
    lanes_to_create: LinkedList<LanePlanEntry>,
    lanes_to_destroy: LinkedList<LanePlanEntry>
}