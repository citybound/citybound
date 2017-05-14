use compact::{CDict, CVec};
use kay::{ID, ActorSystem, Fate};
use super::super::planning::plan::{Plan, PlanResult, PlanDelta, PlanResultDelta, IntersectionRef,
                                   TrimmedStrokeRef, TransferStrokeRef, BuiltStrokes};

#[derive(Clone)]
pub struct MaterializedRealityState {
    current_plan: Plan,
    current_result: PlanResult,
    built_intersection_lanes: CDict<IntersectionRef, CVec<ID>>,
    built_trimmed_lanes: CDict<TrimmedStrokeRef, ID>,
    built_transfer_lanes: CDict<TransferStrokeRef, ID>,
}

#[allow(large_enum_variant)]
pub enum MaterializedReality {
    Ready(MaterializedRealityState),
    WaitingForUnbuild(ID, CVec<ID>, MaterializedRealityState, Plan, PlanResult, PlanResultDelta),
}
use self::MaterializedReality::{Ready, WaitingForUnbuild};

#[derive(Compact, Clone)]
pub struct Simulate {
    pub requester: ID,
    pub delta: PlanDelta,
}

#[derive(Compact, Clone)]
pub struct SimulationResult(pub PlanResultDelta);

#[derive(Compact, Clone)]
pub struct BuiltStrokesChanged(pub BuiltStrokes);

