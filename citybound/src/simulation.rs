use world_record::{FutureState};
use std::time::{Duration, Instant};
use std::path::PathBuf;
use std::thread;

struct TimingInfo {
    target_ticks_per_second: u32,
    last_tick: Instant,
}

pub type SimulationStep<State> = Box<Fn(&State, &mut State) -> ()>;
pub type SimulationListener<State> = Box<Fn(&State, &State) -> ()>;

pub struct Simulation<State> {
    a: State,
    b: State,
    past_is_a: bool,
    timing_info: TimingInfo,
    save_after_next_step: bool,
    steps: Vec<SimulationStep<State>>,
    listeners: Vec<SimulationListener<State>>
}

impl <State: FutureState> Simulation<State> {
    pub fn new (path: PathBuf, steps: Vec<SimulationStep<State>>, listeners: Vec<SimulationListener<State>>) -> Simulation<State> {
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
    
    pub fn step(&mut self) -> Duration {
        {
            let (past, future) = if self.past_is_a {(&self.a, &mut self.b)}
                                else {(&self.b, &mut self.a)};
                                            
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
        return duration_to_sleep;
    }
    
    pub fn save_soon(&mut self) {
        self.save_after_next_step = true;
    }
}