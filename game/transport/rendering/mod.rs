use descartes::{Band, FiniteCurve, WithUniqueOrthogonal, Path, RoughlyComparable};
use compact::CVec;
use kay::{ActorSystem, World, Actor, TypedID};
use monet::{Instance, Vertex, Geometry, Renderer, RendererID};
use stagemaster::geometry::{band_to_geometry, dash_path};
use super::lane::{Lane, LaneID, TransferLane, TransferLaneID};
use style::colors;
use itertools::Itertools;

#[path = "./resources/car.rs"]
mod car;

#[path = "./resources/traffic_light.rs"]
mod traffic_light;

use monet::{Renderable, RenderableID, GrouperID, GrouperIndividual, GrouperIndividualID};

const LANE_ASPHALT_THING_ID: u16 = 2000;
const LANE_MARKER_THING_ID: u16 = 2200;
const LANE_MARKER_GAPS_THING_ID: u16 = 2400;

impl Renderable for Lane {
    fn setup_in_scene(&mut self, _renderer_id: RendererID, _: &mut World) {}

    #[allow(cyclomatic_complexity)]
    fn render_to_scene(&mut self, renderer_id: RendererID, frame: usize, world: &mut World) {
        let mut cars_iter = self.microtraffic.cars.iter();
        let mut current_offset = 0.0;
        let mut car_instances = CVec::with_capacity(self.microtraffic.cars.len());
        for segment in self.construction.path.segments().iter() {
            for car in cars_iter.take_while_ref(|car| {
                *car.position - current_offset < segment.length()
            })
            {
                let position2d = segment.along(*car.position - current_offset);
                let direction = segment.direction_along(*car.position - current_offset);
                car_instances.push(Instance {
                    instance_position: [position2d.x, position2d.y, 0.0],
                    instance_direction: [direction.x, direction.y],
                    instance_color: if DEBUG_VIEW_LANDMARKS {
                        colors::RANDOM_COLORS[car.destination.landmark.as_raw().instance_id as
                                                  usize %
                                                  colors::RANDOM_COLORS.len()]
                    } else {
                        colors::RANDOM_COLORS[car.trip.as_raw().instance_id as usize %
                                                  colors::RANDOM_COLORS.len()]
                    },
                })
            }
            current_offset += segment.length;
        }

        if DEBUG_VIEW_OBSTACLES {
            for &(obstacle, _id) in &self.microtraffic.obstacles {
                let position2d = if *obstacle.position < self.construction.length {
                    self.construction.path.along(*obstacle.position)
                } else {
                    self.construction.path.end() +
                        (*obstacle.position - self.construction.length) *
                            self.construction.path.end_direction()
                };
                let direction = self.construction.path.direction_along(*obstacle.position);

                car_instances.push(Instance {
                    instance_position: [position2d.x, position2d.y, 0.0],
                    instance_direction: [direction.x, direction.y],
                    instance_color: [1.0, 0.0, 0.0],
                });
            }
        }

        if !car_instances.is_empty() {
            renderer_id.add_several_instances(8000, frame, car_instances, world);
        }
        // no traffic light for u-turn
        if self.connectivity.on_intersection &&
            !self.construction.path.end_direction().is_roughly_within(
                -self.construction.path.start_direction(),
                0.1,
            )
        {
            let mut position = self.construction.path.start();
            let (position_shift, batch_id) =
                if !self.construction.path.start_direction().is_roughly_within(
                    self.construction
                        .path
                        .end_direction(),
                    0.5,
                )
                {
                    let dot = self.construction.path.end_direction().dot(
                        &self.construction
                            .path
                            .start_direction()
                            .orthogonal(),
                    );
                    let shift = if dot > 0.0 { 1.0 } else { -1.0 };
                    let batch_id = if dot > 0.0 { 8004 } else { 8003 };
                    (shift, batch_id)
                } else {
                    (0.0, 8002)
                };
            position += self.construction.path.start_direction().orthogonal() * position_shift;
            let direction = self.construction.path.start_direction();

            let instance = Instance {
                instance_position: [position.x, position.y, 6.0],
                instance_direction: [direction.x, direction.y],
                instance_color: [0.1, 0.1, 0.1],
            };
            renderer_id.add_instance(8001, frame, instance, world);

            if self.microtraffic.yellow_to_red && self.microtraffic.green {
                let instance = Instance {
                    instance_position: [position.x, position.y, 6.7],
                    instance_direction: [direction.x, direction.y],
                    instance_color: [1.0, 0.8, 0.0],
                };
                renderer_id.add_instance(batch_id, frame, instance, world)
            } else if self.microtraffic.green {
                let instance = Instance {
                    instance_position: [position.x, position.y, 6.1],
                    instance_direction: [direction.x, direction.y],
                    instance_color: [0.0, 1.0, 0.2],
                };
                renderer_id.add_instance(batch_id, frame, instance, world)
            }

            if !self.microtraffic.green {
                let instance = Instance {
                    instance_position: [position.x, position.y, 7.3],
                    instance_direction: [direction.x, direction.y],
                    instance_color: [1.0, 0.0, 0.0],
                };
                renderer_id.add_instance(batch_id, frame, instance, world);

                if self.microtraffic.yellow_to_green {
                    let instance = Instance {
                        instance_position: [position.x, position.y, 6.7],
                        instance_direction: [direction.x, direction.y],
                        instance_color: [1.0, 0.8, 0.0],
                    };
                    renderer_id.add_instance(batch_id, frame, instance, world)
                }
            }
        }

        if DEBUG_VIEW_SIGNALS && self.connectivity.on_intersection {
            let geometry = band_to_geometry(
                &Band::new(self.construction.path.clone(), 0.3),
                if self.microtraffic.green { 0.4 } else { 0.2 },
            );
            let instance = Instance::with_color(if self.microtraffic.green {
                [0.0, 1.0, 0.0]
            } else {
                [1.0, 0.0, 0.0]
            });
            renderer_id.update_individual(
                4000 + self.id.as_raw().instance_id as u16,
                geometry,
                instance,
                true,
                world,
            );
        }

        // let has_next = self.connectivity.interactions.iter().any(|inter| {
        //     match inter.kind {
        //         InteractionKind::Next { .. } => true,
        //         _ => false,
        //     }
        // });
        // if !has_next {
        //     let instance = Instance {
        //         instance_position: [
        //             self.construction.path.end().x,
        //             self.construction.path.end().y,
        //             0.5,
        //         ],
        //         instance_direction: [1.0, 0.0],
        //         instance_color: [1.0, 0.0, 0.0],
        //     };
        //     renderer_id.add_instance( 1333, frame, instance, world);
        // }

        // let has_previous = self.connectivity.interactions.iter().any(|inter| {
        //     match inter.kind {
        //         InteractionKind::Previous { .. } => true,
        //         _ => false,
        //     }
        // });
        // if !has_previous {
        //     let instance = Instance {
        //         instance_position: [
        //             self.construction.path.start().x,
        //             self.construction.path.start().y,
        //             0.5,
        //         ],
        //         instance_direction: [1.0, 0.0],
        //         instance_color: [0.0, 1.0, 0.0],
        //     };
        //     renderer_id.add_instance( 1333, frame, instance, world);
        // }

        if DEBUG_VIEW_LANDMARKS && self.pathfinding.routes_changed {
            let (random_color, is_landmark) = if let Some(location) = self.pathfinding.location {
                let random_color: [f32; 3] =
                    colors::RANDOM_COLORS[location.landmark.as_raw().instance_id as usize %
                                              colors::RANDOM_COLORS.len()];
                let weaker_random_color = [
                    (random_color[0] + 1.0) / 2.0,
                    (random_color[1] + 1.0) / 2.0,
                    (random_color[2] + 1.0) / 2.0,
                ];
                (weaker_random_color, location.is_landmark())
            } else {
                ([1.0, 1.0, 1.0], false)
            };

            let instance = band_to_geometry(
                &Band::new(
                    self.construction.path.clone(),
                    if is_landmark { 2.5 } else { 1.0 },
                ),
                0.4,
            );
            renderer_id.update_individual(
                4000 + self.id.as_raw().instance_id as u16,
                instance,
                Instance::with_color(random_color),
                true,
                world,
            );
        }

        use super::pathfinding::DEBUG_VIEW_CONNECTIVITY;

        if DEBUG_VIEW_CONNECTIVITY {
            if !self.pathfinding.debug_highlight_for.is_empty() {
                let (random_color, is_landmark) =
                    if let Some(location) = self.pathfinding.location {
                        let random_color: [f32; 3] = colors::RANDOM_COLORS
                            [location.landmark.as_raw().instance_id as usize %
                            colors::RANDOM_COLORS.len()];
                        (random_color, location.is_landmark())
                    } else {
                        ([1.0, 1.0, 1.0], false)
                    };

                let geometry = band_to_geometry(
                    &Band::new(
                        self.construction.path.clone(),
                        if is_landmark { 2.5 } else { 1.0 },
                    ),
                    0.4,
                );
                renderer_id.update_individual(
                    40_000 + self.id.as_raw().instance_id as u16,
                    geometry,
                    Instance::with_color(random_color),
                    true,
                    world,
                );
            } else {
                renderer_id.update_individual(
                    40_000 + self.id.as_raw().instance_id as u16,
                    Geometry::empty(),
                    Instance::with_color([0.0, 0.0, 0.0]),
                    true,
                    world,
                );
            }
        }
    }
}

