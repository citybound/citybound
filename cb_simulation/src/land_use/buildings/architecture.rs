use kay::{World, TypedID};
use descartes::{N, P2, WithUniqueOrthogonal};
use util::random::{Rng};
use michelangelo::{Vertex, Mesh};

use super::{Lot, BuildingStyle};

pub fn ideal_lot_shape(building_style: BuildingStyle) -> (f32, f32, f32) {
    match building_style {
        BuildingStyle::FamilyHouse => (20.0, 30.0, 0.5),
        BuildingStyle::GroceryShop => (15.0, 20.0, 0.5),
        BuildingStyle::Bakery => (20.0, 30.0, 0.5),
        BuildingStyle::Mill => (20.0, 30.0, 0.5),
        BuildingStyle::Field => (50.0, 100.0, 0.1),
        BuildingStyle::NeighboringTownConnection => (5.0, 5.0, 0.1),
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
}

pub const ALL_MATERIALS: [BuildingMaterial; 7] = [
    BuildingMaterial::WhiteWall,
    BuildingMaterial::TiledRoof,
    BuildingMaterial::FlatRoof,
    BuildingMaterial::FieldWheat,
    BuildingMaterial::FieldRows,
    BuildingMaterial::FieldPlant,
    BuildingMaterial::FieldMeadow,
];

impl ::std::fmt::Display for BuildingMaterial {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Debug::fmt(self, f)
    }
}

#[derive(Clone)]
pub struct BuildingMesh(pub ::std::collections::HashMap<BuildingMaterial, Mesh>);

pub fn build_building<R: Rng>(
    lot: &Lot,
    building_type: BuildingStyle,
    household_ids: &[::economy::households::HouseholdID],
    rng: &mut R,
    world: &mut World,
) -> BuildingMesh {
    let building_position = lot.center_point();
    let building_orientation = lot.best_road_connection().1;

    let (main_footprint, entrance_footprint) = generate_house_footprint(lot, rng);

    match building_type {
        BuildingStyle::FamilyHouse => {
            let height = 3.0 + 3.0 * rng.gen::<f32>();
            let entrance_height = 2.0 + rng.gen::<f32>();

            let (roof_brick_mesh, roof_wall_mesh) =
                main_footprint.open_gable_roof_mesh(height, 0.3);
            let (entrance_roof_brick_mesh, entrance_roof_wall_mesh) =
                entrance_footprint.open_gable_roof_mesh(entrance_height, 0.3);

            BuildingMesh(
                vec![
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
                ].into_iter()
                .collect(),
            )
        }
        BuildingStyle::GroceryShop => {
            let height = 3.0 + rng.gen::<f32>();
            let entrance_height = height - 0.7;

            BuildingMesh(
                vec![
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
                ].into_iter()
                .collect(),
            )
        }
        BuildingStyle::Field => {
            use ::economy::households::household_kinds::*;

            let material = if let Some(farm) = household_ids.get(0) {
                let farm_type_id = farm.as_raw().type_id;
                if farm_type_id == grain_farm::GrainFarmID::local_first(world).as_raw().type_id {
                    BuildingMaterial::FieldWheat
                } else if farm_type_id == vegetable_farm::VegetableFarmID::local_first(world)
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
            BuildingMesh(
                Some((material, Mesh::from_area(&lot.area)))
                    .into_iter()
                    .collect(),
            )
        }
        BuildingStyle::Mill => {
            let height = 3.0 + rng.gen::<f32>();
            let tower_height = 5.0 + rng.gen::<f32>();

            let (roof_brick_mesh, roof_wall_mesh) =
                main_footprint.open_gable_roof_mesh(height, 0.3);
            let (tower_roof_brick_mesh, tower_roof_wall_mesh) =
                entrance_footprint.open_gable_roof_mesh(tower_height, 0.3);

            BuildingMesh(
                vec![
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
                ].into_iter()
                .collect(),
            )
        }
        BuildingStyle::Bakery => {
            let height = 3.0 + rng.gen::<f32>();
            let entrance_height = height;

            let (entrance_roof_brick_mesh, entrance_roof_wall_mesh) =
                entrance_footprint.open_gable_roof_mesh(entrance_height, 0.3);

            BuildingMesh(
                vec![
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
                ].into_iter()
                .collect(),
            )
        }
        BuildingStyle::NeighboringTownConnection => {
            let length = 20.0;
            let building_orientation_orth = building_orientation.orthogonal_right();

            let vertices = vec![
                building_position - length / 4.0 * building_orientation,
                building_position + length / 2.0 * building_orientation_orth,
                building_position + length / 4.0 * building_orientation,
                building_position - length / 2.0 * building_orientation_orth,
            ].into_iter()
            .map(|v| Vertex {
                position: [v.x, v.y, 3.0],
            }).collect();

            let indices = vec![0, 1, 2, 2, 3, 0];

            BuildingMesh(
                Some((BuildingMaterial::WhiteWall, Mesh::new(vertices, indices)))
                    .into_iter()
                    .collect(),
            )
        }
    }
}

pub struct Footprint {
    back_right: P2,
    back_left: P2,
    front_right: P2,
    front_left: P2,
}

impl Footprint {
    fn wall_mesh(&self, wall_height: N) -> Mesh {
        let vertices = vec![
            Vertex {
                position: [self.back_right.x, self.back_right.y, 0.0],
            },
            Vertex {
                position: [self.back_left.x, self.back_left.y, 0.0],
            },
            Vertex {
                position: [self.front_left.x, self.front_left.y, 0.0],
            },
            Vertex {
                position: [self.front_right.x, self.front_right.y, 0.0],
            },
            Vertex {
                position: [self.back_right.x, self.back_right.y, wall_height],
            },
            Vertex {
                position: [self.back_left.x, self.back_left.y, wall_height],
            },
            Vertex {
                position: [self.front_left.x, self.front_left.y, wall_height],
            },
            Vertex {
                position: [self.front_right.x, self.front_right.y, wall_height],
            },
        ];

        let indices = vec![
            0, 1, 4, 1, 5, 4, 1, 2, 5, 2, 6, 5, 2, 3, 6, 3, 7, 6, 3, 0, 7, 0, 4, 7,
        ];

        Mesh::new(vertices, indices)
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
}

pub fn generate_house_footprint<R: Rng>(lot: &Lot, rng: &mut R) -> (Footprint, Footprint) {
    let building_position = lot.center_point();
    let building_orientation = lot.best_road_connection().1;
    let building_orientation_orth = building_orientation.orthogonal_right();

    let footprint_width = 10.0 + rng.gen::<f32>() * 7.0;
    let footprint_depth = 7.0 + rng.gen::<f32>() * 5.0;

    let entrance_position = building_position
        + building_orientation_orth * (0.5 - 1.0 * rng.gen::<f32>()) * footprint_width
        + building_orientation * (rng.gen::<f32>() * 0.3 + 0.1) * footprint_depth;
    let entrance_width = 5.0 + rng.gen::<f32>() * 4.0;
    let entrance_depth = 3.0 + rng.gen::<f32>() * 3.0;

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
