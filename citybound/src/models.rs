use world_record::{ID, FutureState, FutureRecordCollection, GrowableBuffer};
use std::path::PathBuf;
use std::f32::consts::PI;
use nalgebra::{Point2, Vector1, Vector2, Vector3, Rotation3, Rotation2, Rotate};
// use std::ops::Range;

#[derive(Default, Clone)]
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
    start: Point2<f32>,
    direction: Point2<f32>,
    end: Point2<f32>,
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

#[derive(Copy, Clone)]
pub struct Eye {
    pub target: Point2<f32>,
    pub azimuth: f32,
    pub inclination: f32,
    pub distance: f32
}

impl Eye {
    pub fn direction(&self) -> Vector3<f32> {
        let inclined_vector = Rotation3::<f32>::new(Vector3::x() * -self.inclination).rotate(&Vector3::<f32>::y());
        Rotation3::new(Vector3::z() * self.azimuth).rotate(&inclined_vector)
    }

    pub fn direction_2d(&self) -> Vector2<f32> {
        Rotation2::new(Vector1::new(-self.azimuth)).rotate(&Vector2::y())
    }

    pub fn right_direction_2d(&self) -> Vector2<f32> {
        Rotation2::new(Vector1::new(-PI / 2.0)).rotate(&self.direction_2d())
    }
}

#[derive(Copy, Clone)]
pub struct UIState {
    pub eye: Eye
}

pub struct State {
    pub core: GrowableBuffer<Core, u32>,
    pub cars: FutureRecordCollection<Car>,
    pub lanes: FutureRecordCollection<Lane>,
    pub lane_connections: FutureRecordCollection<LaneConnection>,
    pub lane_overlaps: FutureRecordCollection<LaneOverlap>,
    pub lane_overlap_groups: FutureRecordCollection<LaneOverlapGroup>,
    pub intersections: FutureRecordCollection<Intersection>,
    pub lane_plan_entries: FutureRecordCollection<LanePlanEntry>,
    pub plans: FutureRecordCollection<Plan>,
    pub ui_state: UIState
}

impl FutureState for State {
     fn new(path: PathBuf) -> State {
        return State {
            core: GrowableBuffer::new(path.join("core")),
            cars: FutureRecordCollection::new(path.join("cars")),
            lanes: FutureRecordCollection::new(path.join("lanes")),
            lane_connections: FutureRecordCollection::new(path.join("lane_connections")),
            lane_overlaps: FutureRecordCollection::new(path.join("lane_overlaps")),
            lane_overlap_groups: FutureRecordCollection::new(path.join("lane_overlap_groups")),
            intersections: FutureRecordCollection::new(path.join("intersections")),
            lane_plan_entries: FutureRecordCollection::new(path.join("lane_plan_entries")),
            plans: FutureRecordCollection::new(path.join("plans")),
            ui_state: UIState{
                eye: Eye{
                    target: Point2::new(0.0, 0.0),
                    azimuth: PI / 4.0,
                    inclination: PI / 4.0,
                    distance: 7.0
                }
            }
        }
    }
    
    fn overwrite_with(&mut self, other: &Self) {
        self.core.overwrite_with(&other.core);
        self.cars.overwrite_with(&other.cars);
        self.lanes.overwrite_with(&other.lanes);
        self.lane_connections.overwrite_with(&other.lane_connections);
        self.lane_overlaps.overwrite_with(&other.lane_overlaps);
        self.lane_overlap_groups.overwrite_with(&other.lane_overlap_groups);
        self.intersections.overwrite_with(&other.intersections);
        self.lane_plan_entries.overwrite_with(&other.lane_plan_entries);
        self.plans.overwrite_with(&other.plans);
        self.ui_state = other.ui_state;
    }
    
    fn materialize(&mut self) {
        self.cars.materialize();
        self.lanes.materialize();
        self.lane_connections.materialize();
        self.lane_overlaps.materialize();
        self.lane_overlap_groups.materialize();
        self.intersections.materialize();
        self.lane_plan_entries.materialize();
        self.plans.materialize();
    }
}