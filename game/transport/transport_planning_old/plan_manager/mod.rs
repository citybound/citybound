use kay::ActorSystem;
use compact::{CDict, CVec};
use descartes::{N, P2, V2};
use planning_old::plan_manager::PlanManager;
use super::lane_stroke::LaneStroke;
use super::road_plan::{LaneStrokeRef, RoadPlanDelta};
use super::materialized_roads::BuiltStrokes;

pub mod apply_intent;
pub mod interaction;
pub mod rendering;
pub mod helper_interactables;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum SelectableStrokeRef {
    New(usize),
    Built(LaneStrokeRef),
}

impl SelectableStrokeRef {
    pub fn get_stroke<'a>(
        &self,
        plan_delta: &'a RoadPlanDelta,
        still_built_strokes: &'a BuiltStrokes,
    ) -> &'a LaneStroke {
        match *self {
            SelectableStrokeRef::New(node_idx) => &plan_delta.new_strokes[node_idx],
            SelectableStrokeRef::Built(old_ref) => {
                still_built_strokes.mapping.get(old_ref).expect(
                    "Expected old_ref to exist!",
                )
            }
        }
    }
}

pub type RoadSelections = CDict<SelectableStrokeRef, (N, N)>;

#[derive(Copy, Clone)]
pub enum ContinuationMode {
    Append,
    Prepend,
}

#[derive(Compact, Clone)]
pub enum RoadIntent {
    NewRoad(CVec<P2>),
    ContinueRoad(CVec<(SelectableStrokeRef, ContinuationMode)>, CVec<P2>, P2),
    ContinueRoadAround(SelectableStrokeRef, ContinuationMode, P2),
    Select(SelectableStrokeRef, N, N),
    MaximizeSelection,
    MoveSelection(V2),
    DeleteSelection,
    Deselect,
    CreateNextLane,
}

impl RoadIntent {
    pub fn commit_before_materialize(&self) -> bool {
        match *self {
            RoadIntent::ContinueRoad(..) |
            RoadIntent::NewRoad(..) => true,
            _ => false,
        }
    }
}

#[derive(Compact, Clone)]
pub struct RoadPlanSettings {
    n_lanes_per_side: usize,
    create_both_sides: bool,
    select_parallel: bool,
    select_opposite: bool,
}

impl Default for RoadPlanSettings {
    fn default() -> Self {
        RoadPlanSettings {
            create_both_sides: true,
            n_lanes_per_side: 2,
            select_parallel: true,
            select_opposite: true,
        }
    }
}

#[derive(Compact, Clone, Default)]
pub struct MaterializedRoadView {
    pub built_strokes: BuiltStrokes,
}

impl PlanManager {
    pub fn built_strokes_after_delta(&self) -> BuiltStrokes {
        BuiltStrokes {
            mapping: self.materialized_view
                .built_strokes
                .mapping
                .pairs()
                .filter_map(|(built_ref, stroke)| if self.current
                    .plan_delta
                    .roads
                    .strokes_to_destroy
                    .contains_key(*built_ref)
                {
                    None
                } else {
                    Some((*built_ref, stroke.clone()))
                })
                .collect(),
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    interaction::auto_setup(system);
    helper_interactables::setup(system);
}
