use ::kay::{ActorSystem, Recipient, ID, Individual, Fate};

#[derive(Copy, Clone)]
pub struct Tick{pub dt: f32}

pub struct Simulation{
    simulatables: Vec<ID>
}

impl Individual for Simulation {}

impl Recipient<Tick> for Simulation {
    fn receive(&mut self, msg: &Tick) -> Fate {match *msg{
        Tick{dt} => {
            for simulatable in &self.simulatables {
                *simulatable << Tick{dt: dt};
            }
            Fate::Live
        }
    }}
}

pub fn setup(system: &mut ActorSystem, simulatables: Vec<ID>) {
    system.add_individual(Simulation{simulatables: simulatables});
    system.add_inbox::<Tick, Simulation>();
}