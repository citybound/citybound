use kay::{ActorSystem, External, World, Actor};
use super::{PlanManager, PlanManagerID, Intent};
use descartes::P2;
use stagemaster::combo::{Bindings, Combo2};
use stagemaster::geometry::AnyShape;
use transport::lane::Lane;
use transport::planning::plan_manager::interaction::{RoadInteraction,
                                                     default_road_planning_bindings};

#[derive(Compact, Clone)]
pub struct Interaction {
    user_interface: UserInterfaceID,
    settings: External<InteractionSettings>,
    road: RoadInteraction,
}

#[derive(Serialize, Deserialize, Clone)]
struct InteractionSettings {
    bindings: Bindings,
}

impl Default for InteractionSettings {
    fn default() -> Self {
        use stagemaster::combo::Button::*;

        InteractionSettings {
            bindings: Bindings::new(
                vec![
                    ("Materialize Plan", Combo2::new(&[Return], &[])),
                    ("Undo Step", Combo2::new(&[LControl, Z], &[LWin, Z])),
                    (
                        "Redo Step",
                        Combo2::new(&[LControl, LShift, Z], &[LWin, LShift, Z])
                    ),
                    // TODO: this has nothing to do with anything here
                    ("Spawn Cars", Combo2::new(&[C], &[])),
                ].into_iter()
                    .chain(default_road_planning_bindings())
                    .collect(),
            ),
        }
    }
}

use monet::RendererID;
use stagemaster::UserInterfaceID;

impl Interaction {
    pub fn init(
        world: &mut World,
        user_interface: UserInterfaceID,
        renderer_id: RendererID,
        id: PlanManagerID,
    ) -> Interaction {
        user_interface.add(
            ::ui_layers::BASE_LAYER,
            id.into(),
            AnyShape::Everywhere,
            0,
            world,
        );
        user_interface.add_2d(id.into(), world);
        user_interface.focus(id.into(), world);
        Interaction {
            settings: External::new(::ENV.load_settings("Plan Editing")),
            user_interface,
            road: RoadInteraction::init(world, renderer_id, user_interface, id),
        }
    }
}

impl PlanManager {
    pub fn invalidate_interactables(&mut self) {
        self.interactables_valid = false;
    }

    pub fn update_interactables(&mut self, world: &mut World) {
        // TODO: ugly that we have think about this here
        let built_strokes_after_delta = self.built_strokes_after_delta();
        self.interaction.road.update_interactables(
            world,
            match self.current.intent {
                Intent::RoadIntent(ref road_intent) => Some(road_intent),
                _ => None,
            },
            &self.current.plan_delta.roads,
            &self.current.selections,
            &built_strokes_after_delta,
            self.interaction.user_interface,
            self.id,
        );

        self.interactables_valid = true;
    }

    pub fn on_step(&mut self, world: &mut World) {
        self.interaction.road.on_step(
            world,
            match self.current.intent {
                Intent::RoadIntent(ref road_intent) => Some(road_intent),
                _ => None,
            },
        );
    }
}

use stagemaster::{Interactable3d, Interactable3dID, Interactable2d, Interactable2dID, Event3d};

impl Interactable3d for PlanManager {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        if let Event3d::Combos(combos) = event {
            self.interaction.settings.bindings.do_rebinding(
                &combos.current,
            );
            let bindings = &self.interaction.settings.bindings;

            if bindings["Materialize Plan"].is_freshly_in(&combos) {
                self.id.materialize(world);
            }

            if bindings["Redo Step"].is_freshly_in(&combos) {
                self.id.redo(world);
            } else if bindings["Undo Step"].is_freshly_in(&combos) {
                self.id.undo(world);
            }

            if bindings["Spawn Cars"].is_freshly_in(&combos) {
                // TODO: this is not supposed to be here!
                //       *but we have only one focusable!*
                //       WTF?! what's wrong with your UI model?
                //       *I uh.. I guess I should actually write a good one*
                //       When will you finally?!
                //       *Uh.. next week maybe?*
                use descartes::P3;
                let lanes_as_interactables: Interactable3dID = Lane::global_broadcast(world).into();
                for _i in 0..100 {
                    lanes_as_interactables.on_event(
                        Event3d::DragFinished {
                            from: P3::new(0.0, 0.0, 0.0),
                            from2d: P2::new(0.0, 0.0),
                            to: P3::new(0.0, 0.0, 0.0),
                            to2d: P2::new(0.0, 0.0),
                        },
                        world,
                    );
                }
            }
        };

        self.interaction.road.handle_event(
            world,
            self.id,
            event,
            &self.interaction.settings.bindings,
        );
    }
}

impl Interactable2d for PlanManager {
    fn draw(&mut self, _: &mut World, ui: &::imgui::Ui<'static>) {
        ui.window(im_str!("Controls")).build(|| {
            ui.text(im_str!("Plan Editing"));
            ui.separator();

            if self.interaction.settings.bindings.settings_ui(ui) {
                ::ENV.write_settings("Plan Editing", &*self.interaction.settings)
            }

            ui.spacing();
        });
    }
}

pub fn setup(system: &mut ActorSystem) {
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
