use kay::{World, TypedID};
use descartes::{N, P2, V2, Intersect, WithUniqueOrthogonal, LinePath, ArcLinePath, ClosedLinePath, PrimitiveArea, Area};
use util::random::{Rng, seed};
use michelangelo::{Vertex, Mesh, Instance, Surface, FlatSurface, Sculpture, SculptLine, SpannedSurface, SkeletonSpine};
use std::collections::HashMap;
use std::rc::Rc;

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

use rand::distributions::uniform::SampleUniform;

#[derive(Clone, Serialize, Deserialize)]
enum Variable<T: Clone + SampleUniform + PartialOrd> {
    Random{min: T, max: T, ident: String},
    Constant(T),
}

impl<T: Copy + Clone + SampleUniform + PartialOrd> Variable<T> {
    fn new_random(min: T, max: T, ident: &str) -> Variable<T> {
        Variable::Random{min, max, ident: ident.to_owned()}
    }

    fn evaluate(&self, lot: &Lot) -> T {
        match *self {
            Variable::Random{min, max, ref ident} => {
                seed((lot.original_lot_id, ident)).gen_range(min, max)
            },
            Variable::Constant(c) => c.clone()
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
enum Choice<T: Clone> {
    Random{options: Vec<T>, ident: String},
    Specific(T),
}

impl<T: Clone> Choice<T> {
    fn evaluate(&self, lot: &Lot) -> T {
        match *self {
            Choice::Random{ref options, ref ident} => {
                seed((lot.original_lot_id, ident)).choose(options).expect("Should have at least one choice").clone()
            },
            Choice::Specific(ref specific) => specific.clone()
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct BuildingRule {
    corpi: Vec<CorpusRule>,
    lot: LotRule
}

impl BuildingRule {
    fn collect_geometry(&self, collector: &mut BuildingGeometryCollector, lot: &Lot) -> Result<(), String> {
        let mut corpus_spines = Vec::new();
        for corpus in &self.corpi {
            corpus_spines.push(corpus.collect_geometry(collector, lot)?)
        }
        self.lot.collect_geometry(&corpus_spines, collector, lot)?;
        Ok(())
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
enum CorpusSide {
    Front,
    Back,
    Left,
    Right
}

#[derive(Clone, Serialize, Deserialize)]
struct CorpusRule {
    fundament: FundamentRule,
    n_floors: Variable<u8>,
    floor_rules: Vec<Choice<FloorRule>>,
    roof: RoofRule
}

impl CorpusRule {
    fn collect_geometry(&self, collector: &mut BuildingGeometryCollector, lot: &Lot) -> Result<SkeletonSpine, String> {
        let fundament_spine = self.fundament.evaluate(lot)?;
        let n_floors = self.n_floors.evaluate(lot);
        let mut current_spine = fundament_spine.clone();

        for f in 0..(n_floors as usize) {
            let rule_to_use = if f == 0 {
                &self.floor_rules[0]
            } else if f == self.floor_rules.len() - 1 {
                &self.floor_rules[self.floor_rules.len() - 1]
            } else {
                let ratio = n_floors as f32 / f as f32;
                let n_middle_floor_rules = self.floor_rules.len() - 2;
                let idx = (self.floor_rules.len() - 1).min(1 + (ratio * (n_middle_floor_rules as f32)) as usize);
                &self.floor_rules[idx]
            };

            current_spine = rule_to_use.evaluate(lot).collect_geometry(current_spine, collector, lot)?;
        }

        self.roof.collect_geometry(current_spine, collector, lot);

        Ok(fundament_spine)
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct LotRule {
    boundary_rule: Option<LotBoundaryRule>,
    ground_rule: Option<LotGroundRule>,
    paving_rules: Vec<PavingRule>
}

impl LotRule {
    fn collect_geometry(&self, corpus_spines: &[SkeletonSpine], collector: &mut BuildingGeometryCollector, lot: &Lot) -> Result<(), String> {
        if let Some(ref boundary_rule) = self.boundary_rule {
            boundary_rule.collect_geometry(collector, lot)?;
        }
        if let Some(ref ground_rule) = self.ground_rule {
            ground_rule.collect_geometry(collector, lot)?;
        }
        for paving_rule in &self.paving_rules {
            paving_rule.collect_geometry(corpus_spines, collector, lot)?;
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct PavingRule {
    paving_material: Choice<BuildingMaterial>,
    start_point_offset_ratio: Variable<N>,
    end_point_corpus: Variable<u8>,
    end_point_corpus_side: Choice<CorpusSide>,
    end_point_offset_ratio: Variable<N>,
    width: Variable<N>
}

impl PavingRule {
    fn collect_geometry(&self, corpus_spines: &[SkeletonSpine], collector: &mut BuildingGeometryCollector, lot: &Lot) -> Result<(), String> {
        let road_boundary = lot.longest_road_boundary();
        let start_point_along = road_boundary.length() * self.start_point_offset_ratio.evaluate(lot);
        let start_point = road_boundary.along(start_point_along);
        let start_direction = road_boundary.direction_along(start_point_along).orthogonal_right();
        let corpus_spine = corpus_spines.get(self.end_point_corpus.evaluate(lot) as usize).ok_or("Doesn't have corpus of this index")?;
        let corpus_side = match self.end_point_corpus_side.evaluate(lot) {
            CorpusSide::Front => corpus_spine.front.clone(),
            CorpusSide::Back => corpus_spine.back.clone(),
            CorpusSide::Left => corpus_spine.left.clone(),
            CorpusSide::Right => corpus_spine.right.clone(),
        };
        let end_point_along = corpus_side.path.length() * self.end_point_offset_ratio.evaluate(lot);
        let end_point = corpus_side.path.along(end_point_along);
        let end_direction = corpus_side.path.direction_along(end_point_along).orthogonal_right();
        let pavement_path = ArcLinePath::biarc(start_point, start_direction, end_point, end_direction).ok_or("Couldn't build pavement biarc")?.to_line_path_with_max_angle(0.1);
        let width = self.width.evaluate(lot);
        collector.collect_surface(self.paving_material.evaluate(lot), FlatSurface::from_band(pavement_path, width/2.0, width/2.0, 0.0));
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct LotGroundRule {
    shrink: Variable<N>,
    ground_material: Choice<BuildingMaterial>
}

impl LotGroundRule {
    fn collect_geometry(&self, collector: &mut BuildingGeometryCollector, lot: &Lot) -> Result<(), String> {
        let lot_surface = FlatSurface::from_primitive_area(lot.area.primitives[0].clone(), 0.0);
        let (_, shrunk_lot_surface) = lot_surface.extrude(0.0, self.shrink.evaluate(lot)).ok_or("Couldn't shrink lot surface")?;
        collector.collect_surface(self.ground_material.evaluate(lot), shrunk_lot_surface);
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct LotBoundaryRule {
    fence_height: Variable<N>,
    fence_material: Choice<BuildingMaterial>,
    fence_gap_offset_ratio: Variable<N>,
    fence_gap_width_ratio: Variable<N>
}

impl LotBoundaryRule {
    fn collect_geometry(&self, collector: &mut BuildingGeometryCollector, lot: &Lot) -> Result<(), String> {
        let road_boundary = lot.longest_road_boundary();
        let start_point_along_road_boundary = road_boundary.length() * self.fence_gap_offset_ratio.evaluate(lot);
        let start_point = road_boundary.along(start_point_along_road_boundary);
        let lot_boundary = lot.area.primitives[0].boundary.path();
        let (start_point_along_lot_boundary, _) = lot_boundary.project(start_point).ok_or("Can't reproject gap onto lot boundary")?;
        let gap_width = road_boundary.length() * self.fence_gap_width_ratio.evaluate(lot);
        let fence_path = lot_boundary.subsection(
            start_point_along_lot_boundary + gap_width / 2.0,
            start_point_along_lot_boundary - gap_width / 2.0).ok_or("Couldnt cut gap in lot boundary")?;
        let (fence_surface, _) = SculptLine::extrude(&Rc::new(SculptLine::new(fence_path, 0.0)), self.fence_height.evaluate(lot), 0.0).ok_or("Couldn't extrude fence")?;
        collector.collect_surface(self.fence_material.evaluate(lot), fence_surface);
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct FundamentRule {
    major_axis_angle_rel_to_road: Variable<N>,
    offset_on_minor_axis: Variable<N>,
    width: Variable<N>,
    max_length: Variable<N>,
    padding: Variable<N>,
}

const MAJOR_AXIS_RAY_HALF_LENGTH: f32 = 1000.0;

impl FundamentRule {
    fn evaluate(&self, lot: &Lot) -> Result<SkeletonSpine, String> {
        let road_direction = lot.best_road_connection().1.orthogonal_left();
        let major_axis_direction = road_direction; // TODO: rotate according to major_axis_angle_rel_to_road
        let minor_axis_direction = major_axis_direction.orthogonal_right();
        let spine_center_point = lot.center_point() + self.offset_on_minor_axis.evaluate(lot) * minor_axis_direction;

        let padding = self.padding.evaluate(lot);

        let major_axis_line_path = LinePath::new(vec![
            spine_center_point + MAJOR_AXIS_RAY_HALF_LENGTH * major_axis_direction,
            spine_center_point - MAJOR_AXIS_RAY_HALF_LENGTH * major_axis_direction
        ].into()).ok_or("Should be able to construct major axis line path")?;
        let intersections = (&major_axis_line_path, lot.area.primitives[0].boundary.path()).intersect();

        let intersection_before = intersections.iter().find(|i| i.along_a < MAJOR_AXIS_RAY_HALF_LENGTH).ok_or("Couldn't find suitable back lot intersection")?;
        let intersection_after = intersections.iter().find(|i| i.along_a > MAJOR_AXIS_RAY_HALF_LENGTH).ok_or("Couldn't find suitable front lot intersection")?;

        if intersection_after.along_a - intersection_before.along_a < 2.0 * padding {
            return Err("Lot intersections too close to allow for padding".to_owned());
        }

        let available_length = intersection_after.along_a - intersection_before.along_a;
        let max_length = self.max_length.evaluate(lot);
        let effective_padding = ((available_length - max_length) / 2.0).max(padding);

        let skeleton_path = major_axis_line_path.subsection(
            intersection_before.along_a + effective_padding,
            intersection_after.along_a - effective_padding
        ).ok_or("Couldn't construct fundament skeleton spine path")?;

        let width = self.width.evaluate(lot);

        SkeletonSpine::new(Rc::new(SculptLine::new(skeleton_path, 0.0)), width).ok_or("Couldn't construct fundament skeleton spine".to_owned())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct FloorRule {
    height: Variable<N>,
    widen_by_next: Variable<N>,
    extend_by_next: Variable<N>,
    front: FacadeRule,
    back: FacadeRule,
    left: FacadeRule,
    right: FacadeRule
}

impl FloorRule {
    fn collect_geometry(self, base_spine: SkeletonSpine, collector: &mut BuildingGeometryCollector, lot: &Lot) -> Result<SkeletonSpine, String> {
        let (_, upper_spine) = base_spine.extrude(self.height.evaluate(lot), 0.0, 0.0).ok_or("Couldn't extrude floor upward.")?;

        self.front.collect_geometry(base_spine.front, upper_spine.front.clone(), collector, lot)?;
        self.back.collect_geometry(base_spine.back, upper_spine.back.clone(), collector, lot)?;
        self.left.collect_geometry(base_spine.left, upper_spine.left.clone(), collector, lot)?;
        self.right.collect_geometry(base_spine.right, upper_spine.right.clone(), collector, lot)?;

        let (_, next_spine) = upper_spine.extrude(0.0, self.widen_by_next.evaluate(lot), self.extend_by_next.evaluate(lot)).ok_or("Couldn't extrude floor outward.")?;
        Ok(next_spine)
    }
}

#[derive(Clone, Serialize, Deserialize)]
enum FacadeRule {
    Face {
        wall_material: Choice<BuildingMaterial>,
        decorations: Vec<Choice<FacadeDecorationRule>>
    },
    Subdivision {
        rules_with_weights: Vec<(FacadeRule, Variable<N>)>
    }
}

impl FacadeRule {
    fn collect_geometry(&self, base_line: Rc<SculptLine>, upper_line: Rc<SculptLine>, collector: &mut BuildingGeometryCollector, lot: &Lot) -> Result<(), String> {
        match *self {
            FacadeRule::Face{ref wall_material, ref decorations} => {
                collector.collect_surface(wall_material.evaluate(lot), SpannedSurface::new(base_line.clone(), upper_line));
                for decoration_choice in decorations {
                    decoration_choice.evaluate(lot).collect_geometry(base_line.clone(), collector, lot)?;
                };
                Ok(())
            },
            FacadeRule::Subdivision{ref rules_with_weights} => {
                let weights = rules_with_weights.iter().map(|rw| rw.1.evaluate(lot)).collect::<Vec<_>>();
                let subdivided_lines = base_line.subdivide(&weights);
                let subdivided_upper_lines = upper_line.subdivide(&weights);
                for ((line_seg, upper_line_seg), (rule, _)) in subdivided_lines.iter().zip(subdivided_upper_lines.iter()).zip(rules_with_weights) {
                    rule.collect_geometry(line_seg.clone(), upper_line_seg.clone(), collector, lot)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct FacadeDecorationRule {
    prop: BuildingProp,
    color: [Variable<N>; 3],
    spacing: Variable<N>,
}

impl FacadeDecorationRule {
    fn collect_geometry(&self, base_line: Rc<SculptLine>, collector: &mut BuildingGeometryCollector, lot: &Lot) -> Result<(), String> {
        let n_spacings = (base_line.path.length() / self.spacing.evaluate(lot)).floor() as usize + 1;
        let effective_spacing = base_line.path.length() / (n_spacings as f32);
        let color = [self.color[0].evaluate(lot), self.color[1].evaluate(lot), self.color[2].evaluate(lot)];
        let instances = (1..n_spacings).map(|i| {
            let along = i as f32 * effective_spacing;
            let pos = base_line.path.along(along);
            let direction = base_line.path.direction_along(along);
            Instance {
                instance_position: [pos.x, pos.y, base_line.z],
                instance_color: color,
                instance_direction: [direction.x, direction.y]
            }
        }).collect();
        collector.collect_props(self.prop, instances);
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct RoofRule {
    height: Variable<N>,
    gable_depth_front: Variable<N>,
    gable_depth_back: Variable<N>,
    roof_material: Choice<BuildingMaterial>,
    gable_material: Choice<BuildingMaterial>
}

impl RoofRule {
    fn collect_geometry(&self, base_spine: SkeletonSpine, collector: &mut BuildingGeometryCollector, lot: &Lot) {
        let (roof_surface, gable_surface) = base_spine.roof(self.height.evaluate(lot), self.gable_depth_front.evaluate(lot), self.gable_depth_back.evaluate(lot));
        collector.collect_surface(self.roof_material.evaluate(lot), roof_surface);
        collector.collect_surface(self.gable_material.evaluate(lot), gable_surface);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
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

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash,  Serialize, Deserialize)]
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
pub struct BuildingGeometry {
    pub meshes: HashMap<BuildingMaterial, Mesh>,
    pub props: HashMap<BuildingProp, Vec<Instance>>,
}

struct BuildingGeometryCollector {
    sculptures: HashMap<BuildingMaterial, Sculpture>,
    props: HashMap<BuildingProp, Vec<Instance>>,
}

impl BuildingGeometryCollector {
    fn new() -> Self {
        BuildingGeometryCollector {
            sculptures: HashMap::new(),
            props: HashMap::new()
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

    fn to_geometry(self) -> BuildingGeometry {
        BuildingGeometry {
            meshes: self.sculptures.into_iter().map(|(material, sculpture)| (material, sculpture.to_mesh())).collect(),
            props: self.props
        }
    }
}

fn family_house() -> BuildingRule {
    let windowed_facade_rule = FacadeRule::Face {
        wall_material: Choice::Specific(BuildingMaterial::WhiteWall),
        decorations: vec![
            Choice::Specific(FacadeDecorationRule {
                prop: BuildingProp::SmallWindow,
                color: [Variable::Constant(0.7), Variable::Constant(0.6), Variable::Constant(0.6)],
                spacing: Variable::new_random(3.0, 5.0, "window-spacing")
            })
        ]
    };

    BuildingRule {
        corpi: vec![
            CorpusRule {
                fundament: FundamentRule {
                    major_axis_angle_rel_to_road: Variable::Constant(0.0),
                    offset_on_minor_axis: Variable::Constant(0.0),
                    width: Variable::new_random(5.0, 12.0, "fundament-width"),
                    max_length: Variable::new_random(10.0, 18.0, "fundament-max-length"),
                    padding: Variable::Constant(5.0),
                },
                n_floors: Variable::new_random(1, 2, "n-floors"),
                floor_rules: vec![Choice::Specific(FloorRule {
                    height: Variable::new_random(2.1, 3.0, "floor-height"),
                    widen_by_next: Variable::Constant(0.0),
                    extend_by_next: Variable::Constant(0.0),
                    front: windowed_facade_rule.clone(),
                    back: windowed_facade_rule.clone(),
                    left: windowed_facade_rule.clone(),
                    right: windowed_facade_rule
                })],
                roof: RoofRule {
                    height: Variable::new_random(2.0, 5.0, "roof-height"),
                    gable_depth_front: Variable::new_random(0.0, 3.0, "gable-depth"),
                    gable_depth_back: Variable::new_random(0.0, 3.0, "gable-depth"),
                    roof_material: Choice::Specific(BuildingMaterial::TiledRoof),
                    gable_material: Choice::Specific(BuildingMaterial::TiledRoof)
                }
            }
        ],
        lot: LotRule {
            boundary_rule: Some(LotBoundaryRule {
                fence_height: Variable::new_random(0.5, 1.5, "fence-height"),
                fence_material: Choice::Specific(BuildingMaterial::WoodenFence),
                fence_gap_offset_ratio: Variable::new_random(0.1, 0.9, "fence-gap-point"),
                fence_gap_width_ratio: Variable::Constant(0.2)
            }),
            ground_rule: None,
            paving_rules: vec![
                PavingRule {
                    start_point_offset_ratio: Variable::new_random(0.1, 0.9, "fence-gap-point"),
                    end_point_corpus: Variable::Constant(0),
                    end_point_corpus_side: Choice::Specific(CorpusSide::Left),
                    end_point_offset_ratio: Variable::Constant(0.5),
                    width: Variable::new_random(1.0, 3.0, "pavement-width"),
                    paving_material: Choice::Specific(BuildingMaterial::LotAsphalt)
                }
            ]
        }
    }
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
) -> BuildingGeometry {
    // TODO keep original building if lot changes
    let mut rng = seed(lot.original_lot_id);

    let (base_width, base_depth) = footprint_dimensions(building_style);

    let (main_footprint, entrance_footprint) =
        generate_house_footprint(lot, base_width, base_depth, 0.0, &mut rng);

    match building_style {
        BuildingStyle::FamilyHouse => {
            let mut collector = BuildingGeometryCollector::new();
            family_house().collect_geometry(&mut collector, lot).expect("Expecting things to work in general");
            collector.to_geometry()

            // let height = 2.6 + 2.0 * rng.gen::<f32>();
            // let entrance_height = 2.0 + rng.gen::<f32>();

            // let (roof_brick_mesh, roof_wall_mesh) =
            //     main_footprint.open_gable_roof_mesh(height, 0.3);
            // let (entrance_roof_brick_mesh, entrance_roof_wall_mesh) =
            //     entrance_footprint.open_gable_roof_mesh(entrance_height, 0.3);

            // let (fence_surface, _) = FlatSurface::from_band(
            //     lot.area.primitives[0].boundary.path().clone(),
            //     0.1,
            //     0.1,
            //     0.0,
            // )
            // .extrude(1.0, 0.0)
            // .unwrap();

            // let fence_mesh = Sculpture::new(vec![fence_surface.into()]).to_mesh();

            // BuildingGeometry {
            //     meshes: vec![
            //         (
            //             BuildingMaterial::WhiteWall,
            //             main_footprint.wall_mesh(height)
            //                 + entrance_footprint.wall_mesh(entrance_height)
            //                 + roof_wall_mesh
            //                 + entrance_roof_wall_mesh,
            //         ),
            //         (
            //             BuildingMaterial::TiledRoof,
            //             roof_brick_mesh + entrance_roof_brick_mesh,
            //         ),
            //         (BuildingMaterial::WoodenFence, fence_mesh),
            //     ]
            //     .into_iter()
            //     .collect(),
            //     props: vec![
            //         (
            //             BuildingProp::SmallWindow,
            //             main_footprint
            //                 .distribute_along_walls(3.0)
            //                 .into_iter()
            //                 .map(|(position, direction)| Instance {
            //                     instance_position: [position.x, position.y, 0.0],
            //                     instance_direction: [direction.x, direction.y],
            //                     instance_color: [0.7, 0.6, 0.6],
            //                 })
            //                 .collect(),
            //         ),
            //         (
            //             BuildingProp::NarrowDoor,
            //             vec![{
            //                 let position = P2::from_coordinates(
            //                     (entrance_footprint.front_right.coords
            //                         + entrance_footprint.back_right.coords)
            //                         / 2.0,
            //                 );
            //                 let direction = (entrance_footprint.back_right
            //                     - entrance_footprint.front_right)
            //                     .normalize();
            //                 Instance {
            //                     instance_position: [position.x, position.y, 0.0],
            //                     instance_direction: [direction.x, direction.y],
            //                     instance_color: [0.6, 0.5, 0.5],
            //                 }
            //             }],
            //         ),
            //     ]
            //     .into_iter()
            //     .collect(),
            // }
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
