use kay::{ActorSystem, Fate, World};
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
use self::interaction::Interaction;

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
    pub fn get_stroke<'a>(
        &self,
        plan_delta: &'a PlanDelta,
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
    interaction: Interaction,
}

use super::super::construction::materialized_reality::Simulate;

impl CurrentPlan {
    fn still_built_strokes(&self) -> Option<BuiltStrokes> {
        self.built_strokes.as_ref().map(|built_strokes| {
            BuiltStrokes {
                mapping: built_strokes
                    .mapping
                    .pairs()
                    .filter_map(|(built_ref, stroke)| if self.current
                        .plan_delta
                        .strokes_to_destroy
                        .contains_key(*built_ref)
                    {
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

    pub fn update_preview(&mut self, world: &mut World) -> &PlanStep {
        if self.preview.is_none() {
            let preview = apply_intent(
                &self.current,
                self.still_built_strokes().as_ref(),
                &self.settings,
            );
            let plan_id = world.id::<Self>();
            world.send_to_id_of::<MaterializedReality, _>(Simulate {
                requester: plan_id,
                delta: preview.plan_delta.clone(),
            });
            self.preview = Some(preview);
        }
        self.preview.as_ref().unwrap()
    }

    pub fn update_interactables(&mut self, world: &mut World) {
        world.broadcast_to_id_of::<Swarm<Selectable>, _>(ClearInteractable);
        world.broadcast_to_id_of::<Swarm<Draggable>, _>(ClearInteractable);
        world.broadcast_to_id_of::<Swarm<Addable>, _>(ClearInteractable);
        world.send_to_id_of::<Deselecter, _>(ClearInteractable);
        if !self.current.selections.is_empty() {
            world.send_to_id_of::<Deselecter, _>(InitInteractable);
        }
        if let Some(still_built_strokes) = self.still_built_strokes() {
            match self.current.intent {
                Intent::ContinueRoad(..) |
                Intent::NewRoad(..) |
                Intent::ContinueRoadAround(..) => {}
                _ => {
                    for (i, stroke) in self.current.plan_delta.new_strokes.iter().enumerate() {
                        let selectable =
                            Selectable::new(SelectableStrokeRef::New(i), stroke.path().clone());
                        world.send_to_id_of::<Swarm<Selectable>, _>(
                            CreateWith(selectable, InitInteractable),
                        );
                    }
                    for (old_stroke_ref, stroke) in still_built_strokes.mapping.pairs() {
                        let selectable = Selectable::new(
                            SelectableStrokeRef::Built(*old_stroke_ref),
                            stroke.path().clone(),
                        );
                        world.send_to_id_of::<Swarm<Selectable>, _>(
                            CreateWith(selectable, InitInteractable),
                        );
                    }
                }
            }
            for (&selection_ref, &(start, end)) in self.current.selections.pairs() {
                let stroke =
                    selection_ref.get_stroke(&self.current.plan_delta, &still_built_strokes);
                if let Some(subsection) = stroke.path().subsection(start, end) {
                    let draggable = Draggable::new(selection_ref, subsection.clone());
                    world.send_to_id_of::<Swarm<Draggable>, _>(
                        CreateWith(draggable, InitInteractable),
                    );
                    if let Some(next_lane_path) = subsection.shift_orthogonally(5.0) {
                        let addable = Addable::new(next_lane_path);
                        world.send_to_id_of::<Swarm<Addable>, _>(
                            CreateWith(addable, InitInteractable),
                        );
                    }
                }
            }
            self.interactables_valid = true;
        } else {
            // TODO: stupid to get initial built strokes like this -> move to future constructor!!!!
            let plan_id = world.id::<Self>();
            world.send_to_id_of::<MaterializedReality, _>(Apply {
                requester: plan_id,
                delta: PlanDelta::default(),
            })
        }
    }

    fn commit(&mut self) {
        self.undo_history.push(self.current.clone());
        self.redo_history.clear();
        self.current = apply_intent(
            &self.current,
            self.still_built_strokes().as_ref(),
            &self.settings,
        );
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
        self.current = apply_intent(
            &self.current,
            self.still_built_strokes().as_ref(),
            &self.settings,
        );
        self.invalidate_preview();
        self.invalidate_interactables();
    }
}

#[derive(Copy, Clone)]
pub struct Undo;
use self::stroke_canvas::{StrokeCanvas, SetPoints};


pub fn setup(system: &mut ActorSystem) {
    system.add(CurrentPlan::default(), |mut the_cp| {
        let current_plan_id = the_cp.world().id::<CurrentPlan>();
        let canvas_id = the_cp.world().id::<StrokeCanvas>();
        let mr_id = the_cp.world().id::<MaterializedReality>();

        the_cp.on(move |_: &Undo, plan, world| {
            let previous_state = plan.undo_history.pop().unwrap_or_default();
            plan.redo_history.push(plan.current.clone());
            plan.current = previous_state;
            world.send(
                canvas_id,
                SetPoints(match plan.current.intent {
                    Intent::ContinueRoad(_, ref points, _) |
                    Intent::NewRoad(ref points) => points.clone(),
                    _ => CVec::new(),
                }),
            );
            plan.invalidate_preview();
            plan.invalidate_interactables();
            Fate::Live
        });

        the_cp.on(move |_: &Redo, plan, world| {
            if let Some(next_state) = plan.redo_history.pop() {
                plan.undo_history.push(plan.current.clone());
                plan.current = next_state;
                world.send(
                    canvas_id,
                    SetPoints(match plan.current.intent {
                        Intent::ContinueRoad(_, ref points, _) |
                        Intent::NewRoad(ref points) => points.clone(),
                        _ => CVec::new(),
                    }),
                );
                plan.invalidate_preview();
                plan.invalidate_interactables();
            }
            Fate::Live
        });

        the_cp.on(|&ChangeIntent(ref intent, progress), plan, _| {
            plan.current.intent = intent.clone();
            match progress {
                IntentProgress::Preview => plan.invalidate_preview(),
                IntentProgress::SubStep => plan.commit_substep(),
                IntentProgress::Finished => plan.commit(),
                IntentProgress::Immediate => plan.commit_immediate(),
            }
            Fate::Live
        });

        the_cp.on(|&Stroke(ref points, state), plan, _| {
            let maybe_new_intent = match plan.current.intent {
                Intent::ContinueRoad(ref continue_from, _, start_reference_point) => {
                    Some(Intent::ContinueRoad(
                        continue_from.clone(),
                        points.clone(),
                        start_reference_point,
                    ))
                }
                _ => {
                    if points.len() >= 2 {
                        plan.invalidate_interactables();
                        Some(Intent::NewRoad(points.clone()))
                    } else {
                        None
                    }
                }
            };
            if let Some(new_intent) = maybe_new_intent {
                plan.current.intent = new_intent;
                match state {
                    StrokeState::Preview => {
                        plan.invalidate_preview();
                    }
                    StrokeState::Intermediate => {
                        plan.commit_substep();
                    }
                    StrokeState::Finished => {
                        plan.commit();
                    }
                }

            }
            Fate::Live
        });

        the_cp.on(|&SetNLanes(n_lanes), plan, _| {
            plan.settings.n_lanes_per_side = n_lanes;
            plan.invalidate_preview();
            Fate::Live
        });

        the_cp.on(|_: &ToggleBothSides, plan, _| {
            plan.settings.create_both_sides = !plan.settings.create_both_sides;
            plan.invalidate_preview();
            Fate::Live
        });

        the_cp.on(|&SimulationResult(ref result_delta), plan, _| {
            plan.preview_result_delta = Some(result_delta.clone());
            plan.preview_result_delta_rendered = false;
            Fate::Live
        });

        the_cp.on(|&BuiltStrokesChanged(ref built_strokes), plan, _| {
            plan.built_strokes = Some(built_strokes.clone());
            Fate::Live
        });

        the_cp.on(move |_: &Materialize, plan, world| {
            match plan.current.intent {
                Intent::ContinueRoad(..) |
                Intent::NewRoad(..) => {
                    plan.commit();
                    world.send(canvas_id, SetPoints(CVec::new()));
                }
                _ => {}
            }

            world.send(
                mr_id,
                Apply {
                    requester: current_plan_id,
                    delta: plan.current.plan_delta.clone(),
                },
            );

            *plan = CurrentPlan {
                settings: plan.settings.clone(),
                interaction: plan.interaction.clone(),
                ..CurrentPlan::default()
            };

            Fate::Live
        })
    });
    self::rendering::setup(system);
    self::stroke_canvas::setup(system);
    self::selectable::setup(system);
    self::deselecter::setup(system);
    self::draggable::setup(system);
    self::addable::setup(system);
    self::interaction::setup(system);
}

#[derive(Copy, Clone)]
pub struct Redo;

#[derive(Copy, Clone)]
pub enum IntentProgress {
    Preview,
    SubStep,
    Finished,
    Immediate,
}

#[derive(Compact, Clone)]
pub struct ChangeIntent(pub Intent, pub IntentProgress);

use self::stroke_canvas::{Stroke, StrokeState};


#[derive(Copy, Clone)]
pub struct SetNLanes(usize);

#[derive(Copy, Clone)]
pub struct ToggleBothSides;

use super::super::construction::materialized_reality::SimulationResult;
use super::super::construction::materialized_reality::BuiltStrokesChanged;

#[derive(Copy, Clone)]
pub struct Materialize;

use super::super::construction::materialized_reality::Apply;

#[derive(Copy, Clone)]
pub struct InitInteractable;
#[derive(Copy, Clone)]
pub struct ClearInteractable;
