use ::kay::{ActorSystem, Recipient, ID, World, Individual};

#[derive(Copy, Clone)]
pub struct Tick{pub dt: f32}

pub struct Simulation{
    simulatables: Vec<ID>
}

impl Individual for Simulation {}

recipient!(Simulation, (&mut self, world: &mut World, self_id: ID) {
    Tick: &Tick{dt} => {
        for simulatable in &self.simulatables {
            world.send(*simulatable, Tick{dt: dt});
        }
    }
});

pub fn setup(system: &mut ActorSystem, simulatables: Vec<ID>) {
    system.add_individual(Simulation{simulatables: simulatables});
    system.add_individual_inbox::<Tick, Simulation>();
}