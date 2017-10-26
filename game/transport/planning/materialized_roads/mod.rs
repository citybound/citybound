use kay::World;
use compact::{CVec, CDict};
use planning::materialized_reality::{MaterializedReality, MaterializedRealityID};
use planning::materialized_reality::MaterializedRealityState::{Ready, Updating};
use super::road_plan::{RoadPlan, RoadPlanResultDelta, LaneStrokeRef, IntersectionRef,
                       TrimmedStrokeRef, TransferStrokeRef};
use super::lane_stroke::LaneStroke;
use super::plan_manager::MaterializedRoadView;
use super::super::lane::LaneID;
use super::super::microtraffic::LaneLikeID;
use super::super::construction::UnbuildableID;

#[derive(Compact, Clone, Default)]
pub struct BuiltStrokes {
    pub mapping: CDict<LaneStrokeRef, LaneStroke>,
}

#[derive(Compact, Clone, Default)]
pub struct MaterializedRoads {
    built_intersection_lanes: CDict<IntersectionRef, CVec<LaneID>>,
    pub built_trimmed_lanes: CDict<TrimmedStrokeRef, LaneLikeID>,
    built_transfer_lanes: CDict<TransferStrokeRef, LaneLikeID>,
    built_strokes: BuiltStrokes,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BuildableRef {
    Intersection(usize),
    TrimmedStroke(usize),
    TransferStroke(usize),
}

#[derive(Compact, Clone)]
#[allow(large_enum_variant)]
pub struct RoadUpdateState {
    lanes_to_unbuild: CVec<LaneLikeID>,
}

impl RoadUpdateState {
    pub fn done(&self) -> bool {
        self.lanes_to_unbuild.is_empty()
    }
}

impl MaterializedRoads {
    pub fn start_applying_roads(
        materialized_reality: MaterializedRealityID,
        materialized_roads: &mut MaterializedRoads,
        result_delta: &RoadPlanResultDelta,
        world: &mut World,
    ) -> RoadUpdateState {
        let mut lanes_to_unbuild = CVec::new();

        for old_ref in result_delta.intersections.to_destroy.keys() {
            for id in materialized_roads.built_intersection_lanes.remove_iter(
                *old_ref,
            )
            {
                lanes_to_unbuild.push(id.into());
            }
        }

        for old_ref in result_delta.trimmed_strokes.to_destroy.keys() {
            let id = materialized_roads
                .built_trimmed_lanes
                .remove(*old_ref)
                .expect("tried to unbuild a non-existing lane");
            lanes_to_unbuild.push(id);
        }

        for old_ref in result_delta.transfer_strokes.to_destroy.keys() {
            let id = materialized_roads
                .built_transfer_lanes
                .remove(*old_ref)
                .expect("tried to unbuild a non-existing transfer lane");
            lanes_to_unbuild.push(id);
        }

        for &id in &lanes_to_unbuild {
            // TODO: ugly: untyped ID shenanigans
            let id_as_unbuildable: UnbuildableID = UnbuildableID { _raw_id: id._raw_id };
            id_as_unbuildable.unbuild(materialized_reality, world);
        }

        RoadUpdateState { lanes_to_unbuild }
    }

    pub fn finish_applying_roads(
        materialized_reality: MaterializedRealityID,
        materialized_roads: &MaterializedRoads,
        new_plan: &RoadPlan,
        result_delta: &RoadPlanResultDelta,
        world: &mut World,
    ) -> (MaterializedRoads, MaterializedRoadView) {
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
                    materialized_reality,
                    BuildableRef::Intersection(new_index),
                    timings.clone(),
                    world,
                );
            }
        }

        for (&TrimmedStrokeRef(new_index), new_stroke) in
            result_delta.trimmed_strokes.to_create.pairs()
        {
            new_stroke.build(
                materialized_reality,
                BuildableRef::TrimmedStroke(new_index),
                world,
            );
        }

        for (&TransferStrokeRef(new_index), new_stroke) in
            result_delta.transfer_strokes.to_create.pairs()
        {
            new_stroke.build_transfer(
                materialized_reality,
                BuildableRef::TransferStroke(new_index),
                world,
            );
        }

        let new_built_intersection_lanes = materialized_roads
            .built_intersection_lanes
            .pairs()
            .map(|(old_ref, ids)| {
                let new_ref = result_delta.intersections.old_to_new.get(*old_ref).expect(
                    "attempted to resurrect a destroyed intersection",
                );
                (*new_ref, ids.clone())
            })
            .collect();

        let new_built_trimmed_lanes = materialized_roads
            .built_trimmed_lanes
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

        let new_built_transfer_lanes = materialized_roads
            .built_transfer_lanes
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

        let new_built_strokes = BuiltStrokes {
            mapping: new_plan
                .strokes
                .iter()
                .enumerate()
                .map(|(idx, stroke)| (LaneStrokeRef(idx), stroke.clone()))
                .collect(),
        };

        (
            MaterializedRoads {
                built_intersection_lanes: new_built_intersection_lanes,
                built_trimmed_lanes: new_built_trimmed_lanes,
                built_transfer_lanes: new_built_transfer_lanes,
                built_strokes: new_built_strokes.clone(),
            },
            MaterializedRoadView { built_strokes: new_built_strokes },
        )
    }
}

impl MaterializedReality {
    pub fn on_lane_built(
        &mut self,
        id: LaneLikeID,
        buildable_ref: BuildableRef,
        world: &mut World,
    ) {
        match self.state {
            Ready(()) => {
                    match buildable_ref {
                        BuildableRef::Intersection(index) => {
                            // TODO: ugly: raw ID shenanigans
                            let id_as_lane: LaneID = LaneID{ _raw_id: id._raw_id};
                            if let Some(other_intersection_lanes) =
                                self.roads.built_intersection_lanes.get(IntersectionRef(index))
                            {
                                id_as_lane.start_connecting_overlaps(
                                    other_intersection_lanes.clone(),
                                    world
                                );
                            }
                            self.roads.built_intersection_lanes.push_at(
                                IntersectionRef(index),
                                id_as_lane,
                            );
                        }
                        BuildableRef::TrimmedStroke(index) => {
                            self.roads.built_trimmed_lanes.insert(
                                TrimmedStrokeRef(index),
                                id,
                            );
                        }
                        BuildableRef::TransferStroke(index) => {
                            self.roads.built_transfer_lanes.insert(
                                TransferStrokeRef(index),
                                id,
                            );
                        }
                    }
                }
            Updating(..) => panic!("a waiting materialized reality shouldn't get build reports"),
        }
    }

    pub fn on_lane_unbuilt(&mut self, id: LaneLikeID, world: &mut World) {
        match self.state {
            Updating(_, _, _, _, RoadUpdateState { ref mut lanes_to_unbuild }) => {

                let pos = lanes_to_unbuild
                    .iter()
                    .position(|unbuild_id| *unbuild_id == id)
                    .expect("Trying to delete unexpected id");

                lanes_to_unbuild.remove(pos);
            }
            Ready(_) => panic!("Can't unbuild when materialized reality is in ready state"),
        };

        self.check_if_done(world);
    }
}

mod kay_auto;
pub use self::kay_auto::*;