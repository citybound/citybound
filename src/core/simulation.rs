use kay::{ActorSystem, ID, Fate};

#[derive(Copy, Clone)]
pub struct Tick {
    pub dt: f32,
    pub current_tick: usize,
}

pub struct Simulation {
    simulatables: Vec<ID>,
    current_tick: usize,
}

pub fn setup(system: &mut ActorSystem, simulatables: Vec<ID>) {
    let initial = Simulation {
        simulatables: simulatables,
        current_tick: 0,
    };
    system.add(initial, |mut the_simulation| {
        the_simulation.on(|&Tick { dt, .. }, sim, world| {
            for simulatable in &sim.simulatables {
                world.send(*simulatable,
                           Tick { dt: dt, current_tick: sim.current_tick });
            }
            sim.current_tick += 1;
            Fate::Live
        })
    });
}

#[derive(Copy, Clone)]
pub struct TimeOfDay {
    minutes_since_midnight: u16,
}

impl TimeOfDay {
    pub fn new(h: usize, m: usize) -> Self {
        TimeOfDay { minutes_since_midnight: m as u16 + (h * 60) as u16 }
    }

    pub fn hours_minutes(&self) -> (usize, usize) {
        ((self.minutes_since_midnight / 60) as usize, (self.minutes_since_midnight % 60) as usize)
    }
}