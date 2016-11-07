use descartes::{Band, FiniteCurve, WithUniqueOrthogonal, Norm};
use kay::{Actor, Recipient, RecipientAsSwarm, ActorSystem, Swarm, Fate};
use monet::{Instance};
use core::geometry::band_to_thing;
use super::{Lane, TransferLane, InteractionKind};

#[path = "./resources/car.rs"]
mod car;

use ::monet::SetupInScene;
use ::monet::AddBatch;

impl RecipientAsSwarm<SetupInScene> for Lane {
    fn receive(_swarm: &mut Swarm<Self>, msg: &SetupInScene) -> Fate {match *msg {
        SetupInScene{renderer_id, scene_id} => {
            renderer_id << AddBatch{scene_id: scene_id, batch_id: 0, thing: car::create()};
            Fate::Live
        }
    }}
}

impl RecipientAsSwarm<SetupInScene> for TransferLane {
    fn receive(_swarm: &mut Swarm<Self>, msg: &SetupInScene) -> Fate {match *msg{
        SetupInScene{renderer_id, scene_id} => {
            renderer_id << AddBatch{scene_id: scene_id, batch_id: 1, thing: car::create()};
            Fate::Live
        }
    }}
}

use ::monet::RenderToScene;
use ::monet::AddInstance;
use ::monet::UpdateThing;

impl Recipient<RenderToScene> for Lane {
    fn receive(&mut self, msg: &RenderToScene) -> Fate {match *msg {
        RenderToScene{renderer_id, scene_id} => {
            for car in &self.cars {
                let position2d = self.path.along(*car.position);
                let direction = self.path.direction_along(*car.position);
                renderer_id << AddInstance{
                    scene_id: scene_id,
                    batch_id: 0,
                    position: Instance{
                        instance_position: [position2d.x, position2d.y, 0.0],
                        instance_direction: [direction.x, direction.y],
                        instance_color: [0.5, 0.5, 0.5]
                    }
                };
            }

            renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: self.id().instance_id as usize,
                    thing: band_to_thing(&Band::new(self.path.clone(), 3.0), 0.0),
                    instance: Instance{
                        instance_position: [0.0, 0.0, 0.0],
                        instance_direction: [1.0, 0.0],
                        instance_color: [0.7, 0.7, 0.7]
                    }
            };
            self.interactions.iter().find(|inter| match inter.kind {
                InteractionKind::Overlap{start, end, ..} => {
                    renderer_id << UpdateThing{
                        scene_id: scene_id,
                        thing_id: 100 + self.id().instance_id as usize,
                        thing: band_to_thing(&Band::new(self.path.subsection(start, end), 1.0), 0.1),
                        instance: Instance{
                            instance_position: [0.0, 0.0, 0.0],
                            instance_direction: [1.0, 0.0],
                            instance_color: [1.0, 0.7, 0.7]
                        }
                    };
                    true
                },
                _ => false
            });
            Fate::Live
        }
    }}
}

impl Recipient<RenderToScene> for TransferLane {
    fn receive(&mut self, msg: &RenderToScene) -> Fate {match *msg{
        RenderToScene{renderer_id, scene_id} => {
            for car in &self.cars {
                let position2d = self.path.along(*car.position);
                let direction = self.path.direction_along(*car.position);
                let rotated_direction = (direction + 0.4 * car.transfer_velocity * direction.orthogonal()).normalize();
                let shifted_position2d = position2d + 3.0 * direction.orthogonal() * car.transfer_position;
                renderer_id << AddInstance{
                    scene_id: scene_id,
                    batch_id: 1,
                    position: Instance{
                        instance_position: [shifted_position2d.x, shifted_position2d.y, 0.0],
                        instance_direction: [rotated_direction.x, rotated_direction.y],
                        instance_color: [0.3, 0.3, 0.0]
                    }
                };
            }

            renderer_id << UpdateThing{
                scene_id: scene_id,
                thing_id: 200 + self.id().instance_id as usize,
                thing: band_to_thing(&Band::new(self.path.clone(), 3.0), 0.1),
                instance: Instance{
                    instance_position: [0.0, 0.0, 0.0],
                    instance_direction: [1.0, 0.0],
                    instance_color: [1.0, 1.0, 0.5]
                }
            };
            Fate::Live
        }
    }}
}

pub fn setup(system: &mut ActorSystem) {
    system.add_inbox::<SetupInScene, Swarm<Lane>>();
    system.add_inbox::<RenderToScene, Swarm<Lane>>();
    system.add_inbox::<SetupInScene, Swarm<TransferLane>>();
    system.add_inbox::<RenderToScene, Swarm<TransferLane>>();

    super::planning::setup(system);
}