pub fn setup(system: &mut ActorSystem) {
    system.add(MaterializedReality::default(), |mut the_mr| {
        let mr_id = the_mr.world().id::<MaterializedReality>();

        use super::super::planning::plan::LaneStrokeRef;

        the_mr.on(|&Simulate { requester, ref delta }, mr, world| {
            let state = match *mr {
                Ready(ref state) |
                WaitingForUnbuild(_, _, ref state, _, _, _) => state,
            };
            let (new_plan, _) = state.current_plan.with_delta(delta);
            let result = new_plan.get_result();
            let result_delta = result.delta(&state.current_result);
            world.send(requester, SimulationResult(result_delta));
            Fate::Live
        });

        use super::super::construction::Unbuild;

        the_mr.on(move |&Apply { ref delta, requester }, mr, world| {
            *mr = match *mr {
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
                        let id = state
                            .built_trimmed_lanes
                            .remove(*old_ref)
                            .expect("tried to unbuild a non-existing lane");
                        ids_to_unbuild.push(id);
                    }

                    for old_ref in result_delta.transfer_strokes.to_destroy.keys() {
                        let id = state
                            .built_transfer_lanes
                            .remove(*old_ref)
                            .expect("tried to unbuild a non-existing transfer lane");
                        ids_to_unbuild.push(id);
                    }

                    for &id in &ids_to_unbuild {
                        world.send(id, Unbuild { report_to: mr_id });
                    }

                    world.send(mr_id, ReportLaneUnbuilt(None));
                    WaitingForUnbuild(requester,
                                      ids_to_unbuild,
                                      state.clone(),
                                      new_plan,
                                      new_result,
                                      result_delta)
                }
            };
            Fate::Live
        });

        use super::AdvertiseForOverlaps;

        the_mr.on(|&ReportLaneBuilt(id, buildable_ref), mr, world| {
            match *mr {
                Ready(ref mut state) => {
                    match buildable_ref {
                        BuildableRef::Intersection(index) => {
                            if let Some(other_intersection_lanes) =
                                state
                                    .built_intersection_lanes
                                    .get(IntersectionRef(index)) {
                                world.send(id,
                                           AdvertiseForOverlaps {
                                               lanes: other_intersection_lanes.clone(),
                                           });
                            }
                            state
                                .built_intersection_lanes
                                .push_at(IntersectionRef(index), id);
                        }
                        BuildableRef::TrimmedStroke(index) => {
                            state
                                .built_trimmed_lanes
                                .insert(TrimmedStrokeRef(index), id);
                        }
                        BuildableRef::TransferStroke(index) => {
                            state
                                .built_transfer_lanes
                                .insert(TransferStrokeRef(index), id);
                        }
                    }
                }
                WaitingForUnbuild(..) => {
                    panic!("a waiting materialized reality
                                shouldn't get build reports")
                }
            }
            Fate::Live
        });

        the_mr.on(move |&ReportLaneUnbuilt(maybe_id), mr, world| {
            let maybe_new_mr = match *mr {
                WaitingForUnbuild(requester,
                                  ref mut ids_to_unbuild,
                                  ref state,
                                  ref new_plan,
                                  ref new_result,
                                  ref result_delta) => {
                    if let Some(id) = maybe_id {
                        let pos = ids_to_unbuild
                            .iter()
                            .position(|unbuild_id| *unbuild_id == id)
                            .expect("Trying to delete unexpected id");
                        ids_to_unbuild.remove(pos);
                    }
                    if ids_to_unbuild.is_empty() {
                        for (&IntersectionRef(new_index), new_intersection) in
                            result_delta.intersections.to_create.pairs() {
                            for (stroke, timings) in
                                new_intersection
                                    .strokes
                                    .iter()
                                    .zip(new_intersection.timings.iter()) {
                                stroke.build_intersection(mr_id,
                                                          BuildableRef::Intersection(new_index),
                                                          timings.clone(),
                                                          world);
                            }
                        }

                        for (&TrimmedStrokeRef(new_index), new_stroke) in
                            result_delta.trimmed_strokes.to_create.pairs() {
                            new_stroke.build(mr_id, BuildableRef::TrimmedStroke(new_index), world);
                        }

                        for (&TransferStrokeRef(new_index), new_stroke) in
                            result_delta.transfer_strokes.to_create.pairs() {
                            new_stroke.build_transfer(mr_id,
                                                      BuildableRef::TransferStroke(new_index),
                                                      world);
                        }

                        let new_built_intersection_lanes =
                            state
                                .built_intersection_lanes
                                .pairs()
                                .map(|(old_ref, ids)| {
                                    let new_ref = result_delta.intersections
                                        .old_to_new
                                        .get(*old_ref)
                                        .expect("attempted to resurrect a destroyed intersection");
                                    (*new_ref, ids.clone())
                                })
                                .collect();

                        let new_built_trimmed_lanes = state
                            .built_trimmed_lanes
                            .pairs()
                            .map(|(old_ref, id)| {
                                let new_ref = result_delta
                                    .trimmed_strokes
                                    .old_to_new
                                    .get(*old_ref)
                                    .expect("attempted to resurrect a destroyed trimmed \
                                                 stroke");
                                (*new_ref, *id)
                            })
                            .collect();

                        let new_built_transfer_lanes = state
                            .built_transfer_lanes
                            .pairs()
                            .map(|(old_ref, id)| {
                                let new_ref = result_delta
                                    .transfer_strokes
                                    .old_to_new
                                    .get(*old_ref)
                                    .expect("attempted to resurrect a destroyed transfer \
                                                 stroke");
                                (*new_ref, *id)
                            })
                            .collect();

                        let built_strokes = BuiltStrokes {
                            mapping: new_plan
                                .strokes
                                .iter()
                                .enumerate()
                                .map(|(idx, stroke)| (LaneStrokeRef(idx), stroke.clone()))
                                .collect(),
                        };

                        world.send(requester, BuiltStrokesChanged(built_strokes));

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
            if let Some(new_mr) = maybe_new_mr {
                *mr = new_mr;
            }
            Fate::Live
        })
    })
}

#[derive(Compact, Clone)]
pub struct Apply {
    pub requester: ID,
    pub delta: PlanDelta,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BuildableRef {
    Intersection(usize),
    TrimmedStroke(usize),
    TransferStroke(usize),
}

#[derive(Copy, Clone)]
pub struct ReportLaneBuilt(pub ID, pub BuildableRef);

#[derive(Copy, Clone)]
pub struct ReportLaneUnbuilt(pub Option<ID>);

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
