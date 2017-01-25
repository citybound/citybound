use kay::{Actor, Recipient, Fate};
use compact::{CVec, CDict};
use descartes::P2;

use super::super::construction::materialized_reality::MaterializedReality;
use super::plan::{PlanDelta, PlanResultDelta, BuiltStrokes, LaneStrokeRef};
use super::lane_stroke::{LaneStroke, LaneStrokeNode, LaneStrokeNodeRef};

mod apply_intent;
use self::apply_intent::apply_intent;
mod rendering;
mod stroke_canvas;

#[derive(Compact, Clone, Default)]
pub struct PlanStep {
    plan_delta: PlanDelta,
    selections: CVec<()>,
    intent: Intent,
}

#[derive(Copy, Clone)]
pub enum ContinuationMode {
    Append,
    Prepend,
}

#[derive(Compact, Clone)]
pub enum Intent {
    None,
    NewRoad(CVec<P2>),
    ContinueRoad(CVec<(LaneStrokeRef, ContinuationMode)>, CVec<P2>, P2),
}

impl Default for Intent {
    fn default() -> Self {
        Intent::None
    }
}

#[derive(Compact, Clone)]
pub struct Settings {
    n_lanes_per_side: usize,
    create_both_sides: bool,
    select_parallel: bool,
    select_opposite: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            create_both_sides: true,
            n_lanes_per_side: 2,
            select_parallel: true,
            select_opposite: true,
        }
    }
}

#[derive(Default)]
pub struct CurrentPlan {
    still_built_strokes: Option<BuiltStrokes>,
    undo_history: CVec<PlanStep>,
    redo_history: CVec<PlanStep>,
    current: PlanStep,
    preview: Option<PlanStep>,
    preview_result_delta: Option<PlanResultDelta>,
    preview_result_delta_rendered: bool,
    interactables_valid: bool,
    settings: Settings,
}
impl Actor for CurrentPlan {}

use super::super::construction::materialized_reality::Simulate;

impl CurrentPlan {
    fn invalidate_preview(&mut self) {
        self.preview = None;
    }

    fn invalidate_interactables(&mut self) {
        self.interactables_valid = false;
    }

    pub fn update_preview(&mut self) -> &PlanStep {
        if self.preview.is_none() {
            let preview = apply_intent(&self.current, &self.settings);
            MaterializedReality::id() <<
            Simulate {
                requester: Self::id(),
                delta: preview.plan_delta.clone(),
            };
            self.preview = Some(preview);
        }
        self.preview.as_ref().unwrap()
    }

    fn commit(&mut self) {
        self.undo_history.push(self.current.clone());
        self.current = apply_intent(&self.current, &self.settings);
        self.invalidate_preview();
        self.invalidate_interactables();
    }

    // just the Intent or selections changed, not the underlying PlanDelta
    fn commit_substep(&mut self) {
        self.undo_history.push(self.current.clone());
        self.invalidate_preview();
        self.invalidate_interactables();
    }
}

#[derive(Copy, Clone)]
pub struct Undo;

impl Recipient<Undo> for CurrentPlan {
    fn receive(&mut self, _msg: &Undo) -> Fate {
        if let Some(previous_state) = self.undo_history.pop() {
            self.redo_history.push(self.current.clone());
            self.current = previous_state;
            self.invalidate_preview();
            self.invalidate_interactables();
        }
        Fate::Live
    }
}

#[derive(Copy, Clone)]
pub struct Redo;

impl Recipient<Redo> for CurrentPlan {
    fn receive(&mut self, _msg: &Redo) -> Fate {
        if let Some(next_state) = self.redo_history.pop() {
            self.undo_history.push(self.current.clone());
            self.current = next_state;
            self.invalidate_preview();
            self.invalidate_interactables();
        }
        Fate::Live
    }
}

use self::stroke_canvas::Stroke;
use self::stroke_canvas::StrokeState;

impl Recipient<Stroke> for CurrentPlan {
    fn receive(&mut self, msg: &Stroke) -> Fate {
        match *msg {
            Stroke(ref points, state) => {
                self.current.intent = Intent::NewRoad(points.clone());
                match state {
                    StrokeState::Preview => {
                        self.invalidate_preview();
                    }
                    StrokeState::Intermediate => {
                        self.commit_substep();
                    }
                    StrokeState::Finished => {
                        self.commit();
                    }
                }
                Fate::Live
            }
        }
    }
}

use super::super::construction::materialized_reality::SimulationResult;

impl Recipient<SimulationResult> for CurrentPlan {
    fn receive(&mut self, msg: &SimulationResult) -> Fate {
        match *msg {
            SimulationResult { ref result_delta, ref still_built_strokes } => {
                self.preview_result_delta = Some(result_delta.clone());
                self.preview_result_delta_rendered = false;
                self.still_built_strokes = Some(still_built_strokes.clone());
                Fate::Live
            }
        }
    }
}

pub fn setup() {
    CurrentPlan::register_default();
    CurrentPlan::handle::<Undo>();
    CurrentPlan::handle::<Redo>();
    CurrentPlan::handle::<Stroke>();
    CurrentPlan::handle::<SimulationResult>();
    self::rendering::setup();
    self::stroke_canvas::setup();
}