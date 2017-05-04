use kay::{Recipient, ID, Actor, Fate};

#[derive(Copy, Clone)]
pub struct Tick {
    pub dt: f32,
    pub current_tick: usize,
}

pub struct Simulation {
    simulatables: Vec<ID>,
    current_tick: usize,
}

impl Actor for Simulation {}

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

pub fn setup(simulatables: Vec<ID>) {
    Simulation::register_with_state(Simulation {
                                        simulatables: simulatables,
                                        current_tick: 0,
                                    });
    Simulation::handle::<Tick>();
}
