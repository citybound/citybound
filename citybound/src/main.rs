#![allow(dead_code)]
extern crate world_record;
extern crate monet;

use std::path::PathBuf;
use world_record::{FutureState, FutureRecordCollection, GrowableBuffer};
use std::time::{Duration, Instant};
use std::thread;
use std::sync::mpsc::channel;

mod models;

pub struct State {
    core: GrowableBuffer<models::Core, u32>,
    cars: FutureRecordCollection<models::Car>,
    lanes: FutureRecordCollection<models::Lane>,
    lane_connections: FutureRecordCollection<models::LaneConnection>,
    lane_overlaps: FutureRecordCollection<models::LaneOverlap>,
    lane_overlap_groups: FutureRecordCollection<models::LaneOverlapGroup>,
    intersections: FutureRecordCollection<models::Intersection>,
    lane_plan_entries: FutureRecordCollection<models::LanePlanEntry>,
    plans: FutureRecordCollection<models::Plan>
}

mod steps;

impl State {
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
            plans: FutureRecordCollection::new(path.join("plans"))
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

struct TimingInfo {
    target_ticks_per_second: u32,
    last_tick: Instant,
}

type SimulationStep = Box<Fn(&State, &mut State) -> ()>;
type SimulationListener = Box<Fn(&State, &State) -> ()>;

struct Simulation {
    a: State,
    b: State,
    past_is_a: bool,
    timing_info: TimingInfo,
    save_after_next_step: bool,
    steps: Vec<SimulationStep>,
    listeners: Vec<SimulationListener>
}

impl Simulation {
    pub fn new (path: PathBuf, steps: Vec<SimulationStep>, listeners: Vec<SimulationListener>) -> Simulation {
        Simulation{
            a: State::new(path.join("a")),
            b: State::new(path.join("b")),
            past_is_a: true,
            timing_info: TimingInfo {
                target_ticks_per_second: 480,
                last_tick: Instant::now()
            },
            save_after_next_step: false,
            steps: steps,
            listeners: listeners
        }
    }
    
    pub fn step(&mut self) {
        {
            let (past, future) = if self.past_is_a {(&self.a, &mut self.b)}
                                else {(&self.b, &mut self.a)};
                                
            println!("simulation step (past #{})!", past.core.header.ticks);
            
            for step in &self.steps {
                step(past, future);
            }
            
            for listener in &self.listeners {
                listener(past, future);
            }
            
            future.materialize();
            
            if self.save_after_next_step {
                //future.flush();
                self.save_after_next_step = false;
            }
        }
        
        let (mutable_past, fresh_future) = if self.past_is_a {(&mut self.a, &self.b)}
                                           else {(&mut self.b, &self.a)};
        mutable_past.overwrite_with(fresh_future);
        
        self.past_is_a = !self.past_is_a;
        
        let target_step_duration = Duration::new(0, 1_000_000_000 / self.timing_info.target_ticks_per_second);
        let elapsed = self.timing_info.last_tick.elapsed();
        let duration_to_sleep = if elapsed < target_step_duration {target_step_duration - elapsed}
                                else {Duration::new(0, 0)};
        self.timing_info.last_tick = Instant::now();
        thread::sleep(duration_to_sleep);
    }
    
    pub fn save_soon(&mut self) {
        self.save_after_next_step = true;
    }
}

fn main() {
    let (to_simulation, from_renderer) = channel::<()>();
    let (to_renderer, from_simulation) = channel::<()>();
    
    let renderer_listener = move |past: &State, future: &State| {
        match from_renderer.try_recv() {
            Ok(_) => {
                println!("creating renderer state...");
                to_renderer.send(()).unwrap();
            },
            Err(_) => {}
        };
        
    };
    
    thread::Builder::new().name("simulation".to_string()).spawn(|| {
        let mut simulation = Simulation::new(
            PathBuf::from("savegames/dev"),
            vec! [Box::new(steps::tick)],
            vec! [Box::new(renderer_listener)]
        );
    
       loop {
           simulation.step();
       }
    }).unwrap();
    
    monet::main_loop(to_simulation, from_simulation);
}