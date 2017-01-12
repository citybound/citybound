use ::kay::{ActorSystem, Recipient, ID, Individual, Fate};

#[derive(Copy, Clone)]
pub struct Tick {
    pub dt: f32,
    pub current_tick: usize,
}

pub struct Simulation {
    simulatables: Vec<ID>,
    current_tick: usize,
}

impl Individual for Simulation {}

impl Recipient<Tick> for Simulation {
    fn receive(&mut self, msg: &Tick) -> Fate {
        match *msg {
            Tick { dt, .. } => {
                for simulatable in &self.simulatables {
                    *simulatable <<
                    Tick {
                        dt: dt,
                        current_tick: self.current_tick,
                    };
                }
                self.current_tick += 1;
                Fate::Live
            }
        }
    }
}

pub fn setup(system: &mut ActorSystem, simulatables: Vec<ID>) {
    system.add_individual(Simulation {
        simulatables: simulatables,
        current_tick: 0,
    });
    system.add_inbox::<Tick, Simulation>();
}
