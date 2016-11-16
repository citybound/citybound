use kay::{ID, ActorSystem, Recipient, Fate, Individual, CDict, CVec};
use super::{Plan, PlanResult, PlanDelta, PlanResultDelta, IntersectionRef, InbetweenStrokeRef, RemainingOldStrokes};

pub struct MaterializedReality{
    current_plan: Plan,
    current_result: PlanResult,
    built_intersection_lanes: CDict<IntersectionRef, CVec<ID>>,
    built_inbetween_lanes: CDict<InbetweenStrokeRef, CVec<ID>>
}
impl Individual for MaterializedReality{}

#[derive(Compact, Clone)]
pub struct Simulate{pub requester: ID, pub delta: PlanDelta, pub fresh: bool}

#[derive(Compact, Clone)]
pub struct SimulationResult{
    pub remaining_old_strokes: RemainingOldStrokes,
    pub result: PlanResult,
    pub result_delta: PlanResultDelta,
    pub fresh: bool
}

impl Recipient<Simulate> for MaterializedReality {
    fn receive(&mut self, msg: &Simulate) -> Fate {match *msg{
        Simulate{requester, ref delta, fresh} => {
            let (new_plan, remaining_old_strokes) = self.current_plan.with_delta(delta);
            let result = new_plan.get_result();
            let result_delta = result.delta(&self.current_result);
            requester << SimulationResult{
                remaining_old_strokes: remaining_old_strokes,
                result: result,
                result_delta: result_delta,
                fresh: fresh
            };
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub struct Apply{pub requester: ID, pub delta: PlanDelta}

use super::super::Unbuild;

impl Recipient<Apply> for MaterializedReality {
    fn receive(&mut self, msg: &Apply) -> Fate {match *msg{
        Apply{requester, ref delta} => {
            let (new_plan, _) = self.current_plan.with_delta(delta);
            let new_result = new_plan.get_result();
            let result_delta = new_result.delta(&self.current_result);

            for old_ref in result_delta.intersections_to_destroy.keys() {
                for id in self.built_intersection_lanes.remove_iter(*old_ref) {
                    id << Unbuild;
                }
            }

            for old_ref in result_delta.inbetween_strokes_to_destroy.keys() {
                for id in self.built_inbetween_lanes.remove_iter(*old_ref) {
                    id << Unbuild;
                }
            }

            for (&IntersectionRef(new_index), new_intersection) in result_delta.new_intersections.pairs() {
                for stroke in &new_intersection.strokes {
                    stroke.build(MaterializedReality::id(), BuildableRef::Intersection(new_index));
                }
            }

            for (&InbetweenStrokeRef(new_index), new_stroke) in result_delta.new_inbetween_strokes.pairs() {
                new_stroke.build(MaterializedReality::id(), BuildableRef::InBetweenStroke(new_index));
            }

            let new_built_intersection_lanes = self.built_intersection_lanes.pairs().map(|(old_ref, ids)| {
                let new_ref = result_delta.old_to_new_intersection_map.get(*old_ref).expect("attempted to resurrect a destroyed intersection");
                (*new_ref, ids.clone())
            }).collect();

            let new_built_inbetween_lanes = self.built_inbetween_lanes.pairs().map(|(old_ref, ids)| {
                let new_ref = result_delta.old_to_new_stroke_map.get(*old_ref).expect("attempted to resurrect a destroyed inbetween stroke");
                (*new_ref, ids.clone())
            }).collect();

            *self = MaterializedReality{
                current_plan: new_plan,
                current_result: new_result,
                built_intersection_lanes: new_built_intersection_lanes,
                built_inbetween_lanes: new_built_inbetween_lanes
            };

            Self::id() << Simulate{requester: requester, delta: PlanDelta::default(), fresh: true};

            Fate::Live
        }
    }}
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BuildableRef{
    InBetweenStroke(usize),
    Intersection(usize)
}

#[derive(Copy, Clone)]
pub struct ReportLaneBuilt(pub ID, pub BuildableRef);

impl Recipient<ReportLaneBuilt> for MaterializedReality {
    fn receive(&mut self, msg: &ReportLaneBuilt) -> Fate {match *msg{
        ReportLaneBuilt(id, buildable_ref) => {
            match buildable_ref {
                BuildableRef::Intersection(index) => self.built_intersection_lanes.push_at(IntersectionRef(index), id),
                BuildableRef::InBetweenStroke(index) => self.built_inbetween_lanes.push_at(InbetweenStrokeRef(index), id)
            }
            Fate::Live
        }
    }}
}

impl Default for MaterializedReality {
    fn default() -> Self {
        MaterializedReality{
            current_plan: Plan::default(),
            current_result: PlanResult::default(),
            built_intersection_lanes: CDict::new(),
            built_inbetween_lanes: CDict::new()
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(MaterializedReality::default());
    system.add_inbox::<Simulate, MaterializedReality>();
    system.add_inbox::<Apply, MaterializedReality>();
    system.add_inbox::<ReportLaneBuilt, MaterializedReality>();
}