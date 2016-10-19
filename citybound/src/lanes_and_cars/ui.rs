use compass::{FiniteCurve};
use kay::{ID, Recipient, World, ActorSystem, InMemory};
use monet::{WorldPosition};
use lanes_and_cars::Lane;

#[path = "./resources/car.rs"]
mod car;

recipient!(Lane, (&mut self, world: &mut World, self_id: ID) {
    ::ui::Render: &::ui::Render{render_manager_id} => {
        for car in &self.cars {
            let position2d = self.path.along(car.position);
            world.send(render_manager_id, ::ui::InstancePosition{
                batch_id: ::type_ids::RenderBatches::Cars as usize,
                position: WorldPosition{world_position: [position2d.x, position2d.y, 0.0]}
            });
        }
    }
});

pub fn setup(system: &mut ActorSystem) {
    system.add_inbox::<::ui::Render, Lane>(InMemory("render", 512, 4));
    system.world().send(ID::individual(::type_ids::Recipients::RenderManager as usize),
        ::ui::AddRenderable::new(
            ID::broadcast::<Lane>(),
            vec![(::type_ids::RenderBatches::Cars, car::create())]
        )
    );
}