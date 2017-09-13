use kay::{ActorSystem, Fate, World};
use kay::swarm::Swarm;
use compact::{COption, CVec, CDict};
use descartes::{V2, N, P2, FiniteCurve};
use stagemaster::UserInterfaceID;
use monet::RendererID;

use super::super::construction::materialized_reality::MaterializedRealityID;
use super::lane_stroke::LaneStroke;
use super::plan::{PlanDelta, PlanResultDelta, BuiltStrokes, LaneStrokeRef};

mod apply_intent;
use self::apply_intent::apply_intent;
mod rendering;

mod helper_interactables;
use self::helper_interactables::{DeselecterID, AddableID, DraggableID, SelectableID,
                                 StrokeCanvasID, StrokeState};

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

#[derive(Copy, Clone)]
pub enum IntentProgress {
    Preview,
    SubStep,
    Finished,
    Immediate,
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

#[derive(Compact, Clone)]
pub struct CurrentPlan {
    id: CurrentPlanID,
    materialized_reality: MaterializedRealityID,
    built_strokes: COption<BuiltStrokes>,
    undo_history: CVec<PlanStep>,
    redo_history: CVec<PlanStep>,
    current: PlanStep,
    preview: COption<PlanStep>,
    preview_result_delta: COption<PlanResultDelta>,
    preview_result_delta_rendered: bool,
    interactables_valid: bool,
    settings: Settings,
    interaction: Interaction,
}

impl CurrentPlan {
    pub fn spawn(
        id: CurrentPlanID,
        user_interface: UserInterfaceID,
        renderer_id: RendererID,
        materialized_reality: MaterializedRealityID,
        world: &mut World,
    ) -> CurrentPlan {
        // TODO: is there a nicer way to get initial built strokes?
        materialized_reality.apply(id, PlanDelta::default(), world);

        StrokeCanvasID::spawn(user_interface, id, world);

        CurrentPlan {
            id: id,
            settings: Settings::default(),
            materialized_reality,
            interaction: Interaction::init(world, user_interface, renderer_id, id),
            built_strokes: COption(None),
            undo_history: CVec::new(),
            redo_history: CVec::new(),
            current: PlanStep::default(),
            preview: COption(None),
            preview_result_delta: COption(None),
            preview_result_delta_rendered: false,
            interactables_valid: false,
        }
    }
}

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
        self.preview = COption(None);
    }

    fn invalidate_interactables(&mut self) {
        self.interactables_valid = false;
    }

    fn update_preview(&mut self, world: &mut World) -> &PlanStep {
        if self.preview.is_none() {
            let preview = apply_intent(
                &self.current,
                self.still_built_strokes().as_ref(),
                &self.settings,
            );
            self.materialized_reality.simulate(
                self.id,
                preview.plan_delta.clone(),
                world,
            );
            self.preview = COption(Some(preview));
        }
        self.preview.as_ref().unwrap()
    }

    fn update_interactables(&mut self, world: &mut World) {
        SelectableID::broadcast(world).clear(self.interaction.user_interface, world);
        DraggableID::broadcast(world).clear(self.interaction.user_interface, world);
        AddableID::broadcast(world).clear(self.interaction.user_interface, world);
        // TODO: ugly/wrong
        DeselecterID::broadcast(world).clear(self.interaction.user_interface, world);

        if !self.current.selections.is_empty() {
            DeselecterID::spawn(self.interaction.user_interface, self.id, world);
        }
        if let Some(still_built_strokes) = self.still_built_strokes() {
            match self.current.intent {
                Intent::ContinueRoad(..) |
                Intent::NewRoad(..) |
                Intent::ContinueRoadAround(..) => {}
                _ => {
                    for (i, stroke) in self.current.plan_delta.new_strokes.iter().enumerate() {
                        SelectableID::spawn(
                            SelectableStrokeRef::New(i),
                            stroke.path().clone(),
                            self.interaction.user_interface,
                            self.id,
                            world,
                        );
                    }
                    for (old_stroke_ref, stroke) in still_built_strokes.mapping.pairs() {
                        SelectableID::spawn(
                            SelectableStrokeRef::Built(*old_stroke_ref),
                            stroke.path().clone(),
                            self.interaction.user_interface,
                            self.id,
                            world,
                        );
                    }
                }
            }
            for (&selection_ref, &(start, end)) in self.current.selections.pairs() {
                let stroke =
                    selection_ref.get_stroke(&self.current.plan_delta, &still_built_strokes);
                if let Some(subsection) = stroke.path().subsection(start, end) {
                    DraggableID::spawn(
                        selection_ref,
                        subsection.clone(),
                        self.interaction.user_interface,
                        self.id,
                        world,
                    );
                    if let Some(next_lane_path) = subsection.shift_orthogonally(5.0) {
                        AddableID::spawn(
                            next_lane_path,
                            self.interaction.user_interface,
                            self.id,
                            world,
                        );
                    }
                }
            }
            self.interactables_valid = true;
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

impl CurrentPlan {
    pub fn undo(&mut self, world: &mut World) {
        let previous_state = self.undo_history.pop().unwrap_or_default();
        self.redo_history.push(self.current.clone());
        self.current = previous_state;
        // TODO: ugly/wrong
        StrokeCanvasID::broadcast(world).set_points(
            match self.current.intent {
                Intent::ContinueRoad(_, ref points, _) |
                Intent::NewRoad(ref points) => points.clone(),
                _ => CVec::new(),
            },
            world,
        );
        self.invalidate_preview();
        self.invalidate_interactables();
    }

    pub fn redo(&mut self, world: &mut World) {
        if let Some(next_state) = self.redo_history.pop() {
            self.undo_history.push(self.current.clone());
            self.current = next_state;
            // TODO: ugly/wrong
            StrokeCanvasID::broadcast(world).set_points(
                match self.current.intent {
                    Intent::ContinueRoad(_, ref points, _) |
                    Intent::NewRoad(ref points) => points.clone(),
                    _ => CVec::new(),
                },
                world,
            );
            self.invalidate_preview();
            self.invalidate_interactables();
        }
    }

    pub fn change_intent(&mut self, intent: &Intent, progress: IntentProgress, _: &mut World) {
        self.current.intent = intent.clone();
        match progress {
            IntentProgress::Preview => self.invalidate_preview(),
            IntentProgress::SubStep => self.commit_substep(),
            IntentProgress::Finished => self.commit(),
            IntentProgress::Immediate => self.commit_immediate(),
        }
    }

    pub fn on_stroke(&mut self, points: &CVec<P2>, state: StrokeState, _: &mut World) {
        let maybe_new_intent = match self.current.intent {
            Intent::ContinueRoad(ref continue_from, _, start_reference_point) => {
                Some(Intent::ContinueRoad(
                    continue_from.clone(),
                    points.clone(),
                    start_reference_point,
                ))
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
    }

    pub fn set_n_lanes(&mut self, n_lanes: usize, _: &mut World) {
        self.settings.n_lanes_per_side = n_lanes;
        self.invalidate_preview();
    }

    pub fn toggle_both_sides(&mut self, _: &mut World) {
        self.settings.create_both_sides = !self.settings.create_both_sides;
        self.invalidate_preview();
    }

    pub fn on_simulation_result(&mut self, result_delta: &PlanResultDelta, _: &mut World) {
        self.preview_result_delta = COption(Some(result_delta.clone()));
        self.preview_result_delta_rendered = false;
    }

    pub fn built_strokes_changed(&mut self, built_strokes: &BuiltStrokes, _: &mut World) {
        self.built_strokes = COption(Some(built_strokes.clone()));
    }
}

impl CurrentPlan {
    pub fn materialize(&mut self, world: &mut World) {
        match self.current.intent {
            Intent::ContinueRoad(..) |
            Intent::NewRoad(..) => {
                self.commit();
                // TODO: ugly/wrong
                StrokeCanvasID::broadcast(world).set_points(CVec::new(), world);
            }
            _ => {}
        }

        self.materialized_reality.apply(
            self.id,
            self.current.plan_delta.clone(),
            world,
        );

        *self = CurrentPlan {
            id: self.id,
            materialized_reality: self.materialized_reality,
            settings: self.settings.clone(),
            interaction: self.interaction.clone(),
            built_strokes: COption(None),
            undo_history: CVec::new(),
            redo_history: CVec::new(),
            current: PlanStep::default(),
            preview: COption(None),
            preview_result_delta: COption(None),
            preview_result_delta_rendered: false,
            interactables_valid: false,
        };
    }
}

pub fn setup(
    system: &mut ActorSystem,
    user_interface: UserInterfaceID,
    renderer_id: RendererID,
    materialized_reality: MaterializedRealityID,
) {
    system.add(Swarm::<CurrentPlan>::new(), |_| {});
    auto_setup(system);
    helper_interactables::setup(system);
    interaction::auto_setup(system);
    rendering::auto_setup(system);

    CurrentPlanID::spawn(
        user_interface,
        renderer_id,
        materialized_reality,
        &mut system.world(),
    );
}

mod kay_auto;
pub use self::kay_auto::*;