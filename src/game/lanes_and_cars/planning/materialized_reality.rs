use compact::{CDict, CVec};
use kay::{ID, ActorSystem, Recipient, Fate, Individual};
use super::{Plan, PlanResult, PlanDelta, PlanResultDelta, IntersectionRef, TrimmedStrokeRef,
            TransferStrokeRef, RemainingOldStrokes};

#[derive(Clone)]
pub struct MaterializedRealityState {
    current_plan: Plan,
    current_result: PlanResult,
    built_intersection_lanes: CDict<IntersectionRef, CVec<ID>>,
    built_trimmed_lanes: CDict<TrimmedStrokeRef, ID>,
    built_transfer_lanes: CDict<TransferStrokeRef, ID>,
}

pub enum MaterializedReality {
    Ready(MaterializedRealityState),
    WaitingForUnbuild(ID, CVec<ID>, MaterializedRealityState, Plan, PlanResult, PlanResultDelta),
}
impl Individual for MaterializedReality {}
use self::MaterializedReality::{Ready, WaitingForUnbuild};

#[derive(Compact, Clone)]
pub struct Simulate {
    pub requester: ID,
    pub delta: PlanDelta,
}

#[derive(Compact, Clone)]
pub struct SimulationResult {
    pub remaining_old_strokes: RemainingOldStrokes,
    pub result_delta: PlanResultDelta,
}

impl Recipient<Simulate> for MaterializedReality {
    fn receive(&mut self, msg: &Simulate) -> Fate {
        match *msg {
            Simulate { requester, ref delta } => {
                let state = match *self {
                    Ready(ref state) |
                    WaitingForUnbuild(_, _, ref state, _, _, _) => state,
                };
                let (new_plan, remaining_old_strokes) = state.current_plan.with_delta(delta);
                let result = new_plan.get_result();
                let result_delta = result.delta(&state.current_result);
                requester <<
                SimulationResult {
                    remaining_old_strokes: remaining_old_strokes,
                    result_delta: result_delta,
                };
                Fate::Live
            }
        }
    }
}

#[derive(Compact, Clone)]
pub struct Apply {
    pub requester: ID,
    pub delta: PlanDelta,
}

use super::super::Unbuild;

