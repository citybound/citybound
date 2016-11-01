use ::kay::{ActorSystem, Recipient, ID, World, Individual};

#[derive(Copy, Clone)]
pub struct Tick{pub dt: f32}

pub struct Simulation{
    simulatables: Vec<ID>
}

impl Individual for Simulation {}

impl Recipient<Tick> for Simulation {
    fn react_to(&mut self, msg: &Tick, world: &mut World, _self_id: ID) {match msg{
        &Tick{dt} => {
            for simulatable in &self.simulatables {
                world.send(*simulatable, Tick{dt: dt});
            }
        }
    }}
}

pub fn setup(system: &mut ActorSystem, simulatables: Vec<ID>) {
    system.add_individual(Simulation{simulatables: simulatables});
    system.add_individual_inbox::<Tick, Simulation>();
}