impl GrouperIndividual for Lane {
    fn render_to_grouper(
        &mut self,
        grouper: GrouperID,
        base_individual_id: u16,
        world: &mut World,
    ) {
        let maybe_path = if self.construction.progress - CONSTRUCTION_ANIMATION_DELAY <
            self.construction.length
        {
            self.construction.path.subsection(
                0.0,
                (self.construction.progress - CONSTRUCTION_ANIMATION_DELAY)
                    .max(0.0),
            )
        } else {
            Some(self.construction.path.clone())
        };
        if base_individual_id == LANE_ASPHALT_THING_ID {
            grouper.update(
                self.id_as(),
                maybe_path
                    .map(|path| {
                        band_to_geometry(
                            &Band::new(path, 6.0),
                            if self.connectivity.on_intersection {
                                0.2
                            } else {
                                0.0
                            },
                        )
                    })
                    .unwrap_or_else(Geometry::empty),
                world,
            );
            if self.construction.progress - CONSTRUCTION_ANIMATION_DELAY >
                self.construction.length
            {
                grouper.freeze(self.id_as(), world);
            }
        } else {
            let left_marker = maybe_path
                .clone()
                .and_then(|path| path.shift_orthogonally(2.5))
                .map(|path| band_to_geometry(&Band::new(path, 0.6), 0.1))
                .unwrap_or_else(Geometry::empty);

            let right_marker = maybe_path
                .and_then(|path| path.shift_orthogonally(-2.5))
                .map(|path| band_to_geometry(&Band::new(path, 0.6), 0.1))
                .unwrap_or_else(Geometry::empty);
            grouper.update(self.id_as(), left_marker + right_marker, world);
            if self.construction.progress - CONSTRUCTION_ANIMATION_DELAY >
                self.construction.length
            {
                grouper.freeze(self.id_as(), world);
            }
        }
    }
}