impl Recipient<Apply> for MaterializedReality {
    #[inline(never)]
    fn receive(&mut self, msg: &Apply) -> Fate {
        match *msg {
            Apply { ref delta, requester } => {
                *self = match *self {
                    WaitingForUnbuild(..) => panic!("Already applying a plan"),
                    Ready(ref mut state) => {
                        let (new_plan, _) = state.current_plan.with_delta(delta);
                        let new_result = new_plan.get_result();
                        let result_delta = new_result.delta(&state.current_result);

                        let mut ids_to_unbuild = CVec::new();

                        for old_ref in result_delta.intersections.to_destroy.keys() {
                            for id in state.built_intersection_lanes.remove_iter(*old_ref) {
                                ids_to_unbuild.push(id);
                            }
                        }

                        for old_ref in result_delta.trimmed_strokes.to_destroy.keys() {
                            let id = state.built_trimmed_lanes
                                .remove(*old_ref)
                                .expect("tried to unbuild a non-existing lane");
                            ids_to_unbuild.push(id);
                        }

                        for old_ref in result_delta.transfer_strokes.to_destroy.keys() {
                            let id = state.built_transfer_lanes
                                .remove(*old_ref)
                                .expect("tried to unbuild a non-existing transfer lane");
                            ids_to_unbuild.push(id);
                        }

                        for &id in &ids_to_unbuild {
                            id << Unbuild { report_to: Self::id() };
                        }

                        Self::id() << ReportLaneUnbuilt(None);
                        WaitingForUnbuild(requester,
                                          ids_to_unbuild,
                                          state.clone(),
                                          new_plan,
                                          new_result,
                                          result_delta)
                    }
                };
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BuildableRef {
    Intersection(usize),
    TrimmedStroke(usize),
    TransferStroke(usize),
}

#[derive(Copy, Clone)]
pub struct ReportLaneBuilt(pub ID, pub BuildableRef);
use super::super::AdvertiseForOverlaps;

impl Recipient<ReportLaneBuilt> for MaterializedReality {
    fn receive(&mut self, msg: &ReportLaneBuilt) -> Fate {
        match *msg {
            ReportLaneBuilt(id, buildable_ref) => {
                match *self {
                    Ready(ref mut state) => {
                        match buildable_ref {
                            BuildableRef::Intersection(index) => {
                                if let Some(other_intersection_lanes) =
                                    state.built_intersection_lanes.get(IntersectionRef(index)) {
                                    id <<
                                    AdvertiseForOverlaps {
                                        lanes: other_intersection_lanes.clone(),
                                    };
                                }
                                state.built_intersection_lanes.push_at(IntersectionRef(index), id);
                            }
                            BuildableRef::TrimmedStroke(index) => {
                                state.built_trimmed_lanes.insert(TrimmedStrokeRef(index), id);
                            }
                            BuildableRef::TransferStroke(index) => {
                                state.built_transfer_lanes.insert(TransferStrokeRef(index), id);
                            }
                        }
                    }
                    WaitingForUnbuild(..) => {
                        panic!("a waiting materialized reality shouldn't get build reports")
                    }
                }
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct ReportLaneUnbuilt(pub Option<ID>);

impl Recipient<ReportLaneUnbuilt> for MaterializedReality {
    fn receive(&mut self, msg: &ReportLaneUnbuilt) -> Fate {
        match *msg {
            ReportLaneUnbuilt(maybe_id) => {
                let maybe_new_self = match *self {
                    WaitingForUnbuild(requester,
                                      ref mut ids_to_unbuild,
                                      ref state,
                                      ref new_plan,
                                      ref new_result,
                                      ref result_delta) => {
                        if let Some(id) = maybe_id {
                            let pos = ids_to_unbuild.iter()
                                .position(|unbuild_id| *unbuild_id == id)
                                .expect("Trying to delete unexpected id");
                            ids_to_unbuild.remove(pos);
                        }
                        if ids_to_unbuild.is_empty() {
                            for (&IntersectionRef(new_index), new_intersection) in
                                result_delta.intersections.to_create.pairs() {
                                for (stroke, timings) in
                                    new_intersection.strokes
                                        .iter()
                                        .zip(new_intersection.timings.iter()) {
                                    stroke.build_intersection(MaterializedReality::id(),
                                                            BuildableRef::Intersection(new_index),
                                                            timings.clone());
                                }
                            }

                            for (&TrimmedStrokeRef(new_index), new_stroke) in
                                result_delta.trimmed_strokes.to_create.pairs() {
                                new_stroke.build(MaterializedReality::id(),
                                                 BuildableRef::TrimmedStroke(new_index));
                            }

                            for (&TransferStrokeRef(new_index), new_stroke) in
                                result_delta.transfer_strokes.to_create.pairs() {
                                new_stroke.build_transfer(MaterializedReality::id(),
                                                          BuildableRef::TransferStroke(new_index));
                            }

                            let new_built_intersection_lanes = state.built_intersection_lanes
                                .pairs()
                                .map(|(old_ref, ids)| {
                                    let new_ref = result_delta.intersections
                                        .old_to_new
                                        .get(*old_ref)
                                        .expect("attempted to resurrect a destroyed intersection");
                                    (*new_ref, ids.clone())
                                })
                                .collect();

                            let new_built_trimmed_lanes = state.built_trimmed_lanes
                                .pairs()
                                .map(|(old_ref, id)| {
                                    let new_ref = result_delta.trimmed_strokes
                                        .old_to_new
                                        .get(*old_ref)
                                        .expect("attempted to resurrect a destroyed trimmed \
                                                 stroke");
                                    (*new_ref, *id)
                                })
                                .collect();

                            let new_built_transfer_lanes = state.built_transfer_lanes
                                .pairs()
                                .map(|(old_ref, id)| {
                                    let new_ref = result_delta.transfer_strokes
                                        .old_to_new
                                        .get(*old_ref)
                                        .expect("attempted to resurrect a destroyed transfer \
                                                 stroke");
                                    (*new_ref, *id)
                                })
                                .collect();

                            Self::id() <<
                            Simulate {
                                requester: requester,
                                delta: PlanDelta::default(),
                            };

                            Some(Ready(MaterializedRealityState {
                                current_plan: new_plan.clone(),
                                current_result: new_result.clone(),
                                built_intersection_lanes: new_built_intersection_lanes,
                                built_trimmed_lanes: new_built_trimmed_lanes,
                                built_transfer_lanes: new_built_transfer_lanes,
                            }))
                        } else {
                            None
                        }
                    }
                    Ready(_) => panic!("Can't unbuild when materialized reality is in ready state"),
                };
                if let Some(new_self) = maybe_new_self {
                    *self = new_self;
                }
                Fate::Live
            }
        }
    }
}

impl Default for MaterializedReality {
    fn default() -> Self {
        Ready(MaterializedRealityState {
            current_plan: Plan::default(),
            current_result: PlanResult::default(),
            built_intersection_lanes: CDict::new(),
            built_trimmed_lanes: CDict::new(),
            built_transfer_lanes: CDict::new(),
        })
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(MaterializedReality::default());
    system.add_inbox::<Simulate, MaterializedReality>();
    system.add_inbox::<Apply, MaterializedReality>();
    system.add_inbox::<ReportLaneBuilt, MaterializedReality>();
    system.add_inbox::<ReportLaneUnbuilt, MaterializedReality>();
}
