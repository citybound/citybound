use descartes::{Band, FiniteCurve, WithUniqueOrthogonal, Norm, Path, Dot, RoughlyComparable};
use compact::CVec;
use kay::{ActorSystem, Fate, World};
use kay::swarm::{Swarm, SubActor};
use monet::{Instance, Thing, Vertex, UpdateThing};
use stagemaster::geometry::{band_to_thing, dash_path};
use super::lane::{Lane, TransferLane};
use super::connectivity::InteractionKind;
use itertools::Itertools;

#[path = "./resources/car.rs"]
mod car;

#[path = "./resources/traffic_light.rs"]
mod traffic_light;

pub mod lane_thing_collector;
use self::lane_thing_collector::ThingCollector;

use monet::SetupInScene;
use monet::AddBatch;

pub fn setup(system: &mut ActorSystem) {

    system.extend::<Swarm<Lane>, _>(|mut the_lane_swarm| {
        the_lane_swarm.on(|&SetupInScene { renderer_id, scene_id }, _, world| {
            world.send(renderer_id,
                       AddBatch {
                           scene_id: scene_id,
                           batch_id: 8000,
                           thing: car::create(),
                       });
            world.send(renderer_id,
                       AddBatch {
                           scene_id: scene_id,
                           batch_id: 8001,
                           thing: traffic_light::create(),
                       });
            world.send(renderer_id,
                       AddBatch {
                           scene_id: scene_id,
                           batch_id: 8002,
                           thing: traffic_light::create_light(),
                       });
            world.send(renderer_id,
                       AddBatch {
                           scene_id: scene_id,
                           batch_id: 8003,
                           thing: traffic_light::create_light_left(),
                       });
            world.send(renderer_id,
                       AddBatch {
                           scene_id: scene_id,
                           batch_id: 8004,
                           thing: traffic_light::create_light_right(),
                       });

            world.send(renderer_id,
                       AddBatch {
                           scene_id: scene_id,
                           batch_id: 1333,
                           thing: Thing::new(vec![Vertex { position: [-1.0, -1.0, 0.0] },
                                                  Vertex { position: [1.0, -1.0, 0.0] },
                                                  Vertex { position: [1.0, 1.0, 0.0] },
                                                  Vertex { position: [-1.0, 1.0, 0.0] }],
                                             vec![0, 1, 2, 2, 3, 0]),
                       });

            Fate::Live
        });
    });

    system.extend(Swarm::<Lane>::subactors(|mut each_lane| {
        each_lane.on(|&RenderToCollector(collector_id), lane, world| {
            let maybe_path = if lane.construction.progress - CONSTRUCTION_ANIMATION_DELAY <
                                lane.construction.length {
                lane.construction
                    .path
                    .subsection(0.0,
                                (lane.construction.progress - CONSTRUCTION_ANIMATION_DELAY)
                                    .max(0.0))
            } else {
                Some(lane.construction.path.clone())
            };
            if collector_id == world.id::<ThingCollector<LaneAsphalt>>() {
                world.send(collector_id,
                           Update(lane.id(),
                                  maybe_path
                                      .map(|path| {
                    band_to_thing(&Band::new(path, 6.0),
                                  if lane.connectivity.on_intersection {
                                      0.2
                                  } else {
                                      0.0
                                  })
                })
                                      .unwrap_or_else(|| Thing::new(vec![], vec![]))));
                if lane.construction.progress - CONSTRUCTION_ANIMATION_DELAY >
                   lane.construction.length {
                    world.send(collector_id, Freeze(lane.id()))
                }
            } else {
                let left_marker = maybe_path
                    .clone()
                    .and_then(|path| path.shift_orthogonally(2.5))
                    .map(|path| band_to_thing(&Band::new(path, 0.6), 0.1))
                    .unwrap_or_else(|| Thing::new(vec![], vec![]));

                let right_marker = maybe_path
                    .and_then(|path| path.shift_orthogonally(-2.5))
                    .map(|path| band_to_thing(&Band::new(path, 0.6), 0.1))
                    .unwrap_or_else(|| Thing::new(vec![], vec![]));
                world.send(collector_id, Update(lane.id(), left_marker + right_marker));
                if lane.construction.progress - CONSTRUCTION_ANIMATION_DELAY >
                   lane.construction.length {
                    world.send(collector_id, Freeze(lane.id()))
                }
            }

            Fate::Live
        });

        each_lane.on(|&RenderToScene { renderer_id, scene_id }, lane, world| {
            let mut cars_iter = lane.microtraffic.cars.iter();
            let mut current_offset = 0.0;
            let mut car_instances = CVec::with_capacity(lane.microtraffic.cars.len());
            for segment in lane.construction.path.segments().iter() {
                for car in cars_iter.take_while_ref(|car| {
                    *car.position - current_offset < segment.length()
                }) {
                    let position2d = segment.along(*car.position - current_offset);
                    let direction = segment.direction_along(*car.position - current_offset);
                    car_instances.push(Instance {
                                           instance_position: [position2d.x, position2d.y, 0.0],
                                           instance_direction: [direction.x, direction.y],
                                           instance_color: if DEBUG_VIEW_LANDMARKS {
                                               ::core::colors::RANDOM_COLORS
                                                   [car.destination.landmark.sub_actor_id as usize %
                                               ::core::colors::RANDOM_COLORS.len()]
                                           } else {
                                               ::core::colors::RANDOM_COLORS
                                                   [car.trip.sub_actor_id as usize %
                                               ::core::colors::RANDOM_COLORS.len()]
                                           },
                                       })
                }
                current_offset += segment.length;
            }

            if DEBUG_VIEW_OBSTACLES {
                for &(obstacle, _id) in &lane.microtraffic.obstacles {
                    let position2d = if *obstacle.position < lane.construction.length {
                        lane.construction.path.along(*obstacle.position)
                    } else {
                        lane.construction.path.end() +
                        (*obstacle.position - lane.construction.length) *
                        lane.construction.path.end_direction()
                    };
                    let direction = lane.construction
                        .path
                        .direction_along(*obstacle.position);

                    car_instances.push(Instance {
                                           instance_position: [position2d.x, position2d.y, 0.0],
                                           instance_direction: [direction.x, direction.y],
                                           instance_color: [1.0, 0.0, 0.0],
                                       });
                }
            }

            if !car_instances.is_empty() {
                world.send(renderer_id,
                           AddSeveralInstances {
                               scene_id: scene_id,
                               batch_id: 8000,
                               instances: car_instances,
                           });
            }
            // no traffic light for u-turn
            if lane.connectivity.on_intersection &&
               !lane.construction
                    .path
                    .end_direction()
                    .is_roughly_within(-lane.construction.path.start_direction(), 0.1) {
                let mut position = lane.construction.path.start();
                let (position_shift, batch_id) = if
                    !lane.construction
                         .path
                         .start_direction()
                         .is_roughly_within(lane.construction.path.end_direction(), 0.5) {
                    let dot = lane.construction
                        .path
                        .end_direction()
                        .dot(&lane.construction.path.start_direction().orthogonal());
                    let shift = if dot > 0.0 { 1.0 } else { -1.0 };
                    let batch_id = if dot > 0.0 { 8004 } else { 8003 };
                    (shift, batch_id)
                } else {
                    (0.0, 8002)
                };
                position += lane.construction.path.start_direction().orthogonal() * position_shift;
                let direction = lane.construction.path.start_direction();

                world.send(renderer_id,
                           AddInstance {
                               scene_id: scene_id,
                               batch_id: 8001,
                               instance: Instance {
                                   instance_position: [position.x, position.y, 6.0],
                                   instance_direction: [direction.x, direction.y],
                                   instance_color: [0.1, 0.1, 0.1],
                               },
                           });

                if lane.microtraffic.yellow_to_red && lane.microtraffic.green {
                    world.send(renderer_id,
                               AddInstance {
                                   scene_id: scene_id,
                                   batch_id: batch_id,
                                   instance: Instance {
                                       instance_position: [position.x, position.y, 6.7],
                                       instance_direction: [direction.x, direction.y],
                                       instance_color: [1.0, 0.8, 0.0],
                                   },
                               })
                } else if lane.microtraffic.green {
                    world.send(renderer_id,
                               AddInstance {
                                   scene_id: scene_id,
                                   batch_id: batch_id,
                                   instance: Instance {
                                       instance_position: [position.x, position.y, 6.1],
                                       instance_direction: [direction.x, direction.y],
                                       instance_color: [0.0, 1.0, 0.2],
                                   },
                               })
                }

                if !lane.microtraffic.green {
                    world.send(renderer_id,
                               AddInstance {
                                   scene_id: scene_id,
                                   batch_id: batch_id,
                                   instance: Instance {
                                       instance_position: [position.x, position.y, 7.3],
                                       instance_direction: [direction.x, direction.y],
                                       instance_color: [1.0, 0.0, 0.0],
                                   },
                               });

                    if lane.microtraffic.yellow_to_green {
                        world.send(renderer_id,
                                   AddInstance {
                                       scene_id: scene_id,
                                       batch_id: batch_id,
                                       instance: Instance {
                                           instance_position: [position.x, position.y, 6.7],
                                           instance_direction: [direction.x, direction.y],
                                           instance_color: [1.0, 0.8, 0.0],
                                       },
                                   })
                    }
                }
            }

            if DEBUG_VIEW_SIGNALS && lane.connectivity.on_intersection {
                world.send(renderer_id,
                           UpdateThing {
                               scene_id: scene_id,
                               thing_id: 4000 + lane.id().sub_actor_id as u16,
                               thing: band_to_thing(&Band::new(lane.construction.path.clone(),
                                                               0.3),
                                                    if lane.microtraffic.green {
                                                        0.4
                                                    } else {
                                                        0.2
                                                    }),
                               instance: Instance::with_color(if lane.microtraffic.green {
                                                                  [0.0, 1.0, 0.0]
                                                              } else {
                                                                  [1.0, 0.0, 0.0]
                                                              }),
                               is_decal: true,
                           });
            }

            if !lane.connectivity
                    .interactions
                    .iter()
                    .any(|inter| match inter.kind {
                             InteractionKind::Next { .. } => true,
                             _ => false,
                         }) {
                world.send(renderer_id,
                           AddInstance {
                               scene_id: scene_id,
                               batch_id: 1333,
                               instance: Instance {
                                   instance_position: [lane.construction.path.end().x,
                                                       lane.construction.path.end().y,
                                                       0.5],
                                   instance_direction: [1.0, 0.0],
                                   instance_color: [1.0, 0.0, 0.0],
                               },
                           });
            }

            if !lane.connectivity
                    .interactions
                    .iter()
                    .any(|inter| match inter.kind {
                             InteractionKind::Previous { .. } => true,
                             _ => false,
                         }) {
                world.send(renderer_id,
                           AddInstance {
                               scene_id: scene_id,
                               batch_id: 1333,
                               instance: Instance {
                                   instance_position: [lane.construction.path.start().x,
                                                       lane.construction.path.start().y,
                                                       0.5],
                                   instance_direction: [1.0, 0.0],
                                   instance_color: [0.0, 1.0, 0.0],
                               },
                           });
            }

            if DEBUG_VIEW_LANDMARKS && lane.pathfinding.routes_changed {
                let (random_color, is_landmark) = if let Some(as_destination) =
                    lane.pathfinding.as_destination {
                    let random_color: [f32; 3] = ::core::colors::RANDOM_COLORS
                        [as_destination.landmark.sub_actor_id as usize %
                    ::core::colors::RANDOM_COLORS.len()];
                    let weaker_random_color = [(random_color[0] + 1.0) / 2.0,
                                               (random_color[1] + 1.0) / 2.0,
                                               (random_color[2] + 1.0) / 2.0];
                    (weaker_random_color, as_destination.is_landmark())
                } else {
                    ([1.0, 1.0, 1.0], false)
                };

                world.send(renderer_id,
                           UpdateThing {
                               scene_id: scene_id,
                               thing_id: 4000 + lane.id().sub_actor_id as u16,
                               thing: band_to_thing(&Band::new(lane.construction.path.clone(),
                                                               if is_landmark {
                                                                   2.5
                                                               } else {
                                                                   1.0
                                                               }),
                                                    0.4),
                               instance: Instance::with_color(random_color),
                               is_decal: true,
                           });
            }
            Fate::Live
        })
    }));

    system.extend::<Swarm<TransferLane>, _>(|mut the_t_lane_swarm| {
        the_t_lane_swarm.on(|_: &SetupInScene, _, _| Fate::Live)
    });

    system.extend(Swarm::<TransferLane>::subactors(|mut each_t_lane| {
        each_t_lane.on(|&RenderToCollector(collector_id), lane, world| {
            let maybe_path = if lane.construction.progress - 2.0 * CONSTRUCTION_ANIMATION_DELAY <
                                lane.construction.length {
                lane.construction
                    .path
                    .subsection(0.0,
                                (lane.construction.progress - 2.0 * CONSTRUCTION_ANIMATION_DELAY)
                                    .max(0.0))
            } else {
                Some(lane.construction.path.clone())
            };

            world.send(collector_id,
                       Update(lane.id(),
                              maybe_path
                                  .map(|path| {
                dash_path(&path, 2.0, 4.0)
                    .into_iter()
                    .map(|dash| band_to_thing(&Band::new(dash, 0.8), 0.2))
                    .sum()
            })
                                  .unwrap_or_else(|| Thing::new(vec![], vec![]))));
            if lane.construction.progress - 2.0 * CONSTRUCTION_ANIMATION_DELAY >
               lane.construction.length {
                world.send(collector_id, Freeze(lane.id()))
            }

            Fate::Live
        });

        each_t_lane.on(|&RenderToScene { renderer_id, scene_id }, lane, world| {
            let mut cars_iter = lane.microtraffic.cars.iter();
            let mut current_offset = 0.0;
            let mut car_instances = CVec::with_capacity(lane.microtraffic.cars.len());
            for segment in lane.construction.path.segments().iter() {
                for car in cars_iter.take_while_ref(|car| {
                    *car.position - current_offset < segment.length()
                }) {
                    let position2d = segment.along(*car.position - current_offset);
                    let direction = segment.direction_along(*car.position - current_offset);
                    let rotated_direction = (direction +
                                             0.3 * car.transfer_velocity * direction.orthogonal())
                            .normalize();
                    let shifted_position2d = position2d +
                                             2.5 * direction.orthogonal() * car.transfer_position;
                    car_instances.push(Instance {
                                           instance_position: [shifted_position2d.x,
                                                               shifted_position2d.y,
                                                               0.0],
                                           instance_direction: [rotated_direction.x,
                                                                rotated_direction.y],
                                           instance_color: if DEBUG_VIEW_LANDMARKS {
                                               ::core::colors::RANDOM_COLORS
                                                   [car.destination.landmark.sub_actor_id as usize %
                                               ::core::colors::RANDOM_COLORS.len()]
                                           } else {
                                               ::core::colors::RANDOM_COLORS
                                                   [car.trip.sub_actor_id as usize %
                                               ::core::colors::RANDOM_COLORS.len()]
                                           },
                                       })
                }
                current_offset += segment.length;
            }

            if DEBUG_VIEW_TRANSFER_OBSTACLES {
                for obstacle in &lane.microtraffic.left_obstacles {
                    let position2d = if *obstacle.position < lane.construction.length {
                        lane.construction.path.along(*obstacle.position)
                    } else {
                        lane.construction.path.end() +
                        (*obstacle.position - lane.construction.length) *
                        lane.construction.path.end_direction()
                    } -
                                     1.0 *
                                     lane.construction
                                         .path
                                         .direction_along(*obstacle.position)
                                         .orthogonal();
                    let direction = lane.construction
                        .path
                        .direction_along(*obstacle.position);

                    car_instances.push(Instance {
                                           instance_position: [position2d.x, position2d.y, 0.0],
                                           instance_direction: [direction.x, direction.y],
                                           instance_color: [1.0, 0.7, 0.7],
                                       });
                }

                for obstacle in &lane.microtraffic.right_obstacles {
                    let position2d = if *obstacle.position < lane.construction.length {
                        lane.construction.path.along(*obstacle.position)
                    } else {
                        lane.construction.path.end() +
                        (*obstacle.position - lane.construction.length) *
                        lane.construction.path.end_direction()
                    } +
                                     1.0 *
                                     lane.construction
                                         .path
                                         .direction_along(*obstacle.position)
                                         .orthogonal();
                    let direction = lane.construction
                        .path
                        .direction_along(*obstacle.position);

                    car_instances.push(Instance {
                                           instance_position: [position2d.x, position2d.y, 0.0],
                                           instance_direction: [direction.x, direction.y],
                                           instance_color: [1.0, 0.7, 0.7],
                                       });
                }
            }

            if !car_instances.is_empty() {
                world.send(renderer_id,
                           AddSeveralInstances {
                               scene_id: scene_id,
                               batch_id: 8000,
                               instances: car_instances,
                           });
            }

            if lane.connectivity.left.is_none() {
                let position = lane.construction
                    .path
                    .along(lane.construction.length / 2.0) +
                               lane.construction
                                   .path
                                   .direction_along(lane.construction.length / 2.0)
                                   .orthogonal();
                world.send(renderer_id,
                           AddInstance {
                               scene_id: scene_id,
                               batch_id: 1333,
                               instance: Instance {
                                   instance_position: [position.x, position.y, 0.0],
                                   instance_direction: [1.0, 0.0],
                                   instance_color: [1.0, 0.0, 0.0],
                               },
                           });
            }
            if lane.connectivity.right.is_none() {
                let position = lane.construction
                    .path
                    .along(lane.construction.length / 2.0) -
                               lane.construction
                                   .path
                                   .direction_along(lane.construction.length / 2.0)
                                   .orthogonal();
                world.send(renderer_id,
                           AddInstance {
                               scene_id: scene_id,
                               batch_id: 1333,
                               instance: Instance {
                                   instance_position: [position.x, position.y, 0.0],
                                   instance_direction: [1.0, 0.0],
                                   instance_color: [1.0, 0.0, 0.0],
                               },
                           });
            }
            Fate::Live
        })
    }));

    self::lane_thing_collector::setup::<LaneAsphalt>(system, [0.7, 0.7, 0.7], 2000, false);
    self::lane_thing_collector::setup::<LaneMarker>(system, [1.0, 1.0, 1.0], 2100, true);

    self::lane_thing_collector::setup::<TransferLaneMarkerGaps>(system,
                                                                [0.7, 0.7, 0.7],
                                                                2200,
                                                                true);
}

