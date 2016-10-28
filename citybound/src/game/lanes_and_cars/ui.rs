use compass::{FiniteCurve};
use kay::{ID, Recipient, World, ActorSystem, InMemory, Swarm};
use monet::{WorldPosition, RenderToScene, SetupInScene, AddBatch, AddInstance};
use super::Lane;

#[path = "./resources/car.rs"]
mod car;

recipient!(Swarm<Lane>, (&mut self, world: &mut World, self_id: ID) {
    SetupInScene: &SetupInScene{renderer_id, scene_id} => {
        world.send(renderer_id, AddBatch::new(scene_id, 0, car::create()));
    }
});

recipient!(Lane, (&mut self, world: &mut World, self_id: ID) {
    RenderToScene: &RenderToScene{renderer_id, scene_id} => {
        for car in &self.cars {
            let position2d = self.path.along(car.position);
            world.send(renderer_id, AddInstance{
                scene_id: scene_id,
                batch_id: 0,
                position: WorldPosition{world_position: [position2d.x, position2d.y, 0.0]}
            });
        }
    }
});

pub fn setup(system: &mut ActorSystem) {
    system.add_inbox::<SetupInScene, Lane>(InMemory("setup_in_scene", 512, 4));
    system.add_inbox::<RenderToScene, Lane>(InMemory("render_to_scene", 512, 4));
}