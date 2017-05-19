use kay::{ActorSystem, ID, Fate};

#[derive(Copy, Clone)]
pub struct DoTick;

const SECONDS_PER_TICK: f32 = 1.0 / 20.0;

#[derive(Copy, Clone)]
pub struct Tick {
    pub dt: f32,
    pub current_tick: Timestamp,
}

#[derive(Copy, Clone)]
pub struct Wake {
    pub current_tick: Timestamp,
}

#[derive(Copy, Clone)]
pub struct Timestamp(pub usize);

#[derive(Copy, Clone)]
pub struct Duration(pub usize);

#[derive(Copy, Clone)]
pub struct WakeUpIn(pub Duration, pub ID);

pub struct Simulation {
    simulatables: Vec<ID>,
    current_tick: Timestamp,
    sleepers: Vec<(Timestamp, ID)>,
}

pub fn setup(system: &mut ActorSystem, simulatables: Vec<ID>) {
    let initial = Simulation {
        simulatables: simulatables,
        current_tick: Timestamp(0),
        sleepers: Vec::new(),
    };
    system.add(initial, |mut the_simulation| {
        the_simulation.on(|_: &DoTick, sim, world| {
            for simulatable in &sim.simulatables {
                world.send(*simulatable,
                           Tick {
                               dt: SECONDS_PER_TICK,
                               current_tick: sim.current_tick,
                           });
            }
            while sim.sleepers
                      .last()
                      .map(|&(end, _)| end.0 < sim.current_tick.0)
                      .unwrap_or(false) {
                let (_, id) = sim.sleepers
                    .pop()
                    .expect("just checked that there are sleepers");
                world.send(id, Wake { current_tick: sim.current_tick });
            }
            sim.current_tick.0 += 1;
            Fate::Live
        });

        the_simulation.on(|&WakeUpIn(remaining_ticks, sleeper_id), sim, _| {
            let wake_up_at = Timestamp(sim.current_tick.0 + remaining_ticks.0);
            let maybe_idx =
                sim.sleepers
                    .binary_search_by_key(&(wake_up_at.0 as isize), |&(t, _)| -(t.0 as isize));
            let insert_idx = match maybe_idx {
                Ok(idx) | Err(idx) => idx,
            };
            sim.sleepers.insert(insert_idx, (wake_up_at, sleeper_id));
            Fate::Live
        });
    });
}

#[derive(Copy, Clone)]
pub struct TimeOfDay {
    minutes_since_midnight: u16,
}

const TICKS_PER_MINUTE: usize = 60;

impl TimeOfDay {
    pub fn new(h: usize, m: usize) -> Self {
        TimeOfDay { minutes_since_midnight: m as u16 + (h * 60) as u16 }
    }

    pub fn from_tick(current_tick: Timestamp) -> Self {
        TimeOfDay { minutes_since_midnight: (current_tick.0 / TICKS_PER_MINUTE) as u16 }
    }

    pub fn hours_minutes(&self) -> (usize, usize) {
        ((self.minutes_since_midnight / 60) as usize, (self.minutes_since_midnight % 60) as usize)
    }
}