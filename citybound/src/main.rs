#![allow(dead_code)]
extern crate world_record;
extern crate monet;

use std::path::PathBuf;
use world_record::{FutureState, FutureRecordCollection};

mod models;

struct State {
    cars: FutureRecordCollection<models::Car>,
    lanes: FutureRecordCollection<models::Lane>,
    lane_connections: FutureRecordCollection<models::LaneConnection>,
    lane_overlaps: FutureRecordCollection<models::LaneOverlap>,
    lane_overlap_groups: FutureRecordCollection<models::LaneOverlapGroup>,
    intersections: FutureRecordCollection<models::Intersection>,
    lane_plan_entries: FutureRecordCollection<models::LanePlanEntry>,
    plans: FutureRecordCollection<models::Plan>
}

impl State {
    fn new(path: PathBuf) -> State {
        return State {
            cars: FutureRecordCollection::new(path.join("cars")),
            lanes: FutureRecordCollection::new(path.join("lanes")),
            lane_connections: FutureRecordCollection::new(path.join("lane_connections")),
            lane_overlaps: FutureRecordCollection::new(path.join("lane_overlaps")),
            lane_overlap_groups: FutureRecordCollection::new(path.join("lane_overlap_groups")),
            intersections: FutureRecordCollection::new(path.join("intersections")),
            lane_plan_entries: FutureRecordCollection::new(path.join("lane_plan_entries")),
            plans: FutureRecordCollection::new(path.join("plans"))
        }
    }
}

impl FutureState for State {
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

struct Simulation {
    a: State,
    b: State,
    past_is_a: bool,
    save_after_next_step: bool,
    steps: Vec<fn(&State, &mut State) -> ()>,
    listeners: Vec<fn(&State, &State) -> ()>
}

impl Simulation {
    pub fn new (path: PathBuf) -> Simulation {
        Simulation{
            a: State::new(path.join("a")),
            b: State::new(path.join("b")),
            past_is_a: true,
            save_after_next_step: false,
            steps: Vec::new(),
            listeners: Vec::new()
        }
    }
    
    pub fn step(&mut self) {
        let (past, future) = if self.past_is_a {(&self.a, &mut self.b)}
                             else {(&self.b, &mut self.a)};
        
        for step in &self.steps {
            step(past, future);
        }
        
        for listener in &self.listeners {
            listener(past, future);
        }
        
        future.materialize();
        self.past_is_a = !self.past_is_a;
        
        if self.save_after_next_step {
            //past.flush();
            self.save_after_next_step = false;
        }
    }
    
    pub fn save_soon(&mut self) {
        self.save_after_next_step = true;
    }
}

fn main() {
    monet::main_loop();
}