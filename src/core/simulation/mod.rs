use kay::{ActorSystem, ID, Fate};

mod time;

pub use self::time::{Timestamp, DurationTicks, DurationSeconds, TICKS_PER_SIM_MINUTE,
                     TICKS_PER_SIM_SECOND, TimeOfDay};

#[derive(Copy, Clone)]
pub struct DoTick;

#[derive(Copy, Clone)]
pub struct Tick {
    pub dt: f32, // in seconds
    pub current_tick: Timestamp,
}

#[derive(Copy, Clone)]
pub struct Wake {
    pub current_tick: Timestamp,
}

#[derive(Copy, Clone)]
pub struct WakeUpIn(pub DurationTicks, pub ID);

pub struct Simulation {
    simulatables: Vec<ID>,
    current_tick: Timestamp,
    sleepers: Vec<(Timestamp, ID)>,
}

pub fn setup(system: &mut ActorSystem, simulatables: Vec<ID>) {
    let initial = Simulation {
        simulatables: simulatables,
        current_tick: Timestamp::new(0),
        sleepers: Vec::new(),
    };
    system.add(initial, |mut the_simulation| {
        the_simulation.on(|_: &DoTick, sim, world| {
            for simulatable in &sim.simulatables {
                world.send(*simulatable,
                           Tick {
                               dt: 1.0 / (TICKS_PER_SIM_SECOND as f32),
                               current_tick: sim.current_tick,
                           });
            }
            while sim.sleepers
                      .last()
                      .map(|&(end, _)| end < sim.current_tick)
                      .unwrap_or(false) {
                let (_, id) = sim.sleepers
                    .pop()
                    .expect("just checked that there are sleepers");
                world.send(id, Wake { current_tick: sim.current_tick });
            }
            sim.current_tick += DurationTicks::new(1);
            Fate::Live
        });

        the_simulation.on(|&WakeUpIn(remaining_ticks, sleeper_id), sim, _| {
            let wake_up_at = sim.current_tick + remaining_ticks;
            let maybe_idx =
                sim.sleepers
                    .binary_search_by_key(&wake_up_at.iticks(), |&(t, _)| -(t.iticks()));
            let insert_idx = match maybe_idx {
                Ok(idx) | Err(idx) => idx,
            };
            sim.sleepers.insert(insert_idx, (wake_up_at, sleeper_id));
            Fate::Live
        });
    });
}