impl Renderable for TransferLane {
    fn setup_in_scene(&mut self, _renderer_id: RendererID, _: &mut World) {}

    fn render_to_scene(&mut self, renderer_id: RendererID, frame: usize, world: &mut World) {
        let mut cars_iter = self.microtraffic.cars.iter();
        let mut current_offset = 0.0;
        let mut car_instances = CVec::with_capacity(self.microtraffic.cars.len());
        for segment in self.construction.path.segments().iter() {
            for car in cars_iter.take_while_ref(|car| {
                *car.position - current_offset < segment.length()
            })
            {
                let position2d = segment.along(*car.position - current_offset);
                let direction = segment.direction_along(*car.position - current_offset);
                let rotated_direction =
                    (direction + 0.3 * car.transfer_velocity * direction.orthogonal()).normalize();
                let shifted_position2d = position2d +
                    2.5 * direction.orthogonal() * car.transfer_position;
                car_instances.push(Instance {
                    instance_position: [shifted_position2d.x, shifted_position2d.y, 0.0],
                    instance_direction: [rotated_direction.x, rotated_direction.y],
                    instance_color: if DEBUG_VIEW_LANDMARKS {
                        colors::RANDOM_COLORS[car.destination.landmark.as_raw().instance_id as
                                                  usize %
                                                  colors::RANDOM_COLORS.len()]
                    } else {
                        colors::RANDOM_COLORS[car.trip.as_raw().instance_id as usize %
                                                  colors::RANDOM_COLORS.len()]
                    },
                })
            }
            current_offset += segment.length;
        }

        if DEBUG_VIEW_TRANSFER_OBSTACLES {
            for obstacle in &self.microtraffic.left_obstacles {
                let position2d = if *obstacle.position < self.construction.length {
                    self.construction.path.along(*obstacle.position)
                } else {
                    self.construction.path.end() +
                        (*obstacle.position - self.construction.length) *
                            self.construction.path.end_direction()
                } -
                    1.0 *
                        self.construction
                            .path
                            .direction_along(*obstacle.position)
                            .orthogonal();
                let direction = self.construction.path.direction_along(*obstacle.position);

                car_instances.push(Instance {
                    instance_position: [position2d.x, position2d.y, 0.0],
                    instance_direction: [direction.x, direction.y],
                    instance_color: [1.0, 0.7, 0.7],
                });
            }

            for obstacle in &self.microtraffic.right_obstacles {
                let position2d = if *obstacle.position < self.construction.length {
                    self.construction.path.along(*obstacle.position)
                } else {
                    self.construction.path.end() +
                        (*obstacle.position - self.construction.length) *
                            self.construction.path.end_direction()
                } +
                    1.0 *
                        self.construction
                            .path
                            .direction_along(*obstacle.position)
                            .orthogonal();
                let direction = self.construction.path.direction_along(*obstacle.position);

                car_instances.push(Instance {
                    instance_position: [position2d.x, position2d.y, 0.0],
                    instance_direction: [direction.x, direction.y],
                    instance_color: [1.0, 0.7, 0.7],
                });
            }
        }

        if !car_instances.is_empty() {
            renderer_id.add_several_instances(8000, frame, car_instances, world);
        }

        if self.connectivity.left.is_none() {
            let position = self.construction.path.along(self.construction.length / 2.0) +
                self.construction
                    .path
                    .direction_along(self.construction.length / 2.0)
                    .orthogonal();
            renderer_id.add_instance(
                1333,
                frame,
                Instance {
                    instance_position: [position.x, position.y, 0.0],
                    instance_direction: [1.0, 0.0],
                    instance_color: [1.0, 0.0, 0.0],
                },
                world,
            );
        }
        if self.connectivity.right.is_none() {
            let position = self.construction.path.along(self.construction.length / 2.0) -
                self.construction
                    .path
                    .direction_along(self.construction.length / 2.0)
                    .orthogonal();
            renderer_id.add_instance(
                1333,
                frame,
                Instance {
                    instance_position: [position.x, position.y, 0.0],
                    instance_direction: [1.0, 0.0],
                    instance_color: [1.0, 0.0, 0.0],
                },
                world,
            );
        }
    }
}

