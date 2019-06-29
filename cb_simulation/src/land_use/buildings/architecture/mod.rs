use kay::{ActorSystem, World, TypedID};
use compact::{COption, CHashMap};
use descartes::{N, P2, V2, WithUniqueOrthogonal, LinePath, ClosedLinePath, PrimitiveArea, Area};
use cb_util::random::{Rng, seed};
use cb_util::config_manager::Name;
use michelangelo::{Vertex, Mesh, Instance, Surface, FlatSurface, Sculpture};
use std::collections::HashMap;

pub mod materials_and_props;
use self::materials_and_props::{BuildingMaterial, BuildingProp};

pub mod language;
use self::language::{Choice, Variable, ArchitectureRule, FacadeRule, FacadeDecorationRule,
CorpusRule, FundamentRule, FloorRule, RoofRule, PavingRule, LotRule, LotBoundaryRule, CorpusSide};

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

#[derive(Clone)]
pub struct BuildingGeometry {
    pub meshes: HashMap<BuildingMaterial, Mesh>,
    pub props: HashMap<BuildingProp, Vec<Instance>>,
}

pub struct BuildingGeometryCollector {
    sculptures: HashMap<BuildingMaterial, Sculpture>,
    props: HashMap<BuildingProp, Vec<Instance>>,
}

impl BuildingGeometryCollector {
    fn new() -> Self {
        BuildingGeometryCollector {
            sculptures: HashMap::new(),
            props: HashMap::new(),
        }
    }

    fn collect_surface<S: Into<Surface>>(&mut self, material: BuildingMaterial, surface: S) {
        self.sculptures
            .entry(material)
            .or_insert_with(|| Sculpture::new(vec![]))
            .push(surface.into());
    }

    fn collect_props(&mut self, prop: BuildingProp, instances: Vec<Instance>) {
        self.props
            .entry(prop)
            .or_insert_with(Vec::new)
            .extend(instances);
    }

    fn into_geometry(self) -> BuildingGeometry {
        BuildingGeometry {
            meshes: self
                .sculptures
                .into_iter()
                .map(|(material, sculpture)| (material, sculpture.to_mesh()))
                .collect(),
            props: self.props,
        }
    }
}

fn default_rules() -> CHashMap<Name, ArchitectureRule> {
    vec![(Name::from("family_house").unwrap(), {
        let windowed_facade_rule = FacadeRule::Face(
            Choice::Specific(BuildingMaterial::WhiteWall),
            vec![Choice::Specific(FacadeDecorationRule {
                prop: BuildingProp::SmallWindow,
                color: [
                    Variable::Constant(0.7),
                    Variable::Constant(0.6),
                    Variable::Constant(0.6),
                ],
                spacing: Variable::new_random(3.0, 5.0, "window-spacing"),
            })]
            .into(),
        );

        ArchitectureRule {
            corpi: vec![CorpusRule {
                fundament: FundamentRule {
                    major_axis_angle_rel_to_road: Variable::Constant(0.0),
                    offset_on_minor_axis: Variable::Constant(0.0),
                    width: Variable::new_random(5.0, 12.0, "fundWidth"),
                    max_length: Variable::new_random(10.0, 18.0, "fundMaxLength"),
                    padding: Variable::Constant(5.0),
                },
                n_floors: Variable::new_random(1, 2, "nFloors"),
                floor_rules: vec![Choice::Specific(FloorRule {
                    height: Variable::new_random(2.1, 3.0, "floorHeight"),
                    widen_by_next: Variable::Constant(0.0),
                    extend_by_next: Variable::Constant(0.0),
                    front: windowed_facade_rule.clone(),
                    back: windowed_facade_rule.clone(),
                    left: windowed_facade_rule.clone(),
                    right: windowed_facade_rule,
                })]
                .into(),
                roof: RoofRule {
                    height: Variable::new_random(2.0, 5.0, "roofHeight"),
                    gable_depth_front: Variable::new_random(0.0, 3.0, "gableDepth"),
                    gable_depth_back: Variable::new_random(0.0, 3.0, "gableDepth"),
                    roof_material: Choice::Specific(BuildingMaterial::TiledRoof),
                    gable_material: Choice::Specific(BuildingMaterial::TiledRoof),
                },
            }]
            .into(),
            lot: LotRule {
                boundary_rule: COption(Some(LotBoundaryRule {
                    fence_height: Variable::new_random(0.5, 1.5, "fenceHeight"),
                    fence_material: Choice::Specific(BuildingMaterial::WoodenFence),
                    fence_gap_offset_ratio: Variable::new_random(0.1, 0.9, "fenceGapPoint"),
                    fence_gap_width_ratio: Variable::Constant(0.2),
                })),
                ground_rule: COption(None),
                paving_rules: vec![PavingRule {
                    start_point_offset_ratio: Variable::new_random(0.1, 0.9, "fenceGapPoint"),
                    end_point_corpus: Variable::Constant(0),
                    end_point_corpus_side: Choice::Specific(CorpusSide::Left),
                    end_point_offset_ratio: Variable::Constant(0.5),
                    width: Variable::new_random(1.0, 3.0, "pavementWidth"),
                    paving_material: Choice::Specific(BuildingMaterial::LotAsphalt),
                }]
                .into(),
            },
        }
    })]
    .into_iter()
    .collect()
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
    architecture_rules: &CHashMap<Name, ArchitectureRule>,
    household_ids: &[::economy::households::HouseholdID],
    world: &mut World,
) -> BuildingGeometry {
    // TODO keep original building if lot changes
    let mut rng = seed(lot.original_lot_id);

    let (base_width, base_depth) = footprint_dimensions(building_style);

    let (main_footprint, entrance_footprint) =
        generate_house_footprint(lot, base_width, base_depth, 0.0, &mut rng);

    match building_style {
        BuildingStyle::FamilyHouse => {
            let mut collector = BuildingGeometryCollector::new();
            architecture_rules
                .get(Name::from("family_house").unwrap())
                .expect("Expected family_house rule to exist")
                .collect_geometry(&mut collector, lot)
                .expect("Expecting things to work in general");
            collector.into_geometry()
        }
        BuildingStyle::GroceryShop => {
            let height = 3.0 + rng.gen::<f32>();
            let entrance_height = height - 0.7;
            let business_color = [
                rng.gen_range(0.3, 0.6),
                rng.gen_range(0.3, 0.6),
                rng.gen_range(0.3, 0.6),
            ];

            BuildingGeometry {
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
            let (_, shrunk_lot_surface) = lot_surface.extrude(0.0, 2.0).unwrap();

            BuildingGeometry {
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

            BuildingGeometry {
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

            BuildingGeometry {
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
        BuildingStyle::NeighboringTownConnection => BuildingGeometry {
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

        let wall_surface = footprint_surface.extrude(wall_height, 0.0).unwrap().0;

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

pub fn setup(system: &mut ActorSystem) {
    system.register::<cb_util::config_manager::ConfigManager<ArchitectureRule>>();
    cb_util::config_manager::auto_setup::<ArchitectureRule>(system);
}

pub fn spawn(world: &mut World) {
    cb_util::config_manager::ConfigManagerID::<ArchitectureRule>::spawn(default_rules(), world);
}