use self::lane_thing_collector::RenderToCollector;
use self::lane_thing_collector::Control::{Update, Freeze};

const CONSTRUCTION_ANIMATION_DELAY: f32 = 120.0;

use monet::RenderToScene;
use monet::AddInstance;
use monet::AddSeveralInstances;

const DEBUG_VIEW_LANDMARKS: bool = false;
const DEBUG_VIEW_SIGNALS: bool = false;
const DEBUG_VIEW_OBSTACLES: bool = false;
const DEBUG_VIEW_TRANSFER_OBSTACLES: bool = false;

use self::lane_thing_collector::Control::Remove;

pub fn on_build(lane: &Lane, world: &mut World) {
    let asphalt_coll_id = world.id::<ThingCollector<LaneAsphalt>>();
    let marker_coll_id = world.id::<ThingCollector<LaneMarker>>();
    world.send(lane.id(), RenderToCollector(asphalt_coll_id));
    if !lane.connectivity.on_intersection {
        world.send(lane.id(), RenderToCollector(marker_coll_id));
    }
}

pub fn on_build_transfer(lane: &TransferLane, world: &mut World) {
    let marker_coll_id = world.id::<ThingCollector<TransferLaneMarkerGaps>>();
    world.send(lane.id(), RenderToCollector(marker_coll_id));
}

