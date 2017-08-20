use kay::{ActorSystem, World, ID, Fate};

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
pub struct WakeUpIn(pub DurationTicks, pub SleeperID);

pub trait Sleeper {
    fn wake(&mut self, current_tick: Timestamp, world: &mut World);
}

pub struct Simulation {
    simulatables: Vec<ID>,
    current_tick: Timestamp,
    sleepers: Vec<(Timestamp, SleeperID)>,
}

use stagemaster::{UserInterface, AddDebugText};

pub fn setup(system: &mut ActorSystem, simulatables: Vec<ID>) {
    let initial = Simulation {
        simulatables: simulatables,
        current_tick: Timestamp::new(0),
        sleepers: Vec::new(),
    };
    system.add(initial, |mut the_simulation| {
        the_simulation.on(|_: &DoTick, sim, world| {
            for simulatable in &sim.simulatables {
                world.send(
                    *simulatable,
                    Tick {
                        dt: 1.0 / (TICKS_PER_SIM_SECOND as f32),
                        current_tick: sim.current_tick,
                    },
                );
            }
            while sim.sleepers
                .last()
                .map(|&(end, _)| end < sim.current_tick)
                .unwrap_or(false)
            {
                let (_, sleeper) = sim.sleepers.pop().expect(
                    "just checked that there are sleepers",
                );
                sleeper.wake(sim.current_tick, world);
            }
            sim.current_tick += DurationTicks::new(1);

            let time = TimeOfDay::from_tick(sim.current_tick).hours_minutes();

            world.send_to_id_of::<UserInterface, _>(AddDebugText {
                key: "Time".chars().collect(),
                text: format!("{:02}:{:02}", time.0, time.1).chars().collect(),
                color: [0.0, 0.0, 0.0, 1.0],
                persistent: false,
            });

            Fate::Live
        });

        the_simulation.on(|&WakeUpIn(remaining_ticks, sleeper_id), sim, _| {
            let wake_up_at = sim.current_tick + remaining_ticks;
            let maybe_idx = sim.sleepers.binary_search_by_key(
                &wake_up_at.iticks(),
                |&(t, _)| -(t.iticks()),
            );
            let insert_idx = match maybe_idx {
                Ok(idx) | Err(idx) => idx,
            };
            sim.sleepers.insert(insert_idx, (wake_up_at, sleeper_id));
            Fate::Live
        });
    });
}

mod kay_auto;
pub use self::kay_auto::*;
