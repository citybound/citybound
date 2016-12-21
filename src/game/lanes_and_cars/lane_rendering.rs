use descartes::{Band, FiniteCurve, WithUniqueOrthogonal, Norm, Path, Dot, RoughlyComparable};
use kay::{Actor, CVec, Individual, Recipient, RecipientAsSwarm, ActorSystem, Swarm, Fate};
use monet::{Instance, Thing, Vertex, UpdateThing};
use core::geometry::{band_to_thing, dash_path};
use super::{Lane, TransferLane, InteractionKind};
use itertools::Itertools;

#[path = "./resources/car.rs"]
mod car;

#[path = "./resources/traffic_light.rs"]
mod traffic_light;

use super::lane_thing_collector::ThingCollector;

use ::monet::SetupInScene;
use ::monet::AddBatch;

impl RecipientAsSwarm<SetupInScene> for Lane {
    fn receive(_swarm: &mut Swarm<Self>, msg: &SetupInScene) -> Fate {match *msg {
        SetupInScene{renderer_id, scene_id} => {
            renderer_id << AddBatch{scene_id: scene_id, batch_id: 0, thing: car::create()};
            renderer_id << AddBatch{scene_id: scene_id, batch_id: 1, thing: traffic_light::create()};
            renderer_id << AddBatch{scene_id: scene_id, batch_id: 2, thing: traffic_light::create_light()};
            renderer_id << AddBatch{scene_id: scene_id, batch_id: 3, thing: traffic_light::create_light_left()};
            renderer_id << AddBatch{scene_id: scene_id, batch_id: 4, thing: traffic_light::create_light_right()};

            renderer_id << AddBatch{scene_id: scene_id, batch_id: 1333, thing: Thing::new(
                vec![
                    Vertex{position: [-1.0, -1.0, 0.0]},
                    Vertex{position: [1.0, -1.0, 0.0]},
                    Vertex{position: [1.0, 1.0, 0.0]},
                    Vertex{position: [-1.0, 1.0, 0.0]}
                ],
                vec![
                    0, 1, 2,
                    2, 3, 0
                ]
            )};

            Fate::Live
        }
    }}
}

impl RecipientAsSwarm<SetupInScene> for TransferLane {
    fn receive(_swarm: &mut Swarm<Self>, msg: &SetupInScene) -> Fate {match *msg{
        SetupInScene{..} => {Fate::Live}
    }}
}

use super::lane_thing_collector::RenderToCollector;
use super::lane_thing_collector::Control::{Update, Freeze};

const CONSTRUCTION_ANIMATION_DELAY : f32 = 120.0;

impl Recipient<RenderToCollector> for Lane {
    fn receive(&mut self, msg: &RenderToCollector) -> Fate {match *msg {
        RenderToCollector(collector_id) => {
            let maybe_path = if self.in_construction - CONSTRUCTION_ANIMATION_DELAY < self.length {
                self.path.subsection(0.0, (self.in_construction - CONSTRUCTION_ANIMATION_DELAY).max(0.0))
            } else {
                Some(self.path.clone())
            };
            if collector_id == ThingCollector::<LaneAsphalt>::id() {
                collector_id << Update(self.id(), maybe_path
                    .map(|path| band_to_thing(&Band::new(path, 6.0), if self.on_intersection {0.2} else {0.0}))
                    .unwrap_or_else(|| Thing::new(vec![], vec![])));
                if self.in_construction - CONSTRUCTION_ANIMATION_DELAY > self.length {
                    collector_id << Freeze(self.id())
                }
            } else {
                let left_marker = maybe_path.clone().and_then(|path|
                    path.shift_orthogonally(2.5)
                ).map(|path| band_to_thing(&Band::new(path, 0.6), 0.1))
                .unwrap_or_else(|| Thing::new(vec![], vec![]));

                let right_marker = maybe_path.and_then(|path|
                    path.shift_orthogonally(-2.5)
                ).map(|path| band_to_thing(&Band::new(path, 0.6), 0.1))
                .unwrap_or_else(|| Thing::new(vec![], vec![]));
                collector_id << Update(self.id(), left_marker + right_marker);
                if self.in_construction - CONSTRUCTION_ANIMATION_DELAY > self.length {
                    collector_id << Freeze(self.id())
                }
            }

            Fate::Live
        }
    }}
}

