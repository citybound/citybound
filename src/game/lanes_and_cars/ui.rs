use compass::{FiniteCurve, WithUniqueOrthogonal, Norm};
use kay::{ID, Recipient, World, ActorSystem, InMemory, Swarm};
use monet::{Instance, RenderToScene, SetupInScene, AddBatch, AddInstance, UpdateThing};
use core::geometry::path_to_band;
use super::{Lane, TransferLane, InteractionKind};

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

recipient!(Swarm<TransferLane>, (&mut self, world: &mut World, self_id: ID) {
    SetupInScene: &SetupInScene{renderer_id, scene_id} => {
        world.send(renderer_id, AddBatch::new(scene_id, 1, car::create()));
    }
});

recipient!(TransferLane, (&mut self, world: &mut World, self_id: ID) {
    RenderToScene: &RenderToScene{renderer_id, scene_id} => {
        for car in &self.cars {
            let position2d = self.path.along(*car.position);
            let direction = self.path.direction_along(*car.position);
            let rotated_direction = (direction + 0.4 * car.transfer_velocity * direction.orthogonal()).normalize();
            let shifted_position2d = position2d + 3.0 * direction.orthogonal() * car.transfer_position;
            world.send(renderer_id, AddInstance{
                scene_id: scene_id,
                batch_id: 1,
                position: Instance{
                    instance_position: [shifted_position2d.x, shifted_position2d.y, 0.0],
                    instance_direction: [rotated_direction.x, rotated_direction.y],
                    instance_color: [0.3, 0.3, 0.0]
                }
            });
        }

        world.send(renderer_id, UpdateThing::new(scene_id, 200 + self_id.instance_id as usize, path_to_band(&self.path, 3.0, 0.1), Instance{
            instance_position: [0.0, 0.0, 0.0],
            instance_direction: [1.0, 0.0],
            instance_color: [1.0, 1.0, 0.5]
        }));
    }
});

pub fn setup(system: &mut ActorSystem) {
    system.add_inbox::<SetupInScene, Lane>(InMemory("setup_in_scene", 512, 4));
    system.add_inbox::<RenderToScene, Lane>(InMemory("render_to_scene", 512, 4));
    system.add_inbox::<SetupInScene, TransferLane>(InMemory("transfer_setup_in_scene", 512, 4));
    system.add_inbox::<RenderToScene, TransferLane>(InMemory("transfer_render_to_scene", 512, 4));
}