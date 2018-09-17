use kay::{ActorSystem, World};
use compact::CVec;

mod time;

pub use self::time::{Instant, Ticks, Duration, TICKS_PER_SIM_MINUTE, TICKS_PER_SIM_SECOND,
TimeOfDay, TimeOfDayRange};

pub trait Simulatable {
    fn tick(&mut self, dt: f32, current_instant: Instant, world: &mut World);
}

pub trait Sleeper {
    fn wake(&mut self, current_instant: Instant, world: &mut World);
}

#[derive(Compact, Clone)]
pub struct Simulation {
    id: SimulationID,
    simulatables: CVec<SimulatableID>,
    current_instant: Instant,
    sleepers: CVec<(Instant, SleeperID)>,
    speed: u16,
}

impl Simulation {
    pub fn spawn(
        id: SimulationID,
        simulatables: &CVec<SimulatableID>,
        _: &mut World,
    ) -> Simulation {
        Simulation {
            id,
            simulatables: simulatables.clone(),
            current_instant: Instant::new(0),
            sleepers: CVec::new(),
            speed: 1,
        }
    }

    pub fn progress(&mut self, world: &mut World) {
        for _ in 0..self.speed {
            for simulatable in &self.simulatables {
                simulatable.tick(
                    1.0 / (TICKS_PER_SIM_SECOND as f32),
                    self.current_instant,
                    world,
                );
            }
            while self
                .sleepers
                .last()
                .map(|&(end, _)| end < self.current_instant)
                .unwrap_or(false)
            {
                let (_, sleeper) = self
                    .sleepers
                    .pop()
                    .expect("just checked that there are sleepers");
                sleeper.wake(self.current_instant, world);
            }
            self.current_instant += Ticks(1);
        }
    }

    pub fn wake_up_in(&mut self, remaining_ticks: Ticks, sleeper_id: SleeperID, _: &mut World) {
        let wake_up_at = self.current_instant + remaining_ticks;
        let maybe_idx = self
            .sleepers
            .binary_search_by_key(&wake_up_at.iticks(), |&(t, _)| -(t.iticks()));
        let insert_idx = match maybe_idx {
            Ok(idx) | Err(idx) => idx,
        };
        self.sleepers.insert(insert_idx, (wake_up_at, sleeper_id));
    }

    pub fn get_info(&mut self, requester: ::browser_ui::BrowserUIID, world: &mut World) {
        requester.on_simulation_info(self.current_instant, self.speed, world);
    }

    pub fn set_speed(&mut self, speed: u16, _world: &mut World) {
        self.speed = speed as u16;
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Simulation>();
    auto_setup(system);
}

pub fn spawn(world: &mut World, simulatables: Vec<SimulatableID>) -> SimulationID {
    SimulationID::spawn(simulatables.into(), world)
}

mod kay_auto;
pub use self::kay_auto::*;