impl Recipient<RenderToCollector> for TransferLane {
    fn receive(&mut self, msg: &RenderToCollector) -> Fate {match *msg {
        RenderToCollector(collector_id) => {
            let maybe_path = if self.in_construction - 2.0 * CONSTRUCTION_ANIMATION_DELAY < self.length {
                self.path.subsection(0.0, (self.in_construction - 2.0 * CONSTRUCTION_ANIMATION_DELAY).max(0.0))
            } else {
                Some(self.path.clone())
            };

            collector_id << Update(self.id(), maybe_path
                .map(|path| dash_path(path, 2.0, 4.0).into_iter().map(|dash|
                    band_to_thing(&Band::new(dash, 0.8), 0.2)
                ).sum())
                .unwrap_or_else(|| Thing::new(vec![], vec![])));
            if self.in_construction - 2.0 * CONSTRUCTION_ANIMATION_DELAY > self.length {
                collector_id << Freeze(self.id())
            }

            Fate::Live
        }
    }}
}

use ::monet::RenderToScene;
use ::monet::AddInstance;
use ::monet::AddSeveralInstances;

const DEBUG_VIEW_LANDMARKS : bool = false;
const DEBUG_VIEW_SIGNALS : bool = false;
const DEBUG_VIEW_OBSTACLES : bool = false;
const DEBUG_VIEW_TRANSFER_OBSTACLES : bool = false;

impl Recipient<RenderToScene> for Lane {
    fn receive(&mut self, msg: &RenderToScene) -> Fate {match *msg {
        RenderToScene{renderer_id, scene_id} => {
            let mut cars_iter = self.cars.iter();
            let mut current_offset = 0.0;
            let mut car_instances = CVec::with_capacity(self.cars.len());
            for segment in self.path.segments().iter() {
                for car in cars_iter.take_while_ref(|car| *car.position - current_offset < segment.length()) {
                    let position2d = segment.along(*car.position - current_offset);
                    let direction = segment.direction_along(*car.position - current_offset);
                    car_instances.push(Instance{
                        instance_position: [position2d.x, position2d.y, 0.0],
                        instance_direction: [direction.x, direction.y],
                        instance_color: if DEBUG_VIEW_LANDMARKS {
                            ::core::geometry::RANDOM_COLORS[
                                car.destination.landmark.instance_id as usize % ::core::geometry::RANDOM_COLORS.len()
                            ]
                        } else {
                            ::core::geometry::RANDOM_COLORS[
                                car.trip.instance_id as usize % ::core::geometry::RANDOM_COLORS.len()
                            ]
                        }
                    })
                }
                current_offset += segment.length;
            }

            if DEBUG_VIEW_OBSTACLES {
                for &(obstacle, _id) in &self.obstacles {
                    let position2d = if *obstacle.position < self.length {
                        self.path.along(*obstacle.position)
                    } else {
                        self.path.end() + (*obstacle.position - self.length) * self.path.end_direction()
                    };
                    let direction = self.path.direction_along(*obstacle.position);

                    car_instances.push(Instance{
                        instance_position: [position2d.x, position2d.y, 0.0],
                        instance_direction: [direction.x, direction.y],
                        instance_color: [1.0, 0.0, 0.0]
                    });
                }
            }

            if !car_instances.is_empty() {
                renderer_id << AddSeveralInstances{
                    scene_id: scene_id,
                    batch_id: 0,
                    instances: car_instances
                };
            }
            //                         no traffic light for u-turn
            if self.on_intersection && !self.path.end_direction().is_roughly_within(-self.path.start_direction(), 0.1) {
                let mut position = self.path.start();
                let (position_shift, batch_id) = if !self.path.start_direction().is_roughly_within(self.path.end_direction(), 0.5) {
                    let dot = self.path.end_direction().dot(&self.path.start_direction().orthogonal());
                    let shift = if dot > 0.0 {1.0} else {-1.0};
                    let batch_id = if dot > 0.0 {4} else {3};
                    (shift, batch_id)
                } else {(0.0, 2)};
                position += self.path.start_direction().orthogonal() * position_shift;
                let direction = self.path.start_direction();

                renderer_id << AddInstance{
                    scene_id: scene_id,
                    batch_id: 1,
                    instance: Instance{
                        instance_position: [position.x, position.y, 6.0],
                        instance_direction: [direction.x, direction.y],
                        instance_color: [0.1, 0.1, 0.1]
                    }
                };

                if self.yellow_to_red && self.green {
                    renderer_id << AddInstance{
                        scene_id: scene_id,
                        batch_id: batch_id,
                        instance: Instance{
                            instance_position: [position.x, position.y, 6.7],
                            instance_direction: [direction.x, direction.y],
                            instance_color: [1.0, 0.8, 0.0]
                        }
                    }
                } else if self.green {
                    renderer_id << AddInstance{
                        scene_id: scene_id,
                        batch_id: batch_id,
                        instance: Instance{
                            instance_position: [position.x, position.y, 6.1],
                            instance_direction: [direction.x, direction.y],
                            instance_color: [0.0, 1.0, 0.2]
                        }
                    }
                }
                
                if !self.green {
                    renderer_id << AddInstance{
                        scene_id: scene_id,
                        batch_id: batch_id,
                        instance: Instance{
                            instance_position: [position.x, position.y, 7.3],
                            instance_direction: [direction.x, direction.y],
                            instance_color: [1.0, 0.0, 0.0]
                        }
                    };

                    if self.yellow_to_green {
                        renderer_id << AddInstance{
                            scene_id: scene_id,
                            batch_id: batch_id,
                            instance: Instance{
                                instance_position: [position.x, position.y, 6.7],
                                instance_direction: [direction.x, direction.y],
                                instance_color: [1.0, 0.8, 0.0]
                            }
                        }
                    }
                }
            }

            if DEBUG_VIEW_SIGNALS && self.on_intersection {
                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 4000 + self.id().instance_id as u16,
                    thing: band_to_thing(&Band::new(self.path.clone(), 0.3), if self.green {0.4} else {0.2}),
                    instance: Instance::with_color(if self.green {[0.0, 1.0, 0.0]} else {[1.0, 0.0, 0.0]}),
                    is_decal: true
                };
            }

            if ! self.interactions.iter().any(|inter| match inter.kind {InteractionKind::Next{..} => true, _ => false}) {
                renderer_id << AddInstance{scene_id: scene_id, batch_id: 1333, instance: Instance{
                    instance_position: [self.path.end().x, self.path.end().y, 0.5],
                    instance_direction: [1.0, 0.0],
                    instance_color: [1.0, 0.0, 0.0]
                }};
            }

            if ! self.interactions.iter().any(|inter| match inter.kind {InteractionKind::Previous{..} => true, _ => false}) {
                renderer_id << AddInstance{scene_id: scene_id, batch_id: 1333, instance: Instance{
                    instance_position: [self.path.start().x, self.path.start().y, 0.5],
                    instance_direction: [1.0, 0.0],
                    instance_color: [0.0, 1.0, 0.0]
                }};
            }

            if DEBUG_VIEW_LANDMARKS && self.pathfinding_info.routes_changed {
                let (random_color, is_landmark) = if let Some(as_destination) = self.pathfinding_info.as_destination {
                    let random_color : [f32; 3] = ::core::geometry::RANDOM_COLORS[
                        as_destination.landmark.instance_id as usize % ::core::geometry::RANDOM_COLORS.len()
                    ];
                    let weaker_random_color = [(random_color[0] + 1.0) / 2.0, (random_color[1] + 1.0) / 2.0, (random_color[2] + 1.0) / 2.0];
                    (weaker_random_color, as_destination.is_landmark())
                } else {
                    ([1.0, 1.0, 1.0], false)
                };

                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 4000 + self.id().instance_id as u16,
                    thing: band_to_thing(&Band::new(self.path.clone(), if is_landmark {2.5} else {1.0}), 0.4),
                    instance: Instance::with_color(random_color),
                    is_decal: true
                };
            }
            Fate::Live
        }
    }}
}

