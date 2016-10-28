use ::kay::{ActorSystem, Recipient, ID, World, InMemory};

#[derive(Copy, Clone)]
pub struct Tick{pub dt: f32}

#[derive(Copy, Clone)]
pub struct AddSimulatable(pub ID);

pub struct Simulation{
    simulatables: Vec<ID>
}

recipient!(Simulation, (&mut self, world: &mut World, self_id: ID) {
    Tick: &Tick{dt} => {
        for simulatable in &self.simulatables {
            world.send(*simulatable, Tick{dt: dt});
        }
    },

    AddSimulatable: &AddSimulatable(id) => {
        self.simulatables.push(id);
    }
});

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(Simulation{simulatables: Vec::new()});
    system.add_individual_inbox::<Tick, Simulation>(InMemory("tick", 512, 1));
    system.add_individual_inbox::<AddSimulatable, Simulation>(InMemory("add_simulatable", 512, 64));
}