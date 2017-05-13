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
