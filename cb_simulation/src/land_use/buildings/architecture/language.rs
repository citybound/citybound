use compact::{CVec, COption, Compact, CHashMap};
use cb_util::config_manager::{Config, Name};
use arrayvec::ArrayString;
use rand::distributions::uniform::SampleUniform;
use land_use::zone_planning::Lot;
use cb_util::random::{Rng, seed};
use michelangelo::{Instance, FlatSurface, SculptLine, SpannedSurface, SkeletonSpine};
use descartes::{N, Intersect, WithUniqueOrthogonal, LinePath, ArcLinePath};
use super::materials_and_props::{BuildingMaterial, BuildingProp};
use super::BuildingGeometryCollector;
use std::rc::Rc;

#[derive(Clone, Serialize, Deserialize, Copy)]
pub enum Variable<T: Clone + Compact + SampleUniform + PartialOrd> {
    Random(T, T, ArrayString<[u8; 16]>),
    Constant(T),
}

impl<T: Copy + Clone + SampleUniform + PartialOrd> Variable<T> {
    pub fn new_random(min: T, max: T, ident: &str) -> Variable<T> {
        Variable::Random(
            min,
            max,
            ArrayString::from(ident).expect("Random ident too long"),
        )
    }

    fn evaluate(&self, lot: &Lot) -> T {
        match *self {
            Variable::Random(min, max, ref ident) => {
                seed((lot.original_lot_id, ident)).gen_range(min, max)
            }
            Variable::Constant(c) => c,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Compact)]
pub enum Choice<T: Clone + Compact> {
    Random(CVec<T>, ArrayString<[u8; 16]>),
    Specific(T),
}

impl<T: Clone + Compact> Choice<T> {
    pub fn new_random(options: Vec<T>, ident: &str) -> Self {
        Choice::Random(
            options.into(),
            ArrayString::from(ident).expect("Random ident too long"),
        )
    }

    fn evaluate(&self, lot: &Lot) -> T {
        match *self {
            Choice::Random(ref options, ref ident) => seed((lot.original_lot_id, ident))
                .choose(options)
                .expect("Should have at least one choice")
                .clone(),
            Choice::Specific(ref specific) => specific.clone(),
        }
    }
}

#[derive(Compact, Clone)]
pub enum ArchitectureRule {
    Building(BuildingRule),
    Corpus(CorpusRule),
    Lot(LotRule),
    Fundament(FundamentRule),
    Floor(FloorRule),
    Facade(FacadeRule),
    FacadeDecoration(FacadeDecorationRule),
    Roof(RoofRule),
    Paving(PavingRule),
    LotBoundary(LotBoundaryRule),
    LotGround(LotGroundRule),
}

impl Config for ArchitectureRule {}

#[derive(Serialize, Deserialize)]
pub struct RuleRef<V> {
    rule: Name,
    _marker: ::std::marker::PhantomData<V>,
}

impl<V> Copy for RuleRef<V> {}

impl<V> Clone for RuleRef<V> {
    fn clone(&self) -> RuleRef<V> {
        *self
    }
}

impl<V> RuleRef<V> {
    pub fn of(name: &str) -> RuleRef<V> {
        RuleRef {
            rule: Name::from(name).unwrap(),
            _marker: ::std::marker::PhantomData,
        }
    }
}

impl RuleRef<BuildingRule> {
    pub fn resolve<'a>(
        &self,
        all_rules: &'a CHashMap<Name, ArchitectureRule>,
    ) -> Result<&'a BuildingRule, String> {
        if let Some(ArchitectureRule::Building(ref r)) = all_rules.get(self.rule) {
            Ok(r)
        } else {
            Err(format!("Couldn't find BuildingRule {}", self.rule))
        }
    }
}

impl RuleRef<CorpusRule> {
    pub fn resolve<'a>(
        &self,
        all_rules: &'a CHashMap<Name, ArchitectureRule>,
    ) -> Result<&'a CorpusRule, String> {
        if let Some(ArchitectureRule::Corpus(ref r)) = all_rules.get(self.rule) {
            Ok(r)
        } else {
            Err(format!("Couldn't find CorpusRule {}", self.rule))
        }
    }
}

