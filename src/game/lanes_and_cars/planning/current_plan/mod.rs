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
mod addable;
use self::addable::Addable;
mod interaction;
use self::interaction::InteractionSettings;

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
                    .expect("Expected old_ref to exist!")
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
    built_strokes: Option<BuiltStrokes>,
    undo_history: CVec<PlanStep>,
    redo_history: CVec<PlanStep>,
    current: PlanStep,
    preview: Option<PlanStep>,
    preview_result_delta: Option<PlanResultDelta>,
    preview_result_delta_rendered: bool,
    interactables_valid: bool,
    settings: Settings,
    interaction: InteractionSettings,
}
impl Actor for CurrentPlan {}

use super::super::construction::materialized_reality::Simulate;

impl CurrentPlan {
    fn still_built_strokes(&self) -> Option<BuiltStrokes> {
        self.built_strokes.as_ref().map(|built_strokes| {
            BuiltStrokes {
                mapping: built_strokes.mapping
                    .pairs()
                    .filter_map(|(built_ref, stroke)| if self.current
                        .plan_delta
                        .strokes_to_destroy
                        .contains_key(*built_ref) {
                        None
                    } else {
                        Some((*built_ref, stroke.clone()))
                    })
                    .collect(),
            }
        })
    }

    fn invalidate_preview(&mut self) {
        self.preview = None;
    }

    fn invalidate_interactables(&mut self) {
        self.interactables_valid = false;
    }

