use compass::{FiniteCurve};
use kay::{ID, Recipient, World, ActorSystem, InMemory, Swarm};
use monet::{Instance, RenderToScene, SetupInScene, AddBatch, AddInstance, UpdateThing};
use core::geometry::path_to_band;
use super::{Lane, InteractionKind};

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
            let position2d = self.path.along(*car.position);
            let direction = self.path.direction_along(*car.position);
            world.send(renderer_id, AddInstance{
                scene_id: scene_id,
                batch_id: 0,
                position: Instance{
                    instance_position: [position2d.x, position2d.y, 0.0],
                    instance_direction: [direction.x, direction.y],
                    instance_color: [0.5, 0.5, 0.5]
                }
            });
        }

        world.send(renderer_id, UpdateThing::new(scene_id, self_id.instance_id as usize, path_to_band(&self.path, 3.0, 0.0), Instance{
            instance_position: [0.0, 0.0, 0.0],
            instance_direction: [1.0, 0.0],
            instance_color: [0.7, 0.7, 0.7]
        }));
        self.interactions.iter().find(|inter| match inter.kind {
            InteractionKind::Overlap{start, end, ..} => {
                world.send(renderer_id, UpdateThing::new(scene_id, 100 + self_id.instance_id as usize, path_to_band(&self.path.subsection(start, end), 1.0, 0.1), Instance{
                    instance_position: [0.0, 0.0, 0.0],
                    instance_direction: [1.0, 0.0],
                    instance_color: [1.0, 0.7, 0.7]
                }));
                true
            },
            _ => false
        });
    }
});

pub fn setup(system: &mut ActorSystem) {
    system.add_inbox::<SetupInScene, Lane>(InMemory("setup_in_scene", 512, 4));
    system.add_inbox::<RenderToScene, Lane>(InMemory("render_to_scene", 512, 4));
}