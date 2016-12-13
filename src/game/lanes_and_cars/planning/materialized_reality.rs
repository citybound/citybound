use kay::{ID, ActorSystem, Recipient, Fate, Individual, CDict, CVec};
use super::{Plan, PlanResult, PlanDelta, PlanResultDelta, IntersectionRef, TrimmedStrokeRef, TransferStrokeRef, RemainingOldStrokes};

pub struct MaterializedReality{
    current_plan: Plan,
    current_result: PlanResult,
    built_intersection_lanes: CDict<IntersectionRef, CVec<ID>>,
    built_trimmed_lanes: CDict<TrimmedStrokeRef, ID>,
    built_transfer_lanes: CDict<TransferStrokeRef, ID>
}
impl Individual for MaterializedReality{}

#[derive(Compact, Clone)]
pub struct Simulate{pub requester: ID, pub delta: PlanDelta}

#[derive(Compact, Clone)]
pub struct SimulationResult{
    pub remaining_old_strokes: RemainingOldStrokes,
    pub result_delta: PlanResultDelta
}

impl Recipient<Simulate> for MaterializedReality {
    fn receive(&mut self, msg: &Simulate) -> Fate {match *msg{
        Simulate{requester, ref delta} => {
            let (new_plan, remaining_old_strokes) = self.current_plan.with_delta(delta);
            let result = new_plan.get_result();
            let result_delta = result.delta(&self.current_result);
            requester << SimulationResult{
                remaining_old_strokes: remaining_old_strokes,
                result_delta: result_delta,
            };
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub struct Apply{pub requester: ID, pub delta: PlanDelta}

use super::super::Unbuild;

impl Recipient<Apply> for MaterializedReality {
    #[inline(never)]
    fn receive(&mut self, msg: &Apply) -> Fate {match *msg{
        Apply{requester, ref delta} => {
            let (new_plan, _) = self.current_plan.with_delta(delta);
            let new_result = new_plan.get_result();
            let result_delta = new_result.delta(&self.current_result);

            for old_ref in result_delta.intersections.to_destroy.keys() {
                for id in self.built_intersection_lanes.remove_iter(*old_ref) {
                    id << Unbuild;
                }
            }

            for old_ref in result_delta.trimmed_strokes.to_destroy.keys() {
                let id = self.built_trimmed_lanes.remove(*old_ref).expect("tried to unbuild a non-existing lane");
                id << Unbuild;
            }

            for old_ref in result_delta.transfer_strokes.to_destroy.keys() {
                let id = self.built_transfer_lanes.remove(*old_ref).expect("tried to unbuild a non-existing transfer lane");
                id << Unbuild;
            }

            for (&IntersectionRef(new_index), new_intersection) in result_delta.intersections.to_create.pairs() {
                for (stroke, timings) in new_intersection.strokes.iter().zip(new_intersection.timings.iter()) {
                    stroke.build_intersection(MaterializedReality::id(), BuildableRef::Intersection(new_index), timings.clone());
                }
            }

            for (&TrimmedStrokeRef(new_index), new_stroke) in result_delta.trimmed_strokes.to_create.pairs() {
                new_stroke.build(MaterializedReality::id(), BuildableRef::TrimmedStroke(new_index));
            }

            for (&TransferStrokeRef(new_index), new_stroke) in result_delta.transfer_strokes.to_create.pairs() {
                new_stroke.build_transfer(MaterializedReality::id(), BuildableRef::TransferStroke(new_index));
            }

            let new_built_intersection_lanes = self.built_intersection_lanes.pairs().map(|(old_ref, ids)| {
                let new_ref = result_delta.intersections.old_to_new.get(*old_ref).expect("attempted to resurrect a destroyed intersection");
                (*new_ref, ids.clone())
            }).collect();

            let new_built_trimmed_lanes = self.built_trimmed_lanes.pairs().map(|(old_ref, id)| {
                let new_ref = result_delta.trimmed_strokes.old_to_new.get(*old_ref).expect("attempted to resurrect a destroyed trimmed stroke");
                (*new_ref, *id)
            }).collect();

            let new_built_transfer_lanes = self.built_transfer_lanes.pairs().map(|(old_ref, id)| {
                let new_ref = result_delta.transfer_strokes.old_to_new.get(*old_ref).expect("attempted to resurrect a destroyed transfer stroke");
                (*new_ref, *id)
            }).collect();

            *self = MaterializedReality{
                current_plan: new_plan,
                current_result: new_result,
                built_intersection_lanes: new_built_intersection_lanes,
                built_trimmed_lanes: new_built_trimmed_lanes,
                built_transfer_lanes: new_built_transfer_lanes
            };

            Self::id() << Simulate{requester: requester, delta: PlanDelta::default()};

            Fate::Live
        }
    }}
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BuildableRef{
    Intersection(usize),
    TrimmedStroke(usize),
    TransferStroke(usize)
}

#[derive(Copy, Clone)]
pub struct ReportLaneBuilt(pub ID, pub BuildableRef);
use super::super::AdvertiseForOverlaps;

impl Recipient<ReportLaneBuilt> for MaterializedReality {
    fn receive(&mut self, msg: &ReportLaneBuilt) -> Fate {match *msg{
        ReportLaneBuilt(id, buildable_ref) => {
            match buildable_ref {
                BuildableRef::Intersection(index) => {
                    if let Some(other_intersection_lanes) = self.built_intersection_lanes.get(IntersectionRef(index)) {
                        id << AdvertiseForOverlaps{lanes: other_intersection_lanes.clone()};
                    }
                    self.built_intersection_lanes.push_at(IntersectionRef(index), id);
                },
                BuildableRef::TrimmedStroke(index) => {self.built_trimmed_lanes.insert(TrimmedStrokeRef(index), id);},
                BuildableRef::TransferStroke(index) => {self.built_transfer_lanes.insert(TransferStrokeRef(index), id);}
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
            built_trimmed_lanes: CDict::new(),
            built_transfer_lanes: CDict::new()
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(MaterializedReality::default());
    system.add_inbox::<Simulate, MaterializedReality>();
    system.add_inbox::<Apply, MaterializedReality>();
    system.add_inbox::<ReportLaneBuilt, MaterializedReality>();
}