impl Recipient<RenderToScene> for TransferLane {
    fn receive(&mut self, msg: &RenderToScene) -> Fate {match *msg{
        RenderToScene{renderer_id, scene_id} => {
            let mut cars_iter = self.cars.iter();
            let mut current_offset = 0.0;
            let mut car_instances = CVec::with_capacity(self.cars.len());
            for segment in self.path.segments().iter() {
                for car in cars_iter.take_while_ref(|car| *car.position - current_offset < segment.length()) {
                    let position2d = segment.along(*car.position - current_offset);
                    let direction = segment.direction_along(*car.position - current_offset);
                    let rotated_direction = (direction + 0.3 * car.transfer_velocity * direction.orthogonal()).normalize();
                    let shifted_position2d = position2d + 2.5 * direction.orthogonal() * car.transfer_position;
                    car_instances.push(Instance{
                        instance_position: [shifted_position2d.x, shifted_position2d.y, 0.0],
                        instance_direction: [rotated_direction.x, rotated_direction.y],
                        instance_color: if DEBUG_VIEW_LANDMARKS {
                            ::core::geometry::RANDOM_COLORS[
                                car.destination.landmark.instance_id as usize % ::core::geometry::RANDOM_COLORS.len()
                            ]
                        } else {
                            ::core::geometry::RANDOM_COLORS[
                                car.trip.instance_id as usize % ::core::geometry::RANDOM_COLORS.len()
                            ]
                        }
                    })
                }
                current_offset += segment.length;
            }

            if DEBUG_VIEW_TRANSFER_OBSTACLES{
                for obstacle in &self.left_obstacles {
                    let position2d = if *obstacle.position < self.length {
                        self.path.along(*obstacle.position)
                    } else {
                        self.path.end() + (*obstacle.position - self.length) * self.path.end_direction()
                    } - 1.0 * self.path.direction_along(*obstacle.position).orthogonal();
                    let direction = self.path.direction_along(*obstacle.position);

                    car_instances.push(Instance{
                        instance_position: [position2d.x, position2d.y, 0.0],
                        instance_direction: [direction.x, direction.y],
                        instance_color: [1.0, 0.7, 0.7]
                    });
                }

                for obstacle in &self.right_obstacles {
                    let position2d = if *obstacle.position < self.length {
                        self.path.along(*obstacle.position)
                    } else {
                        self.path.end() + (*obstacle.position - self.length) * self.path.end_direction()
                    } + 1.0 * self.path.direction_along(*obstacle.position).orthogonal();
                    let direction = self.path.direction_along(*obstacle.position);

                    car_instances.push(Instance{
                        instance_position: [position2d.x, position2d.y, 0.0],
                        instance_direction: [direction.x, direction.y],
                        instance_color: [1.0, 0.7, 0.7]
                    });
                }
            }

            if !car_instances.is_empty() {
                renderer_id << AddSeveralInstances{
                    scene_id: scene_id,
                    batch_id: 0,
                    instances: car_instances
                };
            }

            if self.left.is_none() {
                let position = self.path.along(self.length / 2.0) + self.path.direction_along(self.length / 2.0).orthogonal();
                renderer_id << AddInstance{scene_id: scene_id, batch_id: 1333, instance: Instance{
                    instance_position: [position.x, position.y, 0.0],
                    instance_direction: [1.0, 0.0],
                    instance_color: [1.0, 0.0, 0.0]
                }};
            }
            if self.right.is_none() {
                let position = self.path.along(self.length / 2.0) - self.path.direction_along(self.length / 2.0).orthogonal();
                renderer_id << AddInstance{scene_id: scene_id, batch_id: 1333, instance: Instance{
                    instance_position: [position.x, position.y, 0.0],
                    instance_direction: [1.0, 0.0],
                    instance_color: [1.0, 0.0, 0.0]
                }};
            }
            Fate::Live
        }
    }}
}