impl RuleRef<LotRule> {
    pub fn resolve<'a>(
        &self,
        all_rules: &'a CHashMap<Name, ArchitectureRule>,
    ) -> Result<&'a LotRule, String> {
        if let Some(ArchitectureRule::Lot(ref r)) = all_rules.get(self.rule) {
            Ok(r)
        } else {
            Err(format!("Couldn't find LotRule {}", self.rule))
        }
    }
}

impl RuleRef<FundamentRule> {
    pub fn resolve<'a>(
        &self,
        all_rules: &'a CHashMap<Name, ArchitectureRule>,
    ) -> Result<&'a FundamentRule, String> {
        if let Some(ArchitectureRule::Fundament(ref r)) = all_rules.get(self.rule) {
            Ok(r)
        } else {
            Err(format!("Couldn't find FundamentRule {}", self.rule))
        }
    }
}

impl RuleRef<FloorRule> {
    pub fn resolve<'a>(
        &self,
        all_rules: &'a CHashMap<Name, ArchitectureRule>,
    ) -> Result<&'a FloorRule, String> {
        if let Some(ArchitectureRule::Floor(ref r)) = all_rules.get(self.rule) {
            Ok(r)
        } else {
            Err(format!("Couldn't find FloorRule {}", self.rule))
        }
    }
}

impl RuleRef<FacadeRule> {
    pub fn resolve<'a>(
        &self,
        all_rules: &'a CHashMap<Name, ArchitectureRule>,
    ) -> Result<&'a FacadeRule, String> {
        if let Some(ArchitectureRule::Facade(ref r)) = all_rules.get(self.rule) {
            Ok(r)
        } else {
            Err(format!("Couldn't find FacadeRule {}", self.rule))
        }
    }
}

impl RuleRef<FacadeDecorationRule> {
    pub fn resolve<'a>(
        &self,
        all_rules: &'a CHashMap<Name, ArchitectureRule>,
    ) -> Result<&'a FacadeDecorationRule, String> {
        if let Some(ArchitectureRule::FacadeDecoration(ref r)) = all_rules.get(self.rule) {
            Ok(r)
        } else {
            Err(format!("Couldn't find FacadeDecorationRule {}", self.rule))
        }
    }
}

impl RuleRef<RoofRule> {
    pub fn resolve<'a>(
        &self,
        all_rules: &'a CHashMap<Name, ArchitectureRule>,
    ) -> Result<&'a RoofRule, String> {
        if let Some(ArchitectureRule::Roof(ref r)) = all_rules.get(self.rule) {
            Ok(r)
        } else {
            Err(format!("Couldn't find RoofRule {}", self.rule))
        }
    }
}

impl RuleRef<PavingRule> {
    pub fn resolve<'a>(
        &self,
        all_rules: &'a CHashMap<Name, ArchitectureRule>,
    ) -> Result<&'a PavingRule, String> {
        if let Some(ArchitectureRule::Paving(ref r)) = all_rules.get(self.rule) {
            Ok(r)
        } else {
            Err(format!("Couldn't find PavingRule {}", self.rule))
        }
    }
}

impl RuleRef<LotBoundaryRule> {
    pub fn resolve<'a>(
        &self,
        all_rules: &'a CHashMap<Name, ArchitectureRule>,
    ) -> Result<&'a LotBoundaryRule, String> {
        if let Some(ArchitectureRule::LotBoundary(ref r)) = all_rules.get(self.rule) {
            Ok(r)
        } else {
            Err(format!("Couldn't find LotBoundaryRule {}", self.rule))
        }
    }
}

impl RuleRef<LotGroundRule> {
    pub fn resolve<'a>(
        &self,
        all_rules: &'a CHashMap<Name, ArchitectureRule>,
    ) -> Result<&'a LotGroundRule, String> {
        if let Some(ArchitectureRule::LotGround(ref r)) = all_rules.get(self.rule) {
            Ok(r)
        } else {
            Err(format!("Couldn't find LotGroundRule {}", self.rule))
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Compact)]
pub struct BuildingRule {
    pub corpi: CVec<RuleRef<CorpusRule>>,
    pub lot: RuleRef<LotRule>,
}