pub fn on_unbuild(lane: &Lane, world: &mut World) {
    world.send_to_id_of::<ThingCollector<LaneAsphalt>, _>(Remove(lane.id()));
    if !lane.connectivity.on_intersection {
        world.send_to_id_of::<ThingCollector<LaneMarker>, _>(Remove(lane.id()));
    }

    if DEBUG_VIEW_LANDMARKS {
        // TODO: ugly
        world.send_to_id_of::<::monet::Renderer, _>(UpdateThing {
                                                        scene_id: 0,
                                                        thing_id: 4000 +
                                                                  lane.id().sub_actor_id as u16,
                                                        thing: Thing::new(vec![], vec![]),
                                                        instance: Instance::with_color([0.0, 0.0,
                                                                                        0.0]),
                                                        is_decal: true,
                                                    });
    }

    if DEBUG_VIEW_SIGNALS {
        world.send_to_id_of::<::monet::Renderer, _>(UpdateThing {
                                                        scene_id: 0,
                                                        thing_id: 4000 +
                                                                  lane.id().sub_actor_id as u16,
                                                        thing: Thing::new(vec![], vec![]),
                                                        instance: Instance::with_color([0.0, 0.0,
                                                                                        0.0]),
                                                        is_decal: true,
                                                    });
    }
}

pub fn on_unbuild_transfer(lane: &TransferLane, world: &mut World) {
    world.send_to_id_of::<ThingCollector<TransferLaneMarkerGaps>, _>(Remove(lane.id()));
}

#[derive(Clone)]
pub struct LaneAsphalt;
#[derive(Clone)]
pub struct LaneMarker;
#[derive(Clone)]
pub struct TransferLaneMarkerGaps;
