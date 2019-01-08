use kay::{World, TypedID};
use descartes::{N, P2, V2, WithUniqueOrthogonal, LinePath, ClosedLinePath, PrimitiveArea, Area};
use util::random::{Rng, seed};
use michelangelo::{Vertex, Mesh, Instance, FlatSurface, Sculpture};
use std::collections::HashMap;

use super::{Lot, BuildingStyle};

pub fn ideal_lot_shape(building_style: BuildingStyle) -> (N, N, N) {
    match building_style {
        BuildingStyle::FamilyHouse => (20.0, 30.0, 0.5),
        BuildingStyle::GroceryShop => (15.0, 20.0, 0.5),
        BuildingStyle::Bakery => (20.0, 30.0, 0.5),
        BuildingStyle::Mill => (20.0, 30.0, 0.5),
        BuildingStyle::Field => (50.0, 100.0, 0.1),
        BuildingStyle::NeighboringTownConnection => (5.0, 5.0, 0.1),
    }
}

fn footprint_dimensions(building_style: BuildingStyle) -> (N, N) {
    match building_style {
        BuildingStyle::FamilyHouse => (12.0, 8.0),
        _ => (15.0, 10.0),
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum BuildingMaterial {
    WhiteWall,
    TiledRoof,
    FlatRoof,
    FieldWheat,
    FieldRows,
    FieldPlant,
    FieldMeadow,
    WoodenFence,
    MetalFence,
    LotAsphalt,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum BuildingProp {
    SmallWindow,
    ShopWindowGlass,
    ShopWindowBanner,
    NarrowDoor,
    WideDoor,
}

impl ::std::fmt::Display for BuildingProp {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Debug::fmt(self, f)
    }
}

pub const ALL_MATERIALS: [BuildingMaterial; 10] = [
    BuildingMaterial::WhiteWall,
    BuildingMaterial::TiledRoof,
    BuildingMaterial::FlatRoof,
    BuildingMaterial::FieldWheat,
    BuildingMaterial::FieldRows,
    BuildingMaterial::FieldPlant,
    BuildingMaterial::FieldMeadow,
    BuildingMaterial::WoodenFence,
    BuildingMaterial::MetalFence,
    BuildingMaterial::LotAsphalt,
];

pub const ALL_PROP_TYPES: [BuildingProp; 5] = [
    BuildingProp::SmallWindow,
    BuildingProp::ShopWindowGlass,
    BuildingProp::ShopWindowBanner,
    BuildingProp::NarrowDoor,
    BuildingProp::WideDoor,
];

impl ::std::fmt::Display for BuildingMaterial {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Debug::fmt(self, f)
    }
}

#[derive(Clone)]
pub struct BuildingMesh {
    pub meshes: HashMap<BuildingMaterial, Mesh>,
    pub props: HashMap<BuildingProp, Vec<Instance>>,
}

pub fn footprint_area(lot: &Lot, building_style: BuildingStyle, extra_padding: N) -> Area {
    if let BuildingStyle::Field = building_style {
        lot.area.clone()
    } else {
        // TODO keep original building if lot changes
        let mut rng = seed(lot.original_lot_id);

        let (base_width, base_depth) = footprint_dimensions(building_style);

        let (main_footprint, _entrance_footprint) =
            generate_house_footprint(lot, base_width, base_depth, extra_padding, &mut rng);

        Area::new(vec![main_footprint.as_primitive_area()].into())
    }
}

pub fn build_building(
    lot: &Lot,
    building_style: BuildingStyle,
    household_ids: &[::economy::households::HouseholdID],
    world: &mut World,
) -> BuildingMesh {
    // TODO keep original building if lot changes
    let mut rng = seed(lot.original_lot_id);

    let (base_width, base_depth) = footprint_dimensions(building_style);

    let (main_footprint, entrance_footprint) =
        generate_house_footprint(lot, base_width, base_depth, 0.0, &mut rng);

    match building_style {
        BuildingStyle::FamilyHouse => {
            let height = 2.6 + 2.0 * rng.gen::<f32>();
            let entrance_height = 2.0 + rng.gen::<f32>();

            let (roof_brick_mesh, roof_wall_mesh) =
                main_footprint.open_gable_roof_mesh(height, 0.3);
            let (entrance_roof_brick_mesh, entrance_roof_wall_mesh) =
                entrance_footprint.open_gable_roof_mesh(entrance_height, 0.3);

            let (fence_surface, _) = FlatSurface::from_band(
                lot.area.primitives[0].boundary.path().clone(),
                0.1,
                0.1,
                0.0,
            )
            .extrude(1.0, 0.0);

            let fence_mesh = Sculpture::new(vec![fence_surface.into()]).to_mesh();

            BuildingMesh {
                meshes: vec![
                    (
                        BuildingMaterial::WhiteWall,
                        main_footprint.wall_mesh(height)
                            + entrance_footprint.wall_mesh(entrance_height)
                            + roof_wall_mesh
                            + entrance_roof_wall_mesh,
                    ),
                    (
                        BuildingMaterial::TiledRoof,
                        roof_brick_mesh + entrance_roof_brick_mesh,
                    ),
                    (BuildingMaterial::WoodenFence, fence_mesh),
                ]
                .into_iter()
                .collect(),
                props: vec![
                    (
                        BuildingProp::SmallWindow,
                        main_footprint
                            .distribute_along_walls(3.0)
                            .into_iter()
                            .map(|(position, direction)| Instance {
                                instance_position: [position.x, position.y, 0.0],
                                instance_direction: [direction.x, direction.y],
                                instance_color: [0.7, 0.6, 0.6],
                            })
                            .collect(),
                    ),
                    (
                        BuildingProp::NarrowDoor,
                        vec![{
                            let position = P2::from_coordinates(
                                (entrance_footprint.front_right.coords
                                    + entrance_footprint.back_right.coords)
                                    / 2.0,
                            );
                            let direction = (entrance_footprint.back_right
                                - entrance_footprint.front_right)
                                .normalize();
                            Instance {
                                instance_position: [position.x, position.y, 0.0],
                                instance_direction: [direction.x, direction.y],
                                instance_color: [0.6, 0.5, 0.5],
                            }
                        }],
                    ),
                ]
                .into_iter()
                .collect(),
            }
        }
        BuildingStyle::GroceryShop => {
            let height = 3.0 + rng.gen::<f32>();
            let entrance_height = height - 0.7;
            let business_color = [
                rng.gen_range(0.3, 0.6),
                rng.gen_range(0.3, 0.6),
                rng.gen_range(0.3, 0.6),
            ];

            BuildingMesh {
                meshes: vec![
                    (
                        BuildingMaterial::WhiteWall,
                        main_footprint.wall_mesh(height)
                            + entrance_footprint.wall_mesh(entrance_height),
                    ),
                    (
                        BuildingMaterial::FlatRoof,
                        main_footprint.flat_roof_mesh(height)
                            + entrance_footprint.flat_roof_mesh(entrance_height),
                    ),
                ]
                .into_iter()
                .collect(),
                props: vec![
                    (
                        BuildingProp::ShopWindowGlass,
                        main_footprint
                            .distribute_along_walls(3.0)
                            .into_iter()
                            .map(|(position, direction)| Instance {
                                instance_position: [position.x, position.y, 0.0],
                                instance_direction: [direction.x, direction.y],
                                instance_color: [0.7, 0.6, 0.6],
                            })
                            .collect(),
                    ),
                    (
                        BuildingProp::ShopWindowBanner,
                        main_footprint
                            .distribute_along_walls(3.0)
                            .into_iter()
                            .map(|(position, direction)| Instance {
                                instance_position: [position.x, position.y, 0.0],
                                instance_direction: [direction.x, direction.y],
                                instance_color: business_color,
                            })
                            .collect(),
                    ),
                    (
                        BuildingProp::WideDoor,
                        vec![{
                            let position = P2::from_coordinates(
                                (entrance_footprint.front_right.coords
                                    + entrance_footprint.back_right.coords)
                                    / 2.0,
                            );
                            let direction = (entrance_footprint.back_right
                                - entrance_footprint.front_right)
                                .normalize();
                            Instance {
                                instance_position: [position.x, position.y, 0.0],
                                instance_direction: [direction.x, direction.y],
                                instance_color: [0.6, 0.5, 0.5],
                            }
                        }],
                    ),
                ]
                .into_iter()
                .collect(),
            }
        }
        BuildingStyle::Field => {
            use ::economy::households::household_kinds::*;

            let material = if let Some(farm) = household_ids.get(0) {
                let farm_type_id = farm.as_raw().type_id;
                if farm_type_id == grain_farm::GrainFarmID::local_first(world).as_raw().type_id {
                    BuildingMaterial::FieldWheat
                } else if farm_type_id
                    == vegetable_farm::VegetableFarmID::local_first(world)
                        .as_raw()
                        .type_id
                {
                    BuildingMaterial::FieldPlant
                } else {
                    BuildingMaterial::FieldMeadow
                }
            } else {
                BuildingMaterial::FieldRows
            };

            let lot_surface = FlatSurface::from_primitive_area(lot.area.primitives[0].clone(), 0.0);
            let (_, shrunk_lot_surface) = lot_surface.extrude(0.0, 2.0);

            BuildingMesh {
                meshes: Some((
                    material,
                    Sculpture::new(vec![shrunk_lot_surface.into()]).to_mesh(),
                ))
                .into_iter()
                .collect(),
                props: HashMap::new(),
            }
        }
        BuildingStyle::Mill => {
            let height = 3.0 + rng.gen::<f32>();
            let tower_height = 5.0 + rng.gen::<f32>();

            let (roof_brick_mesh, roof_wall_mesh) =
                main_footprint.open_gable_roof_mesh(height, 0.3);
            let (tower_roof_brick_mesh, tower_roof_wall_mesh) =
                entrance_footprint.open_gable_roof_mesh(tower_height, 0.3);

            BuildingMesh {
                meshes: vec![
                    (
                        BuildingMaterial::WhiteWall,
                        main_footprint.wall_mesh(height)
                            + entrance_footprint.wall_mesh(tower_height)
                            + roof_wall_mesh
                            + tower_roof_wall_mesh,
                    ),
                    (
                        BuildingMaterial::FlatRoof,
                        roof_brick_mesh + tower_roof_brick_mesh,
                    ),
                ]
                .into_iter()
                .collect(),
                props: vec![(
                    BuildingProp::WideDoor,
                    vec![{
                        let position = P2::from_coordinates(
                            (entrance_footprint.front_right.coords
                                + entrance_footprint.back_right.coords)
                                / 2.0,
                        );
                        let direction = (entrance_footprint.back_right
                            - entrance_footprint.front_right)
                            .normalize();
                        Instance {
                            instance_position: [position.x, position.y, 0.0],
                            instance_direction: [direction.x, direction.y],
                            instance_color: [0.6, 0.5, 0.5],
                        }
                    }],
                )]
                .into_iter()
                .collect(),
            }
        }
        BuildingStyle::Bakery => {
            let height = 3.0 + rng.gen::<f32>();
            let entrance_height = height;
            let business_color = [
                rng.gen_range(0.3, 0.6),
                rng.gen_range(0.3, 0.6),
                rng.gen_range(0.3, 0.6),
            ];

            let (entrance_roof_brick_mesh, entrance_roof_wall_mesh) =
                entrance_footprint.open_gable_roof_mesh(entrance_height, 0.3);

            BuildingMesh {
                meshes: vec![
                    (
                        BuildingMaterial::WhiteWall,
                        main_footprint.wall_mesh(height)
                            + entrance_footprint.wall_mesh(entrance_height)
                            + entrance_roof_wall_mesh,
                    ),
                    (BuildingMaterial::TiledRoof, entrance_roof_brick_mesh),
                    (
                        BuildingMaterial::FlatRoof,
                        main_footprint.flat_roof_mesh(height),
                    ),
                ]
                .into_iter()
                .collect(),
                props: vec![
                    (
                        BuildingProp::ShopWindowGlass,
                        main_footprint
                            .distribute_along_walls(3.0)
                            .into_iter()
                            .map(|(position, direction)| Instance {
                                instance_position: [position.x, position.y, 0.0],
                                instance_direction: [direction.x, direction.y],
                                instance_color: [0.7, 0.6, 0.6],
                            })
                            .collect(),
                    ),
                    (
                        BuildingProp::ShopWindowBanner,
                        main_footprint
                            .distribute_along_walls(3.0)
                            .into_iter()
                            .map(|(position, direction)| Instance {
                                instance_position: [position.x, position.y, 0.0],
                                instance_direction: [direction.x, direction.y],
                                instance_color: business_color,
                            })
                            .collect(),
                    ),
                    (
                        BuildingProp::WideDoor,
                        vec![{
                            let position = P2::from_coordinates(
                                (entrance_footprint.front_right.coords
                                    + entrance_footprint.back_right.coords)
                                    / 2.0,
                            );
                            let direction = (entrance_footprint.back_right
                                - entrance_footprint.front_right)
                                .normalize();
                            Instance {
                                instance_position: [position.x, position.y, 0.0],
                                instance_direction: [direction.x, direction.y],
                                instance_color: [0.6, 0.5, 0.5],
                            }
                        }],
                    ),
                ]
                .into_iter()
                .collect(),
            }
        }
        BuildingStyle::NeighboringTownConnection => BuildingMesh {
            meshes: Some((
                BuildingMaterial::WhiteWall,
                Mesh::from_area(&lot.original_area),
            ))
            .into_iter()
            .collect(),
            props: HashMap::new(),
        },
    }
}

pub struct Footprint {
    back_right: P2,
    back_left: P2,
    front_right: P2,
    front_left: P2,
}

impl Footprint {
    fn as_primitive_area(&self) -> PrimitiveArea {
        PrimitiveArea::new(
            ClosedLinePath::new(
                LinePath::new(
                    vec![
                        self.back_right,
                        self.back_left,
                        self.front_left,
                        self.front_right,
                        self.back_right,
                    ]
                    .into(),
                )
                .unwrap(),
            )
            .unwrap(),
        )
    }

    fn wall_mesh(&self, wall_height: N) -> Mesh {
        let footprint_surface = FlatSurface::from_primitive_area(self.as_primitive_area(), 0.0);

        let wall_surface = footprint_surface.extrude(wall_height, 0.0).0;

        Sculpture::new(vec![wall_surface.into()]).to_mesh()
    }

    fn flat_roof_mesh(&self, base_height: N) -> Mesh {
        let vertices = vec![
            Vertex {
                position: [self.back_right.x, self.back_right.y, base_height],
            },
            Vertex {
                position: [self.back_left.x, self.back_left.y, base_height],
            },
            Vertex {
                position: [self.front_left.x, self.front_left.y, base_height],
            },
            Vertex {
                position: [self.front_right.x, self.front_right.y, base_height],
            },
        ];

        let indices = vec![0, 1, 3, 1, 2, 3];

        Mesh::new(vertices, indices)
    }

    fn open_gable_roof_mesh(&self, base_height: N, angle: N) -> (Mesh, Mesh) {
        let roof_height = (self.back_right - self.front_right).norm() * angle.sin();
        let mid_right = (self.back_right + self.front_right.coords) / 2.0;
        let mid_left = (self.back_left + self.front_left.coords) / 2.0;

        let vertices = vec![
            Vertex {
                position: [self.back_right.x, self.back_right.y, base_height],
            },
            Vertex {
                position: [self.back_left.x, self.back_left.y, base_height],
            },
            Vertex {
                position: [mid_left.x, mid_left.y, base_height + roof_height],
            },
            Vertex {
                position: [self.front_left.x, self.front_left.y, base_height],
            },
            Vertex {
                position: [self.front_right.x, self.front_right.y, base_height],
            },
            Vertex {
                position: [mid_right.x, mid_right.y, base_height + roof_height],
            },
        ];

        let roof_indices = vec![0, 1, 2, 2, 5, 0, 2, 3, 4, 4, 5, 2];

        let wall_indices = vec![1, 2, 3, 4, 5, 0];

        (
            Mesh::new(vertices.clone(), roof_indices),
            Mesh::new(vertices, wall_indices),
        )
    }

    fn distribute_along_walls(&self, spacing: N) -> Vec<(P2, V2)> {
        [
            self.back_right,
            self.back_left,
            self.front_left,
            self.front_right,
            self.back_right,
        ]
        .windows(2)
        .flat_map(|corner_pair| {
            let wall_length = (corner_pair[1] - corner_pair[0]).norm();
            let available_wall_length = wall_length - spacing;

            if available_wall_length > 0.0 {
                let n_subdivisions = (available_wall_length / spacing).floor();
                let actual_spacing = available_wall_length / n_subdivisions;
                let wall_path = LinePath::new(vec![corner_pair[0], corner_pair[1]].into()).unwrap();
                (0..(n_subdivisions as usize))
                    .into_iter()
                    .map(|i| {
                        let distance_along = spacing + i as f32 * actual_spacing;
                        (
                            wall_path.along(distance_along),
                            wall_path.direction_along(distance_along),
                        )
                    })
                    .collect()
            } else {
                vec![]
            }
        })
        .collect()
    }
}

pub fn generate_house_footprint<R: Rng>(
    lot: &Lot,
    base_width: N,
    base_depth: N,
    extra_padding: N,
    rng: &mut R,
) -> (Footprint, Footprint) {
    let building_position = lot.center_point();
    let building_orientation = lot.best_road_connection().1;
    let building_orientation_orth = building_orientation.orthogonal_right();

    let footprint_width = base_width * rng.gen_range(0.7, 1.3) + 2.0 * extra_padding;
    let footprint_depth = base_depth * rng.gen_range(0.7, 1.3) + 2.0 * extra_padding;

    let entrance_position = building_position
        + building_orientation_orth * rng.gen_range(-0.5, 0.5) * footprint_width
        + building_orientation * rng.gen_range(0.1, 0.4) * footprint_depth;
    let entrance_width = footprint_width * rng.gen_range(0.5, 0.7);
    let entrance_depth = footprint_depth * rng.gen_range(0.3, 0.7);

    (
        Footprint {
            back_right: building_position + building_orientation_orth * footprint_width / 2.0
                - building_orientation * footprint_depth / 2.0,
            back_left: building_position
                - building_orientation_orth * footprint_width / 2.0
                - building_orientation * footprint_depth / 2.0,
            front_left: building_position - building_orientation_orth * footprint_width / 2.0
                + building_orientation * footprint_depth / 2.0,
            front_right: building_position
                + building_orientation_orth * footprint_width / 2.0
                + building_orientation * footprint_depth / 2.0,
        },
        Footprint {
            back_right: entrance_position
                + building_orientation * entrance_width / 2.0
                + building_orientation_orth * entrance_depth / 2.0,
            back_left: entrance_position - building_orientation * entrance_width / 2.0
                + building_orientation_orth * entrance_depth / 2.0,
            front_left: entrance_position
                - building_orientation * entrance_width / 2.0
                - building_orientation_orth * entrance_depth / 2.0,
            front_right: entrance_position + building_orientation * entrance_width / 2.0
                - building_orientation_orth * entrance_depth / 2.0,
        },
    )
}