impl BuildingRule {
    pub fn collect_geometry(
        &self,
        collector: &mut BuildingGeometryCollector,
        lot: &Lot,
        all_rules: &CHashMap<Name, ArchitectureRule>,
    ) -> Result<(), String> {
        let mut corpus_spines = Vec::new();
        for corpus in &self.corpi {
            corpus_spines.push(
                corpus
                    .resolve(all_rules)?
                    .collect_geometry(collector, lot, all_rules)?,
            )
        }
        self.lot
            .resolve(all_rules)?
            .collect_geometry(&corpus_spines, collector, lot, all_rules)?;
        Ok(())
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum CorpusSide {
    Front,
    Back,
    Left,
    Right,
}

#[derive(Clone, Serialize, Deserialize, Compact)]
pub struct CorpusRule {
    pub fundament: RuleRef<FundamentRule>,
    pub n_floors: Variable<u8>,
    pub floor_rules: CVec<Choice<RuleRef<FloorRule>>>,
    pub roof: RuleRef<RoofRule>,
}

impl CorpusRule {
    fn collect_geometry(
        &self,
        collector: &mut BuildingGeometryCollector,
        lot: &Lot,
        all_rules: &CHashMap<Name, ArchitectureRule>,
    ) -> Result<SkeletonSpine, String> {
        let fundament_spine = self.fundament.resolve(all_rules)?.evaluate(lot)?;
        let n_floors = self.n_floors.evaluate(lot);
        let mut current_spine = fundament_spine.clone();

        for f in 0..(n_floors as usize) {
            let rule_choice_to_use = if f == 0 {
                &self.floor_rules[0]
            } else if f == self.floor_rules.len() - 1 {
                &self.floor_rules[self.floor_rules.len() - 1]
            } else {
                let ratio = f32::from(n_floors) / f as f32;
                let n_middle_floor_rules = self.floor_rules.len() - 2;
                let idx = (self.floor_rules.len() - 1)
                    .min(1 + (ratio * (n_middle_floor_rules as f32)) as usize);
                &self.floor_rules[idx]
            };

            current_spine = rule_choice_to_use
                .evaluate(lot)
                .resolve(all_rules)?
                .collect_geometry(current_spine, collector, lot, all_rules)?;
        }

        self.roof
            .resolve(all_rules)?
            .collect_geometry(current_spine, collector, lot);

        Ok(fundament_spine)
    }
}

#[derive(Clone, Serialize, Deserialize, Compact)]
pub struct LotRule {
    pub boundary_rule: COption<RuleRef<LotBoundaryRule>>,
    pub ground_rule: COption<RuleRef<LotGroundRule>>,
    pub paving_rules: CVec<RuleRef<PavingRule>>,
}

impl LotRule {
    fn collect_geometry(
        &self,
        corpus_spines: &[SkeletonSpine],
        collector: &mut BuildingGeometryCollector,
        lot: &Lot,
        all_rules: &CHashMap<Name, ArchitectureRule>,
    ) -> Result<(), String> {
        if let COption(Some(ref boundary_rule)) = self.boundary_rule {
            boundary_rule
                .resolve(all_rules)?
                .collect_geometry(collector, lot)?;
        }
        if let COption(Some(ref ground_rule)) = self.ground_rule {
            ground_rule
                .resolve(all_rules)?
                .collect_geometry(collector, lot)?;
        }
        for paving_rule in &self.paving_rules {
            paving_rule
                .resolve(all_rules)?
                .collect_geometry(corpus_spines, collector, lot)?;
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Compact)]
pub struct PavingRule {
    pub paving_material: Choice<BuildingMaterial>,
    pub start_point_offset_ratio: Variable<N>,
    pub end_point_corpus: Variable<u8>,
    pub end_point_corpus_side: Choice<CorpusSide>,
    pub end_point_offset_ratio: Variable<N>,
    pub width: Variable<N>,
}

impl PavingRule {
    fn collect_geometry(
        &self,
        corpus_spines: &[SkeletonSpine],
        collector: &mut BuildingGeometryCollector,
        lot: &Lot,
    ) -> Result<(), String> {
        let road_boundary = lot.longest_road_boundary();
        let start_point_along =
            road_boundary.length() * self.start_point_offset_ratio.evaluate(lot);
        let start_point = road_boundary.along(start_point_along);
        let start_direction = road_boundary
            .direction_along(start_point_along)
            .orthogonal_right();
        let corpus_spine = corpus_spines
            .get(self.end_point_corpus.evaluate(lot) as usize)
            .ok_or("Doesn't have corpus of this index")?;
        let corpus_side = match self.end_point_corpus_side.evaluate(lot) {
            CorpusSide::Front => corpus_spine.front.clone(),
            CorpusSide::Back => corpus_spine.back.clone(),
            CorpusSide::Left => corpus_spine.left.clone(),
            CorpusSide::Right => corpus_spine.right.clone(),
        };
        let end_point_along = corpus_side.path.length() * self.end_point_offset_ratio.evaluate(lot);
        let end_point = corpus_side.path.along(end_point_along);
        let end_direction = corpus_side
            .path
            .direction_along(end_point_along)
            .orthogonal_right();
        let pavement_path =
            ArcLinePath::biarc(start_point, start_direction, end_point, end_direction)
                .ok_or("Couldn't build pavement biarc")?
                .to_line_path_with_max_angle(0.1);
        let width = self.width.evaluate(lot);
        collector.collect_surface(
            self.paving_material.evaluate(lot),
            FlatSurface::from_band(pavement_path, width / 2.0, width / 2.0, 0.0),
        );
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Compact)]
pub struct LotGroundRule {
    pub shrink: Variable<N>,
    pub ground_material: Choice<BuildingMaterial>,
}

impl LotGroundRule {
    fn collect_geometry(
        &self,
        collector: &mut BuildingGeometryCollector,
        lot: &Lot,
    ) -> Result<(), String> {
        let lot_surface = FlatSurface::from_primitive_area(lot.area.primitives[0].clone(), 0.0);
        let (_, shrunk_lot_surface) = lot_surface
            .extrude(0.0, self.shrink.evaluate(lot))
            .ok_or("Couldn't shrink lot surface")?;
        collector.collect_surface(self.ground_material.evaluate(lot), shrunk_lot_surface);
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Compact)]
pub struct LotBoundaryRule {
    pub fence_height: Variable<N>,
    pub fence_material: Choice<BuildingMaterial>,
    pub fence_gap_offset_ratio: Variable<N>,
    pub fence_gap_width_ratio: Variable<N>,
}

impl LotBoundaryRule {
    fn collect_geometry(
        &self,
        collector: &mut BuildingGeometryCollector,
        lot: &Lot,
    ) -> Result<(), String> {
        let road_boundary = lot.longest_road_boundary();
        let start_point_along_road_boundary =
            road_boundary.length() * self.fence_gap_offset_ratio.evaluate(lot);
        let start_point = road_boundary.along(start_point_along_road_boundary);
        let lot_boundary = lot.area.primitives[0].boundary.path();
        let (start_point_along_lot_boundary, _) = lot_boundary
            .project(start_point)
            .ok_or("Can't reproject gap onto lot boundary")?;
        let gap_width = road_boundary.length() * self.fence_gap_width_ratio.evaluate(lot);
        let fence_path = lot_boundary
            .subsection(
                start_point_along_lot_boundary + gap_width / 2.0,
                start_point_along_lot_boundary - gap_width / 2.0,
            )
            .ok_or("Couldnt cut gap in lot boundary")?;
        let (fence_surface, _) = SculptLine::extrude(
            &Rc::new(SculptLine::new(fence_path, 0.0)),
            self.fence_height.evaluate(lot),
            0.0,
        )
        .ok_or("Couldn't extrude fence")?;
        collector.collect_surface(self.fence_material.evaluate(lot), fence_surface);
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Compact)]
pub struct FundamentRule {
    pub major_axis_angle_rel_to_road: Variable<N>,
    pub offset_on_minor_axis: Variable<N>,
    pub width: Variable<N>,
    pub max_length: Variable<N>,
    pub padding: Variable<N>,
}

const MAJOR_AXIS_RAY_HALF_LENGTH: f32 = 1000.0;

impl FundamentRule {
    fn evaluate(&self, lot: &Lot) -> Result<SkeletonSpine, String> {
        let road_direction = lot.best_road_connection().1.orthogonal_left();
        // TODO: rotate according to major_axis_angle_rel_to_road
        let major_axis_direction = road_direction;
        let minor_axis_direction = major_axis_direction.orthogonal_right();
        let spine_center_point =
            lot.center_point() + self.offset_on_minor_axis.evaluate(lot) * minor_axis_direction;

        let padding = self.padding.evaluate(lot);

        let major_axis_line_path = LinePath::new(
            vec![
                spine_center_point + MAJOR_AXIS_RAY_HALF_LENGTH * major_axis_direction,
                spine_center_point - MAJOR_AXIS_RAY_HALF_LENGTH * major_axis_direction,
            ]
            .into(),
        )
        .ok_or("Should be able to construct major axis line path")?;
        let intersections = (
            &major_axis_line_path,
            lot.area.primitives[0].boundary.path(),
        )
            .intersect();

        let intersection_before = intersections
            .iter()
            .find(|i| i.along_a < MAJOR_AXIS_RAY_HALF_LENGTH)
            .ok_or("Couldn't find suitable back lot intersection")?;
        let intersection_after = intersections
            .iter()
            .find(|i| i.along_a > MAJOR_AXIS_RAY_HALF_LENGTH)
            .ok_or("Couldn't find suitable front lot intersection")?;

        if intersection_after.along_a - intersection_before.along_a < 2.0 * padding {
            return Err("Lot intersections too close to allow for padding".to_owned());
        }

        let available_length = intersection_after.along_a - intersection_before.along_a;
        let max_length = self.max_length.evaluate(lot);
        let effective_padding = ((available_length - max_length) / 2.0).max(padding);

        let skeleton_path = major_axis_line_path
            .subsection(
                intersection_before.along_a + effective_padding,
                intersection_after.along_a - effective_padding,
            )
            .ok_or_else(|| "Couldn't construct fundament skeleton spine path")?;

        let width = self.width.evaluate(lot);

        SkeletonSpine::new(Rc::new(SculptLine::new(skeleton_path, 0.0)), width)
            .ok_or_else(|| "Couldn't construct fundament skeleton spine".to_owned())
    }
}

#[derive(Clone, Serialize, Deserialize, Compact)]
pub struct FloorRule {
    pub height: Variable<N>,
    pub widen_by_next: Variable<N>,
    pub extend_by_next: Variable<N>,
    pub front: RuleRef<FacadeRule>,
    pub back: RuleRef<FacadeRule>,
    pub left: RuleRef<FacadeRule>,
    pub right: RuleRef<FacadeRule>,
}

impl FloorRule {
    fn collect_geometry(
        &self,
        base_spine: SkeletonSpine,
        collector: &mut BuildingGeometryCollector,
        lot: &Lot,
        all_rules: &CHashMap<Name, ArchitectureRule>,
    ) -> Result<SkeletonSpine, String> {
        let (_, upper_spine) = base_spine
            .extrude(self.height.evaluate(lot), 0.0, 0.0)
            .ok_or("Couldn't extrude floor upward.")?;

        self.front.resolve(all_rules)?.collect_geometry(
            base_spine.front,
            upper_spine.front.clone(),
            collector,
            lot,
            all_rules,
        )?;
        self.back.resolve(all_rules)?.collect_geometry(
            base_spine.back,
            upper_spine.back.clone(),
            collector,
            lot,
            all_rules,
        )?;
        self.left.resolve(all_rules)?.collect_geometry(
            base_spine.left,
            upper_spine.left.clone(),
            collector,
            lot,
            all_rules,
        )?;
        self.right.resolve(all_rules)?.collect_geometry(
            base_spine.right,
            upper_spine.right.clone(),
            collector,
            lot,
            all_rules,
        )?;

        let (_, next_spine) = upper_spine
            .extrude(
                0.0,
                self.widen_by_next.evaluate(lot),
                self.extend_by_next.evaluate(lot),
            )
            .ok_or("Couldn't extrude floor outward.")?;
        Ok(next_spine)
    }
}

#[derive(Clone, Serialize, Deserialize, Compact)]
pub struct WeightedRule {
    rule: RuleRef<FacadeRule>,
    weight: Variable<N>,
}

#[derive(Clone, Serialize, Deserialize, Compact)]
pub enum FacadeRule {
    Face(
        Choice<BuildingMaterial>,
        CVec<Choice<RuleRef<FacadeDecorationRule>>>,
    ),
    Subdivision(CVec<WeightedRule>),
}

impl FacadeRule {
    fn collect_geometry(
        &self,
        base_line: Rc<SculptLine>,
        upper_line: Rc<SculptLine>,
        collector: &mut BuildingGeometryCollector,
        lot: &Lot,
        all_rules: &CHashMap<Name, ArchitectureRule>,
    ) -> Result<(), String> {
        match *self {
            FacadeRule::Face(ref wall_material, ref decorations) => {
                collector.collect_surface(
                    wall_material.evaluate(lot),
                    SpannedSurface::new(base_line.clone(), upper_line),
                );
                for decoration_choice in decorations {
                    decoration_choice
                        .evaluate(lot)
                        .resolve(all_rules)?
                        .collect_geometry(base_line.clone(), collector, lot)?;
                }
                Ok(())
            }
            FacadeRule::Subdivision(ref rules_with_weights) => {
                let weights = rules_with_weights
                    .iter()
                    .map(|rw| rw.weight.evaluate(lot))
                    .collect::<Vec<_>>();
                let subdivided_lines = base_line.subdivide(&weights);
                let subdivided_upper_lines = upper_line.subdivide(&weights);
                for ((line_seg, upper_line_seg), WeightedRule { rule, .. }) in subdivided_lines
                    .iter()
                    .zip(subdivided_upper_lines.iter())
                    .zip(rules_with_weights)
                {
                    rule.resolve(all_rules)?.collect_geometry(
                        line_seg.clone(),
                        upper_line_seg.clone(),
                        collector,
                        lot,
                        all_rules,
                    )?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct FacadeDecorationRule {
    pub prop: BuildingProp,
    pub color: [Variable<N>; 3],
    pub spacing: Variable<N>,
}

impl FacadeDecorationRule {
    fn collect_geometry(
        &self,
        base_line: Rc<SculptLine>,
        collector: &mut BuildingGeometryCollector,
        lot: &Lot,
    ) -> Result<(), String> {
        let n_spacings =
            (base_line.path.length() / self.spacing.evaluate(lot)).floor() as usize + 1;
        let effective_spacing = base_line.path.length() / (n_spacings as f32);
        let color = [
            self.color[0].evaluate(lot),
            self.color[1].evaluate(lot),
            self.color[2].evaluate(lot),
        ];
        let instances = (1..n_spacings)
            .map(|i| {
                let along = i as f32 * effective_spacing;
                let pos = base_line.path.along(along);
                let direction = base_line.path.direction_along(along);
                Instance {
                    instance_position: [pos.x, pos.y, base_line.z],
                    instance_color: color,
                    instance_direction: [direction.x, direction.y],
                }
            })
            .collect();
        collector.collect_props(self.prop, instances);
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Compact)]
pub struct RoofRule {
    pub height: Variable<N>,
    pub gable_depth_front: Variable<N>,
    pub gable_depth_back: Variable<N>,
    pub roof_material: Choice<BuildingMaterial>,
    pub gable_material: Choice<BuildingMaterial>,
}

impl RoofRule {
    fn collect_geometry(
        &self,
        base_spine: SkeletonSpine,
        collector: &mut BuildingGeometryCollector,
        lot: &Lot,
    ) {
        let (roof_surface, gable_surface) = base_spine.roof(
            self.height.evaluate(lot),
            self.gable_depth_front.evaluate(lot),
            self.gable_depth_back.evaluate(lot),
        );
        collector.collect_surface(self.roof_material.evaluate(lot), roof_surface);
        collector.collect_surface(self.gable_material.evaluate(lot), gable_surface);
    }
}
