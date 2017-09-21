use compact::{CDict, CVec};
use kay::{ID, ActorSystem, Fate, World};
use kay::swarm::Swarm;
use super::super::planning::plan::{Plan, PlanResult, PlanDelta, LaneStrokeRef, PlanResultDelta,
                                   IntersectionRef, TrimmedStrokeRef, TransferStrokeRef,
                                   BuiltStrokes};
use super::super::planning::current_plan::CurrentPlanID;
use super::{Unbuild, AdvertiseForOverlaps};

#[derive(Compact, Clone)]
pub struct MaterializedReality {
    id: MaterializedRealityID,
    current_plan: Plan,
    current_result: PlanResult,
    built_intersection_lanes: CDict<IntersectionRef, CVec<ID>>,
    built_trimmed_lanes: CDict<TrimmedStrokeRef, ID>,
    built_transfer_lanes: CDict<TransferStrokeRef, ID>,
    state: MaterializedRealityState,
}

#[derive(Compact, Clone)]
#[allow(large_enum_variant)]
pub enum MaterializedRealityState {
    Ready(()),
    WaitingForUnbuild(CurrentPlanID, CVec<ID>, Plan, PlanResult, PlanResultDelta),
}
use self::MaterializedRealityState::{Ready, WaitingForUnbuild};

impl MaterializedReality {
    pub fn spawn(id: MaterializedRealityID, _: &mut World) -> MaterializedReality {
        MaterializedReality {
            id,
            current_plan: Plan::default(),
            current_result: PlanResult::default(),
            built_intersection_lanes: CDict::new(),
            built_trimmed_lanes: CDict::new(),
            built_transfer_lanes: CDict::new(),
            state: MaterializedRealityState::Ready(()),
        }
    }

    pub fn simulate(&mut self, requester: CurrentPlanID, delta: &PlanDelta, world: &mut World) {
        let (new_plan, _) = self.current_plan.with_delta(delta);
        let result = new_plan.get_result();
        let result_delta = result.delta(&self.current_result);
        requester.on_simulation_result(result_delta, world);
    }

    pub fn apply(&mut self, requester: CurrentPlanID, delta: &PlanDelta, world: &mut World) {
        self.state = match self.state {
            WaitingForUnbuild(..) => panic!("Already applying a plan"),
            Ready(()) => {
                let (new_plan, _) = self.current_plan.with_delta(delta);
                    let new_result = new_plan.get_result();
                    let result_delta = new_result.delta(&self.current_result);

                    let mut ids_to_unbuild = CVec::new();

                    for old_ref in result_delta.intersections.to_destroy.keys() {
                        for id in self.built_intersection_lanes.remove_iter(*old_ref) {
                            ids_to_unbuild.push(id);
                        }
                    }

                    for old_ref in result_delta.trimmed_strokes.to_destroy.keys() {
                        let id = self.built_trimmed_lanes.remove(*old_ref).expect(
                            "tried to unbuild a non-existing lane",
                        );
                        ids_to_unbuild.push(id);
                    }

                    for old_ref in result_delta.transfer_strokes.to_destroy.keys() {
                        let id = self.built_transfer_lanes.remove(*old_ref).expect(
                            "tried to unbuild a non-existing transfer lane",
                        );
                        ids_to_unbuild.push(id);
                    }

                    for &id in &ids_to_unbuild {
                        world.send(id, Unbuild { report_to: self.id });
                    }

                    self.id.on_lane_unbuilt(None, world);
                    WaitingForUnbuild(
                        requester,
                        ids_to_unbuild,
                        new_plan,
                        new_result,
                        result_delta,
                    )
            }
        }
    }