    pub fn update_preview(&mut self) -> &PlanStep {
        if self.preview.is_none() {
            let preview = apply_intent(&self.current,
                                       self.still_built_strokes().as_ref(),
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
        Swarm::<Addable>::all() << ClearInteractable;
        Deselecter::id() << ClearInteractable;
        if !self.current.selections.is_empty() {
            Deselecter::id() << InitInteractable;
        }
        if let Some(still_built_strokes) = self.still_built_strokes() {
            match self.current.intent {
                Intent::ContinueRoad(..) |
                Intent::NewRoad(..) |
                Intent::ContinueRoadAround(..) => {}
                _ => {
                    for (i, stroke) in self.current.plan_delta.new_strokes.iter().enumerate() {
                        let selectable = Selectable::new(SelectableStrokeRef::New(i),
                                                         stroke.path().clone());
                        Swarm::<Selectable>::id() << CreateWith(selectable, InitInteractable);
                    }
                    for (old_stroke_ref, stroke) in still_built_strokes.mapping.pairs() {
                        let selectable =
                            Selectable::new(SelectableStrokeRef::Built(*old_stroke_ref),
                                            stroke.path().clone());
                        Swarm::<Selectable>::id() << CreateWith(selectable, InitInteractable);
                    }
                }
            }
            for (&selection_ref, &(start, end)) in self.current.selections.pairs() {
                let stroke =
                    selection_ref.get_stroke(&self.current.plan_delta, &still_built_strokes);
                if let Some(subsection) = stroke.path().subsection(start, end) {
                    let draggable = Draggable::new(selection_ref, subsection.clone());
                    Swarm::<Draggable>::id() << CreateWith(draggable, InitInteractable);
                    if let Some(next_lane_path) = subsection.shift_orthogonally(5.0) {
                        let addable = Addable::new(next_lane_path);
                        Swarm::<Addable>::id() << CreateWith(addable, InitInteractable);
                    }
                }
            }
            self.interactables_valid = true;
        } else {
            // TODO: kinda stupid to get initial built strokes like this
            MaterializedReality::id() <<
            Apply {
                requester: Self::id(),
                delta: PlanDelta::default(),
            }
        }
    }

    fn commit(&mut self) {
        self.undo_history.push(self.current.clone());
        self.redo_history.clear();
        self.current = apply_intent(&self.current,
                                    self.still_built_strokes().as_ref(),
                                    &self.settings);
        self.invalidate_preview();
        self.invalidate_interactables();
    }

    // just the Intent changed, not the PlanDelta or selections
    fn commit_substep(&mut self) {
        self.undo_history.push(self.current.clone());
        self.redo_history.clear();
        self.invalidate_preview();
        self.invalidate_interactables();
    }

    // TODO: not really nice that this works differently
    // (needed for proper history)
    fn commit_immediate(&mut self) {
        let mut history_current = self.current.clone();
        history_current.intent = Intent::None;
        self.undo_history.push(history_current);
        self.redo_history.clear();
        self.current = apply_intent(&self.current,
                                    self.still_built_strokes().as_ref(),
                                    &self.settings);
        self.invalidate_preview();
        self.invalidate_interactables();
    }
}

#[derive(Copy, Clone)]
pub struct Undo;
use self::stroke_canvas::{StrokeCanvas, SetPoints};

impl Recipient<Undo> for CurrentPlan {
    fn receive(&mut self, _msg: &Undo) -> Fate {
        let previous_state = self.undo_history.pop().unwrap_or_default();
        self.redo_history.push(self.current.clone());
        self.current = previous_state;
        StrokeCanvas::id() <<
        SetPoints(match self.current.intent {
            Intent::ContinueRoad(_, ref points, _) |
            Intent::NewRoad(ref points) => points.clone(),
            _ => CVec::new(),
        });
        self.invalidate_preview();
        self.invalidate_interactables();
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
            StrokeCanvas::id() <<
            SetPoints(match self.current.intent {
                Intent::ContinueRoad(_, ref points, _) |
                Intent::NewRoad(ref points) => points.clone(),
                _ => CVec::new(),
            });
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
    Immediate,
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
                    IntentProgress::Immediate => {
                        self.commit_immediate();
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
                            self.invalidate_interactables();
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

#[derive(Copy, Clone)]
pub struct SetNLanes(usize);

impl Recipient<SetNLanes> for CurrentPlan {
    fn receive(&mut self, msg: &SetNLanes) -> Fate {
        match *msg {
            SetNLanes(n_lanes) => {
                self.settings.n_lanes_per_side = n_lanes;
                self.invalidate_preview();
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct ToggleBothSides;

impl Recipient<ToggleBothSides> for CurrentPlan {
    fn receive(&mut self, _msg: &ToggleBothSides) -> Fate {
        self.settings.create_both_sides = !self.settings.create_both_sides;
        self.invalidate_preview();
        Fate::Live
    }
}

use super::super::construction::materialized_reality::SimulationResult;

impl Recipient<SimulationResult> for CurrentPlan {
    fn receive(&mut self, msg: &SimulationResult) -> Fate {
        match *msg {
            SimulationResult(ref result_delta) => {
                self.preview_result_delta = Some(result_delta.clone());
                self.preview_result_delta_rendered = false;
                Fate::Live
            }
        }
    }
}

use super::super::construction::materialized_reality::BuiltStrokesChanged;

impl Recipient<BuiltStrokesChanged> for CurrentPlan {
    fn receive(&mut self, msg: &BuiltStrokesChanged) -> Fate {
        match *msg {
            BuiltStrokesChanged(ref built_strokes) => {
                self.built_strokes = Some(built_strokes.clone());
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct Materialize;

use super::super::construction::materialized_reality::Apply;

impl Recipient<Materialize> for CurrentPlan {
    fn receive(&mut self, _msg: &Materialize) -> Fate {
        match self.current.intent {
            Intent::ContinueRoad(..) |
            Intent::NewRoad(..) => {
                self.commit();
                StrokeCanvas::id() << SetPoints(CVec::new());
            }
            _ => {}
        }

        MaterializedReality::id() <<
        Apply {
            requester: Self::id(),
            delta: self.current.plan_delta.clone(),
        };

        *self = CurrentPlan::default();
        Fate::Live
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
    CurrentPlan::handle::<Materialize>();
    CurrentPlan::handle::<SetNLanes>();
    CurrentPlan::handle::<ToggleBothSides>();
    CurrentPlan::handle::<BuiltStrokesChanged>();
    CurrentPlan::handle::<SimulationResult>();
    self::rendering::setup();
    self::stroke_canvas::setup();
    self::selectable::setup();
    self::deselecter::setup();
    self::draggable::setup();
    self::addable::setup();
    self::interaction::setup();
}
