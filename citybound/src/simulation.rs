use ::kay::{ActorSystem, Message, Recipient, Known, ID, World, InMemory};

#[derive(Copy, Clone)]
pub struct Tick;
message!(Tick, ::type_ids::Messages::Tick);

#[derive(Copy, Clone)]
pub struct AddSimulatable(pub ID);
message!(AddSimulatable, ::type_ids::Messages::AddSimulatable);

struct Simulation{
    simulatables: Vec<ID>
}

recipient!(Simulation, (&mut self, world: &mut World, self_id: ID) {
    Tick: _ => {
        for simulatable in &self.simulatables {
            world.send(*simulatable, Tick);
        }
    },

    AddSimulatable: &AddSimulatable(id) => {
        self.simulatables.push(id);
    }
});

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(Simulation{
        simulatables: Vec::new()
    }, ::type_ids::Recipients::Simulation as usize);
    system.add_individual_inbox::<Tick, Simulation>(InMemory("tick", 512, 1), ::type_ids::Recipients::Simulation as usize);
    system.add_individual_inbox::<AddSimulatable, Simulation>(InMemory("add_simulatable", 512, 64), ::type_ids::Recipients::Simulation as usize);
}