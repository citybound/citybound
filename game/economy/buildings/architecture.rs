use descartes::{N, P2, Norm, WithUniqueOrthogonal};
use rand::Rng;
use monet::{Vertex, Geometry};

use super::Lot;

#[derive(Compact, Clone)]
pub struct BuildingGeometry {
    pub wall: Geometry,
    pub brick_roof: Geometry,
    pub flat_roof: Geometry,
    pub field: Geometry,
}

#[derive(Copy, Clone)]
pub enum BuildingStyle {
    FamilyHouse,
    GroceryShop,
    CropFarm,
}

pub fn build_building<R: Rng>(
    lot: &Lot,
    building_type: BuildingStyle,
    rng: &mut R,
) -> BuildingGeometry {
    let (main_footprint, entrance_footprint) = generate_house_footprint(lot, rng);

    match building_type {
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
        BuildingStyle::CropFarm => {
            BuildingGeometry {
                wall: Geometry::empty(),
                brick_roof: Geometry::empty(),
                flat_roof: Geometry::empty(),
                field: main_footprint.flat_roof_geometry(0.0),
            }
        }
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
        let mid_right = (self.back_right + self.front_right.to_vector()) / 2.0;
        let mid_left = (self.back_left + self.front_left.to_vector()) / 2.0;

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
}

pub fn generate_house_footprint<R: Rng>(lot: &Lot, rng: &mut R) -> (Footprint, Footprint) {
    let orientation_orth = lot.orientation.orthogonal();

    let footprint_width = 10.0 + rng.next_f32() * 7.0;
    let footprint_depth = 7.0 + rng.next_f32() * 5.0;

    let entrance_position = lot.position +
        lot.orientation * (0.5 - 1.0 * rng.next_f32()) * footprint_width -
        orientation_orth * (rng.next_f32() * 0.3 + 0.1) * footprint_depth;
    let entrance_width = 5.0 + rng.next_f32() * 4.0;
    let entrance_depth = 3.0 + rng.next_f32() * 3.0;

    (
        Footprint {
            back_right: lot.position + lot.orientation * footprint_width / 2.0 -
                orientation_orth * footprint_depth / 2.0,
            back_left: lot.position - lot.orientation * footprint_width / 2.0 -
                orientation_orth * footprint_depth / 2.0,
            front_left: lot.position - lot.orientation * footprint_width / 2.0 +
                orientation_orth * footprint_depth / 2.0,
            front_right: lot.position + lot.orientation * footprint_width / 2.0 +
                orientation_orth * footprint_depth / 2.0,
        },
        Footprint {
            back_right: entrance_position + orientation_orth * entrance_width / 2.0 +
                lot.orientation * entrance_depth / 2.0,
            back_left: entrance_position - orientation_orth * entrance_width / 2.0 +
                lot.orientation * entrance_depth / 2.0,
            front_left: entrance_position - orientation_orth * entrance_width / 2.0 -
                lot.orientation * entrance_depth / 2.0,
            front_right: entrance_position + orientation_orth * entrance_width / 2.0 -
                lot.orientation * entrance_depth / 2.0,
        },
    )
}