    pub fn on_lane_built(&mut self, id: ID, buildable_ref: BuildableRef, world: &mut World) {
        match self.state {
            Ready(()) => {
                    match buildable_ref {
                        BuildableRef::Intersection(index) => {
                            if let Some(other_intersection_lanes) =
                                self.built_intersection_lanes.get(IntersectionRef(index))
                            {
                                world.send(
                                    id,
                                    AdvertiseForOverlaps {
                                        lanes: other_intersection_lanes.clone(),
                                    },
                                );
                            }
                            self.built_intersection_lanes.push_at(
                                IntersectionRef(index),
                                id,
                            );
                        }
                        BuildableRef::TrimmedStroke(index) => {
                            self.built_trimmed_lanes.insert(
                                TrimmedStrokeRef(index),
                                id,
                            );
                        }
                        BuildableRef::TransferStroke(index) => {
                            self.built_transfer_lanes.insert(
                                TransferStrokeRef(index),
                                id,
                            );
                        }
                    }
                }
            WaitingForUnbuild(..) => {
                panic!(
                    "a waiting materialized reality
                                shouldn't get build reports"
                )
            }
        }
    }

    pub fn on_lane_unbuilt(&mut self, maybe_id: Option<ID>, world: &mut World) {
        let maybe_new_self = match self.state {
            WaitingForUnbuild(requester,
                              ref mut ids_to_unbuild,
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
                        result_delta.intersections.to_create.pairs()
                    {
                        for (stroke, timings) in
                            new_intersection.strokes.iter().zip(
                                new_intersection
                                    .timings
                                    .iter(),
                            )
                        {
                            stroke.build_intersection(
                                self.id,
                                BuildableRef::Intersection(new_index),
                                timings.clone(),
                                world,
                            );
                        }
                    }

                    for (&TrimmedStrokeRef(new_index), new_stroke) in
                        result_delta.trimmed_strokes.to_create.pairs()
                    {
                        new_stroke.build(self.id, BuildableRef::TrimmedStroke(new_index), world);
                    }

                    for (&TransferStrokeRef(new_index), new_stroke) in
                        result_delta.transfer_strokes.to_create.pairs()
                    {
                        new_stroke.build_transfer(
                            self.id,
                            BuildableRef::TransferStroke(new_index),
                            world,
                        );
                    }

                    let new_built_intersection_lanes = self.built_intersection_lanes
                        .pairs()
                        .map(|(old_ref, ids)| {
                            let new_ref =
                                result_delta.intersections.old_to_new.get(*old_ref).expect(
                                    "attempted to resurrect a destroyed intersection",
                                );
                            (*new_ref, ids.clone())
                        })
                        .collect();

                    let new_built_trimmed_lanes = self.built_trimmed_lanes
                        .pairs()
                        .map(|(old_ref, id)| {
                            let new_ref = result_delta
                                .trimmed_strokes
                                .old_to_new
                                .get(*old_ref)
                                .expect(
                                    "attempted to resurrect a destroyed trimmed \
                                                 stroke",
                                );
                            (*new_ref, *id)
                        })
                        .collect();

                    let new_built_transfer_lanes = self.built_transfer_lanes
                        .pairs()
                        .map(|(old_ref, id)| {
                            let new_ref = result_delta
                                .transfer_strokes
                                .old_to_new
                                .get(*old_ref)
                                .expect(
                                    "attempted to resurrect a destroyed transfer \
                                                 stroke",
                                );
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

                    requester.built_strokes_changed(built_strokes, world);

                    Some(MaterializedReality {
                        id: self.id,
                        current_plan: new_plan.clone(),
                        current_result: new_result.clone(),
                        built_intersection_lanes: new_built_intersection_lanes,
                        built_trimmed_lanes: new_built_trimmed_lanes,
                        built_transfer_lanes: new_built_transfer_lanes,
                        state: MaterializedRealityState::Ready(()),
                    })
                } else {
                    None
                }
            }
            Ready(_) => panic!("Can't unbuild when materialized reality is in ready state"),
        };
        if let Some(new_self) = maybe_new_self {
            *self = new_self;
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BuildableRef {
    Intersection(usize),
    TrimmedStroke(usize),
    TransferStroke(usize),
}

pub fn setup(system: &mut ActorSystem) -> MaterializedRealityID {
    system.add(Swarm::<MaterializedReality>::new(), |_| {});

    auto_setup(system);

    MaterializedRealityID::spawn(&mut system.world())
}

mod kay_auto;
pub use self::kay_auto::*;