impl GrouperIndividual for TransferLane {
    fn render_to_grouper(
        &mut self,
        grouper: GrouperID,
        _base_individual_id: u16,
        world: &mut World,
    ) {
        let maybe_path = if self.construction.progress - 2.0 * CONSTRUCTION_ANIMATION_DELAY <
            self.construction.length
        {
            self.construction.path.subsection(
                0.0,
                (self.construction.progress - 2.0 * CONSTRUCTION_ANIMATION_DELAY)
                    .max(0.0),
            )
        } else {
            Some(self.construction.path.clone())
        };

        grouper.update(
            self.id_as(),
            maybe_path
                .map(|path| {
                    dash_path(&path, 2.0, 4.0)
                        .into_iter()
                        .map(|dash| band_to_geometry(&Band::new(dash, 0.8), 0.2))
                        .sum()
                })
                .unwrap_or_else(Geometry::empty),
            world,
        );
        if self.construction.progress - 2.0 * CONSTRUCTION_ANIMATION_DELAY >
            self.construction.length
        {
            grouper.freeze(self.id_as(), world);
        }
    }
}

pub fn setup(system: &mut ActorSystem) {

    system.register::<LaneRenderer>();

    auto_setup(system);

    let asphalt_group = GrouperID::spawn(
        colors::ASPHALT,
        LANE_ASPHALT_THING_ID,
        false,
        &mut system.world(),
    );

    let marker_group = GrouperID::spawn(
        colors::ROAD_MARKER,
        LANE_MARKER_THING_ID,
        true,
        &mut system.world(),
    );

    let gaps_group = GrouperID::spawn(
        colors::ASPHALT,
        LANE_MARKER_GAPS_THING_ID,
        true,
        &mut system.world(),
    );

    LaneRendererID::spawn(asphalt_group, marker_group, gaps_group, &mut system.world());
}

const CONSTRUCTION_ANIMATION_DELAY: f32 = 120.0;

const DEBUG_VIEW_LANDMARKS: bool = false;
const DEBUG_VIEW_SIGNALS: bool = false;
const DEBUG_VIEW_OBSTACLES: bool = false;
const DEBUG_VIEW_TRANSFER_OBSTACLES: bool = false;

#[derive(Compact, Clone)]
pub struct LaneRenderer {
    id: LaneRendererID,
    asphalt_grouper: GrouperID,
    marker_grouper: GrouperID,
    gaps_grouper: GrouperID,
}

