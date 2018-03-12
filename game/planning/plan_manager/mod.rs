use kay::{ActorSystem, World};
use compact::{COption, CVec, CDict};
use stagemaster::UserInterfaceID;
use monet::RendererID;

use super::materialized_reality::MaterializedRealityID;
use super::plan::{PlanDelta, PlanResultDelta};

use transport::transport_planning::plan_manager::{RoadIntent, RoadSelections, RoadPlanSettings,
                                                  MaterializedRoadView};
use transport::transport_planning::plan_manager::apply_intent::apply_road_intent;
use land_use::zone_planning::{ZonePlanDelta, ZonePlanAction};

mod rendering;

mod interaction;
use self::interaction::Interaction;

#[derive(Compact, Clone, Default)]
pub struct PlanStep {
    pub plan_delta: PlanDelta,
    selections: RoadSelections,
    pub intent: Intent,
}

#[derive(Compact, Clone)]
pub enum Intent {
    None,
    RoadIntent(RoadIntent),
    ZoneIntent(ZonePlanAction),
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
pub struct PlanManager {
    id: PlanManagerID,
    materialized_reality: MaterializedRealityID,
    pub materialized_view: MaterializedRoadView,
    undo_history: CVec<PlanStep>,
    redo_history: CVec<PlanStep>,
    pub current: PlanStep,
    preview: COption<PlanStep>,
    preview_rendered_in: CDict<RendererID, ()>,
    preview_result_delta: COption<PlanResultDelta>,
    preview_result_delta_rendered_in: CDict<RendererID, ()>,
    interactables_valid: bool,
    pub settings: RoadPlanSettings,
    interaction: Interaction,
}

impl PlanManager {
    pub fn spawn(
        id: PlanManagerID,
        user_interface: UserInterfaceID,
        renderer_id: RendererID,
        materialized_reality: MaterializedRealityID,
        world: &mut World,
    ) -> PlanManager {
        // TODO: is there a nicer way to get initial built strokes?
        materialized_reality.apply(id, PlanDelta::default(), world);

        PlanManager {
            id: id,
            settings: RoadPlanSettings::default(),
            materialized_reality,
            interaction: Interaction::init(world, user_interface, renderer_id, id),
            materialized_view: MaterializedRoadView::default(),
            undo_history: CVec::new(),
            redo_history: CVec::new(),
            current: PlanStep::default(),
            preview: COption(None),
            preview_rendered_in: CDict::new(),
            preview_result_delta: COption(None),
            preview_result_delta_rendered_in: CDict::new(),
            interactables_valid: false,
        }
    }
}

impl PlanManager {
    pub fn invalidate_preview(&mut self) {
        self.preview = COption(None);
    }

    pub fn update_preview(&mut self, world: &mut World) -> &PlanStep {
        if self.preview.is_none() {
            let preview = self.apply_intent();

            self.materialized_reality.simulate(
                self.id,
                preview.plan_delta.clone(),
                world,
            );
            self.preview = COption(Some(preview));
        }
        self.preview.as_ref().unwrap()
    }

    pub fn commit(&mut self) {
        self.undo_history.push(self.current.clone());
        self.redo_history.clear();
        self.current = self.apply_intent();
        self.invalidate_preview();
        self.invalidate_interactables();
    }

    // just the Intent changed, not the PlanDelta or selections
    pub fn commit_substep(&mut self) {
        self.undo_history.push(self.current.clone());
        self.redo_history.clear();
        self.invalidate_preview();
        self.invalidate_interactables();
    }

    // TODO: not really nice that this works differently
    // (needed for proper history)
    pub fn commit_immediate(&mut self) {
        let mut history_current = self.current.clone();
        history_current.intent = Intent::None;
        self.undo_history.push(history_current);
        self.redo_history.clear();
        self.current = self.apply_intent();
        self.invalidate_preview();
        self.invalidate_interactables();
    }

    pub fn apply_intent(&self) -> PlanStep {
        match self.current.intent {
            Intent::RoadIntent(ref road_intent) => {
                let (new_road_delta, new_road_selections, maybe_new_road_intent) =
                    apply_road_intent(
                        road_intent,
                        &self.current.plan_delta.roads,
                        &self.current.selections,
                        &self.materialized_view,
                        &self.settings,
                    );

                // TODO
                let new_zone_delta = ZonePlanDelta::default();

                PlanStep {
                    plan_delta: PlanDelta {
                        roads: new_road_delta,
                        zones: new_zone_delta,
                    },
                    selections: new_road_selections,
                    intent: maybe_new_road_intent.map(Intent::RoadIntent).unwrap_or(
                        Intent::None,
                    ),
                }
            }
            Intent::ZoneIntent(ref zone_plan_action) => {
                let mut new_zone_delta = self.current.plan_delta.zones.clone();
                new_zone_delta.actions.push(zone_plan_action.clone());

                PlanStep {
                    plan_delta: PlanDelta {
                        roads: self.current.plan_delta.roads.clone(),
                        zones: new_zone_delta,
                    },
                    selections: RoadSelections::default(),
                    intent: Intent::None,
                }
            }
            _ => self.current.clone(),
        }
    }
}

impl PlanManager {
    pub fn undo(&mut self, world: &mut World) {
        let previous_state = self.undo_history.pop().unwrap_or_default();
        self.redo_history.push(self.current.clone());
        self.current = previous_state;
        self.invalidate_preview();
        self.invalidate_interactables();
        self.on_step(world);
    }

    pub fn redo(&mut self, world: &mut World) {
        if let Some(next_state) = self.redo_history.pop() {
            self.undo_history.push(self.current.clone());
            self.current = next_state;
            self.invalidate_preview();
            self.invalidate_interactables();
            self.on_step(world);
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

    pub fn on_simulation_result(&mut self, result_delta: &PlanResultDelta, _: &mut World) {
        self.preview_result_delta = COption(Some(result_delta.clone()));
        self.preview_result_delta_rendered_in = CDict::new();
    }

    pub fn materialized_reality_changed(
        &mut self,
        new_materialized_view: &MaterializedRoadView,
        _: &mut World,
    ) {
        self.materialized_view = new_materialized_view.clone();
    }
}

impl PlanManager {
    pub fn materialize(&mut self, world: &mut World) {
        let commit_before_materialize = match self.current.intent {
            Intent::RoadIntent(ref road_intent) => road_intent.commit_before_materialize(),
            _ => false,
        };

        if commit_before_materialize {
            self.commit();
            self.on_step(world);
        }

        self.materialized_reality.apply(
            self.id,
            self.current.plan_delta.clone(),
            world,
        );

        *self = PlanManager {
            id: self.id,
            materialized_reality: self.materialized_reality,
            settings: self.settings.clone(),
            interaction: self.interaction.clone(),
            materialized_view: MaterializedRoadView::default(),
            undo_history: CVec::new(),
            redo_history: CVec::new(),
            current: PlanStep::default(),
            preview: COption(None),
            preview_rendered_in: CDict::new(),
            preview_result_delta: COption(None),
            preview_result_delta_rendered_in: CDict::new(),
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
    system.register::<PlanManager>();
    auto_setup(system);
    interaction::auto_setup(system);
    rendering::auto_setup(system);

    PlanManagerID::spawn(
        user_interface,
        renderer_id,
        materialized_reality,
        &mut system.world(),
    );
}

mod kay_auto;
pub use self::kay_auto::*;
