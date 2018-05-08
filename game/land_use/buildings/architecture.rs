use descartes::{N, P2, WithUniqueOrthogonal};
use rand::Rng;
use monet::{Vertex, Geometry};

use super::{Lot, BuildingStyle};

pub fn ideal_lot_shape(building_style: BuildingStyle) -> (f32, f32) {
    match building_style {
        BuildingStyle::FamilyHouse => (15.0, 40.0),
        BuildingStyle::GroceryShop => (10.0, 30.0),
        BuildingStyle::Bakery => (15.0, 30.0),
        BuildingStyle::Mill => (15.0, 30.0),
        BuildingStyle::Field => (50.0, 100.0),
        BuildingStyle::NeighboringTownConnection => (5.0, 5.0),
    }
}

#[derive(Compact, Clone)]
pub struct BuildingGeometry {
    pub wall: Geometry,
    pub brick_roof: Geometry,
    pub flat_roof: Geometry,
    pub field: Geometry,
}

pub fn build_building<R: Rng>(
    lot: &Lot,
    building_type: BuildingStyle,
    rng: &mut R,
) -> BuildingGeometry {
    let building_position = lot.center_point;
    let building_orientation = (lot.connection_point - lot.center_point).normalize();

    let (main_footprint, entrance_footprint) = generate_house_footprint(lot, rng);

    match building_type {
        BuildingStyle::FamilyHouse => {
            let height = 3.0 + 3.0 * rng.next_f32();
            let entrance_height = 2.0 + rng.next_f32();

            let (roof_brick_geometry, roof_wall_geometry) =
                main_footprint.open_gable_roof_geometry(height, 0.3);
            let (entrance_roof_brick_geometry, entrance_roof_wall_geometry) =
                entrance_footprint.open_gable_roof_geometry(entrance_height, 0.3);

            BuildingGeometry {
                wall: main_footprint.wall_geometry(height) +
                    entrance_footprint.wall_geometry(entrance_height) +
                    roof_wall_geometry + entrance_roof_wall_geometry,
                brick_roof: roof_brick_geometry + entrance_roof_brick_geometry,
                flat_roof: Geometry::empty(),
                field: Geometry::empty(),
            }
        }
        BuildingStyle::GroceryShop => {
            let height = 3.0 + rng.next_f32();
            let entrance_height = height - 0.7;

            BuildingGeometry {
                wall: main_footprint.wall_geometry(height) +
                    entrance_footprint.wall_geometry(entrance_height),
                brick_roof: Geometry::empty(),
                flat_roof: main_footprint.flat_roof_geometry(height) +
                    entrance_footprint.flat_roof_geometry(entrance_height),
                field: Geometry::empty(),
            }
        }
        BuildingStyle::Field => {
            BuildingGeometry {
                wall: Geometry::empty(),
                brick_roof: Geometry::empty(),
                flat_roof: Geometry::empty(),
                field: main_footprint.scale(3.0).flat_roof_geometry(0.0),
            }
        }
        BuildingStyle::Mill => {
            let height = 3.0 + rng.next_f32();
            let tower_height = 5.0 + rng.next_f32();

            let (roof_brick_geometry, roof_wall_geometry) =
                main_footprint.open_gable_roof_geometry(height, 0.3);
            let (tower_roof_brick_geometry, tower_roof_wall_geometry) =
                entrance_footprint.open_gable_roof_geometry(tower_height, 0.3);

            BuildingGeometry {
                wall: main_footprint.wall_geometry(height) +
                    entrance_footprint.wall_geometry(tower_height) +
                    roof_wall_geometry + tower_roof_wall_geometry,
                brick_roof: Geometry::empty(),
                flat_roof: roof_brick_geometry + tower_roof_brick_geometry,
                field: Geometry::empty(),
            }
        }
        BuildingStyle::Bakery => {
            let height = 3.0 + rng.next_f32();
            let entrance_height = height;

            let (entrance_roof_brick_geometry, entrance_roof_wall_geometry) =
                entrance_footprint.open_gable_roof_geometry(entrance_height, 0.3);

            BuildingGeometry {
                wall: main_footprint.wall_geometry(height) +
                    entrance_footprint.wall_geometry(entrance_height) +
                    entrance_roof_wall_geometry,
                brick_roof: entrance_roof_brick_geometry,
                flat_roof: main_footprint.flat_roof_geometry(height),
                field: Geometry::empty(),
            }
        }
        BuildingStyle::NeighboringTownConnection => {
            let length = 100.0;
            let building_orientation_orth = building_orientation.orthogonal();

            let vertices = vec![
                building_position - length / 4.0 * building_orientation_orth,
                building_position + length / 2.0 * building_orientation,
                building_position + length / 4.0 * building_orientation_orth,
                building_position - length / 2.0 * building_orientation,
            ].into_iter()
                .map(|v| Vertex { position: [v.x, v.y, 3.0] })
                .collect();

            let indices = vec![0, 1, 2, 2, 3, 0];

            BuildingGeometry {
                wall: Geometry::new(vertices, indices),
                brick_roof: Geometry::empty(),
                flat_roof: Geometry::empty(),
                field: Geometry::empty(),
            }
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
    fn wall_geometry(&self, wall_height: N) -> Geometry {
        let vertices =
            vec![
                Vertex { position: [self.back_right.x, self.back_right.y, 0.0] },
                Vertex { position: [self.back_left.x, self.back_left.y, 0.0] },
                Vertex { position: [self.front_left.x, self.front_left.y, 0.0] },
                Vertex { position: [self.front_right.x, self.front_right.y, 0.0] },

                Vertex { position: [self.back_right.x, self.back_right.y, wall_height] },
                Vertex { position: [self.back_left.x, self.back_left.y, wall_height] },
                Vertex { position: [self.front_left.x, self.front_left.y, wall_height] },
                Vertex { position: [self.front_right.x, self.front_right.y, wall_height] },
            ];

        let indices = vec![
            0,
            1,
            4,
            1,
            5,
            4,
            1,
            2,
            5,
            2,
            6,
            5,
            2,
            3,
            6,
            3,
            7,
            6,
            3,
            0,
            7,
            0,
            4,
            7,
        ];

        Geometry::new(vertices, indices)
    }

    fn flat_roof_geometry(&self, base_height: N) -> Geometry {
        let vertices =
            vec![
                Vertex { position: [self.back_right.x, self.back_right.y, base_height] },
                Vertex { position: [self.back_left.x, self.back_left.y, base_height] },
                Vertex { position: [self.front_left.x, self.front_left.y, base_height] },
                Vertex { position: [self.front_right.x, self.front_right.y, base_height] },
            ];

        let indices = vec![0, 1, 3, 1, 2, 3];

        Geometry::new(vertices, indices)
    }

    fn open_gable_roof_geometry(&self, base_height: N, angle: N) -> (Geometry, Geometry) {
        let roof_height = (self.back_right - self.front_right).norm() * angle.sin();
        let mid_right = (self.back_right + self.front_right.coords) / 2.0;
        let mid_left = (self.back_left + self.front_left.coords) / 2.0;

        let vertices =
            vec![
                Vertex { position: [self.back_right.x, self.back_right.y, base_height] },
                Vertex { position: [self.back_left.x, self.back_left.y, base_height] },
                Vertex { position: [mid_left.x, mid_left.y, base_height + roof_height] },
                Vertex { position: [self.front_left.x, self.front_left.y, base_height] },
                Vertex { position: [self.front_right.x, self.front_right.y, base_height] },
                Vertex { position: [mid_right.x, mid_right.y, base_height + roof_height] },
            ];

        let roof_indices = vec![0, 1, 2, 2, 5, 0, 2, 3, 4, 4, 5, 2];

        let wall_indices = vec![1, 2, 3, 4, 5, 0];

        (
            Geometry::new(vertices.clone(), roof_indices),
            Geometry::new(vertices, wall_indices),
        )
    }

    fn scale(&self, factor: f32) -> Footprint {
        let center = P2::from_coordinates(
            (self.back_left.coords + self.back_right.coords + self.front_left.coords +
                 self.front_right.coords) / 4.0,
        );

        Footprint {
            back_left: center + factor * (self.back_left - center),
            back_right: center + factor * (self.back_right - center),
            front_left: center + factor * (self.front_left - center),
            front_right: center + factor * (self.front_right - center),
        }
    }
}

pub fn generate_house_footprint<R: Rng>(lot: &Lot, rng: &mut R) -> (Footprint, Footprint) {
    let building_position = lot.center_point;
    let building_orientation = (lot.connection_point - lot.center_point).normalize();
    let building_orientation_orth = building_orientation.orthogonal();

    let footprint_width = 10.0 + rng.next_f32() * 7.0;
    let footprint_depth = 7.0 + rng.next_f32() * 5.0;

    let entrance_position = building_position +
        building_orientation * (0.5 - 1.0 * rng.next_f32()) * footprint_width -
        building_orientation_orth * (rng.next_f32() * 0.3 + 0.1) * footprint_depth;
    let entrance_width = 5.0 + rng.next_f32() * 4.0;
    let entrance_depth = 3.0 + rng.next_f32() * 3.0;

    (
        Footprint {
            back_right: building_position + building_orientation * footprint_width / 2.0 -
                building_orientation_orth * footprint_depth / 2.0,
            back_left: building_position - building_orientation * footprint_width / 2.0 -
                building_orientation_orth * footprint_depth / 2.0,
            front_left: building_position - building_orientation * footprint_width / 2.0 +
                building_orientation_orth * footprint_depth / 2.0,
            front_right: building_position + building_orientation * footprint_width / 2.0 +
                building_orientation_orth * footprint_depth / 2.0,
        },
        Footprint {
            back_right: entrance_position + building_orientation_orth * entrance_width / 2.0 +
                building_orientation * entrance_depth / 2.0,
            back_left: entrance_position - building_orientation_orth * entrance_width / 2.0 +
                building_orientation * entrance_depth / 2.0,
            front_left: entrance_position - building_orientation_orth * entrance_width / 2.0 -
                building_orientation * entrance_depth / 2.0,
            front_right: entrance_position + building_orientation_orth * entrance_width / 2.0 -
                building_orientation * entrance_depth / 2.0,
        },
    )
}