impl Renderable for LaneRenderer {
    fn setup_in_scene(&mut self, renderer_id: RendererID, world: &mut World) {
        renderer_id.add_batch(8000, car::create(), world);
        renderer_id.add_batch(8001, traffic_light::create(), world);
        renderer_id.add_batch(8002, traffic_light::create_light(), world);
        renderer_id.add_batch(8003, traffic_light::create_light_left(), world);
        renderer_id.add_batch(8004, traffic_light::create_light_right(), world);

        renderer_id.add_batch(
            1333,
            Geometry::new(
                vec![
                    Vertex { position: [-1.0, -1.0, 0.0] },
                    Vertex { position: [1.0, -1.0, 0.0] },
                    Vertex { position: [1.0, 1.0, 0.0] },
                    Vertex { position: [-1.0, 1.0, 0.0] },
                ],
                vec![0, 1, 2, 2, 3, 0],
            ),
            world,
        );
    }

    fn render_to_scene(&mut self, renderer_id: RendererID, frame: usize, world: &mut World) {
        // Render a single invisible car to clean all instances every frame
        renderer_id.add_instance(
            8000,
            frame,
            Instance {
                instance_position: [-1000000.0, -1000000.0, -1000000.0],
                instance_direction: [0.0, 0.0],
                instance_color: [0.0, 0.0, 0.0],
            },
            world,
        );

        let lanes_as_renderables: RenderableID = Lane::local_broadcast(world).into();
        lanes_as_renderables.render_to_scene(renderer_id, frame, world);

        let transfer_lanes_as_renderables: RenderableID = TransferLane::local_broadcast(world)
            .into();
        transfer_lanes_as_renderables.render_to_scene(renderer_id, frame, world);
    }
}

impl LaneRenderer {
    pub fn spawn(
        id: LaneRendererID,
        asphalt_grouper: GrouperID,
        marker_grouper: GrouperID,
        gaps_grouper: GrouperID,
        _: &mut World,
    ) -> LaneRenderer {
        LaneRenderer {
            id,
            asphalt_grouper,
            marker_grouper,
            gaps_grouper,
        }
    }

    pub fn on_build(
        &mut self,
        lane: GrouperIndividualID,
        on_intersection: bool,
        world: &mut World,
    ) {
        self.asphalt_grouper.initial_add(lane, world);

        if !on_intersection {
            self.marker_grouper.initial_add(lane, world);
        }
    }

    pub fn on_build_transfer(&mut self, lane: GrouperIndividualID, world: &mut World) {
        self.gaps_grouper.initial_add(lane, world);
    }

    pub fn on_unbuild(
        &mut self,
        lane: GrouperIndividualID,
        on_intersection: bool,
        world: &mut World,
    ) {
        self.asphalt_grouper.remove(lane, world);

        if !on_intersection {
            self.marker_grouper.remove(lane, world);
        }
    }

    pub fn on_unbuild_transfer(&mut self, lane: GrouperIndividualID, world: &mut World) {
        self.gaps_grouper.remove(lane, world);
    }
}

pub fn on_build(lane: &Lane, world: &mut World) {
    LaneRenderer::local_first(world).on_build(
        lane.id_as(),
        lane.connectivity
            .on_intersection,
        world,
    );
}

pub fn on_build_transfer(lane: &TransferLane, world: &mut World) {
    LaneRenderer::local_first(world).on_build_transfer(lane.id_as(), world);
}

pub fn on_unbuild(lane: &Lane, world: &mut World) {
    LaneRenderer::local_first(world).on_unbuild(
        lane.id_as(),
        lane.connectivity
            .on_intersection,
        world,
    );

    if DEBUG_VIEW_LANDMARKS {
        // TODO: move this to LaneRenderer
        Renderer::local_first(world).update_individual(
            4000 + lane.id.as_raw().instance_id as u16,
            Geometry::empty(),
            Instance::with_color([0.0, 0.0, 0.0]),
            true,
            world,
        );
    }

    if DEBUG_VIEW_SIGNALS {
        Renderer::local_first(world).update_individual(
            4000 + lane.id.as_raw().instance_id as u16,
            Geometry::empty(),
            Instance::with_color([0.0, 0.0, 0.0]),
            true,
            world,
        );
    }
}

pub fn on_unbuild_transfer(lane: &TransferLane, world: &mut World) {
    LaneRenderer::local_first(world).on_unbuild_transfer(lane.id_as(), world);
}

mod kay_auto;
pub use self::kay_auto::*;