use super::lane_thing_collector::Control::Remove;

pub fn on_build(lane: &Lane) {
    lane.id() << RenderToCollector(ThingCollector::<LaneAsphalt>::id());
    if !lane.on_intersection {
        lane.id() << RenderToCollector(ThingCollector::<LaneMarker>::id());
    }
}

pub fn on_build_transfer(lane: &TransferLane) {
    lane.id() << RenderToCollector(ThingCollector::<TransferLaneMarkerGaps>::id());
}

pub fn on_unbuild(lane: &Lane) {
    ThingCollector::<LaneAsphalt>::id() << Remove(lane.id());
    if !lane.on_intersection {
        ThingCollector::<LaneMarker>::id() << Remove(lane.id());
    }

    if DEBUG_VIEW_LANDMARKS {
        // TODO: ugly
        ::monet::Renderer::id() << UpdateThing{
            scene_id: 0,
            thing_id: 4000 + lane.id().instance_id as u16,
            thing: Thing::new(vec![], vec![]),
            instance: Instance::with_color([0.0, 0.0, 0.0]),
            is_decal: true
        };
    }

    if DEBUG_VIEW_SIGNALS {
        ::monet::Renderer::id() << UpdateThing{
            scene_id: 0,
            thing_id: 4000 + lane.id().instance_id as u16,
            thing: Thing::new(vec![], vec![]),
            instance: Instance::with_color([0.0, 0.0, 0.0]),
            is_decal: true
        };
    }
}

pub fn on_unbuild_transfer(lane: &TransferLane) {
    ThingCollector::<TransferLaneMarkerGaps>::id() << Remove(lane.id());
}

#[derive(Clone)]
pub struct LaneAsphalt;
#[derive(Clone)]
pub struct LaneMarker;
#[derive(Clone)]
pub struct TransferLaneMarkerGaps;

pub fn setup(system: &mut ActorSystem) {
    system.add_inbox::<SetupInScene, Swarm<Lane>>();
    system.add_inbox::<RenderToCollector, Swarm<Lane>>();
    system.add_inbox::<RenderToScene, Swarm<Lane>>();
    super::lane_thing_collector::setup::<LaneAsphalt>(system, [0.7, 0.7, 0.7], 2000, false);
    super::lane_thing_collector::setup::<LaneMarker>(system, [1.0, 1.0, 1.0], 2100, true);

    system.add_inbox::<SetupInScene, Swarm<TransferLane>>();
    system.add_inbox::<RenderToCollector, Swarm<TransferLane>>();
    system.add_inbox::<RenderToScene, Swarm<TransferLane>>();
    super::lane_thing_collector::setup::<TransferLaneMarkerGaps>(system, [0.7, 0.7, 0.7], 2200, true);
    
    super::planning::setup(system);
}