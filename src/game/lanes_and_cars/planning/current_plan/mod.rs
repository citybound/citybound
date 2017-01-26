use kay::{Actor, Recipient, Fate};
use kay::swarm::{Swarm, CreateWith};
use compact::{CVec, CDict};
use descartes::{V2, N, P2, FiniteCurve};

use super::super::construction::materialized_reality::MaterializedReality;
use super::lane_stroke::LaneStroke;
use super::plan::{PlanDelta, PlanResultDelta, BuiltStrokes, LaneStrokeRef};

mod apply_intent;
use self::apply_intent::apply_intent;
mod rendering;
mod stroke_canvas;
mod selectable;
use self::selectable::Selectable;
mod deselecter;
use self::deselecter::Deselecter;
mod draggable;
use self::draggable::Draggable;

#[derive(Compact, Clone, Default)]
pub struct PlanStep {
    plan_delta: PlanDelta,
    selections: CDict<SelectableStrokeRef, (N, N)>,
    intent: Intent,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum SelectableStrokeRef {
    New(usize),
    Built(LaneStrokeRef),
}

impl SelectableStrokeRef {
    pub fn get_stroke<'a>(&self,
                          plan_delta: &'a PlanDelta,
                          still_built_strokes: &'a BuiltStrokes)
                          -> &'a LaneStroke {
        match *self {
            SelectableStrokeRef::New(node_idx) => &plan_delta.new_strokes[node_idx],
            SelectableStrokeRef::Built(old_ref) => {
                still_built_strokes.mapping
                    .get(old_ref)
                    .unwrap()
            }
        }
    }
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
    ContinueRoad(CVec<(SelectableStrokeRef, ContinuationMode)>, CVec<P2>, P2),
    ContinueRoadAround(SelectableStrokeRef, ContinuationMode, P2),
    Select(SelectableStrokeRef, N, N),
    MaximizeSelection,
    MoveSelection(V2),
    DeleteSelection,
    Deselect,
    CreateNextLane,
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
            let preview = apply_intent(&self.current,
                                       self.still_built_strokes.as_ref(),
                                       &self.settings);
            MaterializedReality::id() <<
            Simulate {
                requester: Self::id(),
                delta: preview.plan_delta.clone(),
            };
            self.preview = Some(preview);
        }
        self.preview.as_ref().unwrap()
    }

    pub fn update_interactables(&mut self) {
        Swarm::<Selectable>::all() << ClearInteractable;
        Swarm::<Draggable>::all() << ClearInteractable;
        Deselecter::id() << ClearInteractable;
        if let Some(ref still_built_strokes) = self.still_built_strokes {
            for (i, stroke) in self.current.plan_delta.new_strokes.iter().enumerate() {
                let selectable = Selectable::new(SelectableStrokeRef::New(i),
                                                 stroke.path().clone());
                Swarm::<Selectable>::id() << CreateWith(selectable, InitInteractable);
            }
            if !self.current.selections.is_empty() {
                Deselecter::id() << InitInteractable;
            }
            for (&selection_ref, &(start, end)) in self.current.selections.pairs() {
                let stroke =
                    selection_ref.get_stroke(&self.current.plan_delta, still_built_strokes);
                if let Some(subsection) = stroke.path().subsection(start, end) {
                    let draggable = Draggable::new(selection_ref, subsection);
                    Swarm::<Draggable>::id() << CreateWith(draggable, InitInteractable);
                }
            }
            self.interactables_valid = true;
        } else {
            MaterializedReality::id() <<
            Simulate {
                requester: Self::id(),
                delta: self.update_preview().plan_delta.clone(),
            }
        }
    }

    fn commit(&mut self) {
        self.undo_history.push(self.current.clone());
        self.current = apply_intent(&self.current,
                                    self.still_built_strokes.as_ref(),
                                    &self.settings);
        self.invalidate_preview();
        self.invalidate_interactables();
    }

    // just the Intent changed, not the PlanDelta or selections
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

#[derive(Copy, Clone)]
pub enum IntentProgress {
    Preview,
    SubStep,
    Finished,
}

#[derive(Compact, Clone)]
pub struct ChangeIntent(pub Intent, pub IntentProgress);

impl Recipient<ChangeIntent> for CurrentPlan {
    fn receive(&mut self, msg: &ChangeIntent) -> Fate {
        match *msg {
            ChangeIntent(ref intent, progress) => {
                self.current.intent = intent.clone();
                match progress {
                    IntentProgress::Preview => {
                        self.invalidate_preview();
                    }
                    IntentProgress::SubStep => {
                        self.commit_substep();
                    }
                    IntentProgress::Finished => {
                        self.commit();
                    }
                }
                Fate::Live
            }
        }
    }
}

use self::stroke_canvas::{Stroke, StrokeState};

impl Recipient<Stroke> for CurrentPlan {
    fn receive(&mut self, msg: &Stroke) -> Fate {
        match *msg {
            Stroke(ref points, state) => {
                let maybe_new_intent = match self.current.intent {
                    Intent::ContinueRoad(ref continue_from, _, start_reference_point) => {
                        Some(Intent::ContinueRoad(continue_from.clone(),
                                                  points.clone(),
                                                  start_reference_point))
                    }
                    _ => {
                        if points.len() >= 2 {
                            Some(Intent::NewRoad(points.clone()))
                        } else {
                            None
                        }
                    }
                };
                if let Some(new_intent) = maybe_new_intent {
                    self.current.intent = new_intent;
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

#[derive(Copy, Clone)]
pub struct InitInteractable;
#[derive(Copy, Clone)]
pub struct ClearInteractable;

pub fn setup() {
    CurrentPlan::register_default();
    CurrentPlan::handle::<Undo>();
    CurrentPlan::handle::<Redo>();
    CurrentPlan::handle::<ChangeIntent>();
    CurrentPlan::handle::<Stroke>();
    CurrentPlan::handle::<SimulationResult>();
    self::rendering::setup();
    self::stroke_canvas::setup();
    self::selectable::setup();
    self::deselecter::setup();
    self::draggable::setup();
}