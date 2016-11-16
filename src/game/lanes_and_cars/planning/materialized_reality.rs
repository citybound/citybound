use kay::{ID, ActorSystem, Recipient, Fate, Individual, CDict, CVec};
use super::{Plan, PlanResult, PlanDelta, PlanResultDelta, IntersectionRef, InbetweenStrokeRef};

pub struct MaterializedReality{
    current_plan: Plan,
    current_result: PlanResult,
    built_intersection_lanes: CDict<IntersectionRef, CVec<ID>>,
    built_inbetween_lanes: CDict<InbetweenStrokeRef, CVec<ID>>
}
impl Individual for MaterializedReality{}

#[derive(Compact, Clone)]
pub struct Simulate{pub requester: ID, pub delta: PlanDelta}

#[derive(Compact, Clone)]
pub struct SimulationResult{pub result: PlanResult, pub result_delta: PlanResultDelta}

impl Recipient<Simulate> for MaterializedReality {
    fn receive(&mut self, msg: &Simulate) -> Fate {match *msg{
        Simulate{requester, ref delta} => {
            let result = self.current_plan.with_delta(delta).get_result();
            let result_delta = result.delta(&self.current_result);
            requester << SimulationResult{result: result, result_delta: result_delta};
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub struct Apply{pub delta: PlanDelta}

use super::super::Unbuild;

impl Recipient<Apply> for MaterializedReality {
    fn receive(&mut self, msg: &Apply) -> Fate {match *msg{
        Apply{ref delta} => {
            let with_delta = self.current_plan.with_delta(delta);
            let new_result = with_delta.get_result();
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
                let new_ref = result_delta.old_to_new_intersection_map.get(*old_ref).unwrap();
                (*new_ref, ids.clone())
            }).collect();

            let new_built_inbetween_lanes = self.built_inbetween_lanes.pairs().map(|(old_ref, ids)| {
                let new_ref = result_delta.old_to_new_stroke_map.get(*old_ref).unwrap();
                (*new_ref, ids.clone())
            }).collect();

            *self = MaterializedReality{
                current_plan: with_delta,
                current_result: new_result,
                built_intersection_lanes: new_built_intersection_lanes,
                built_inbetween_lanes: new_built_inbetween_lanes
            };

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