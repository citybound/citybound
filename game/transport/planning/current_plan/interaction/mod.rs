use compact::CVec;
use kay::{ActorSystem, Fate, External, World};
use kay::swarm::Swarm;
use super::{CurrentPlan, CurrentPlanID, SelectableStrokeRef};
use stagemaster::geometry::AnyShape;
use stagemaster::combo::{Bindings, Combo2};
use stagemaster::combo::Button::*;
use descartes::{N, P2, FiniteCurve};

use super::helper_interactables::{DeselecterID, AddableID, DraggableID, SelectableID,
                                  StrokeCanvasID, StrokeState};

#[derive(Compact, Clone)]
pub struct Interaction {
    user_interface: UserInterfaceID,
    selectables: CVec<SelectableID>,
    addables: CVec<AddableID>,
    draggables: CVec<DraggableID>,
    pub stroke_canvas: StrokeCanvasID,
    deselecter: Option<DeselecterID>,
    settings: External<InteractionSettings>,
}

#[derive(Serialize, Deserialize, Clone)]
struct InteractionSettings {
    bindings: Bindings,
}

impl Default for InteractionSettings {
    fn default() -> Self {
        InteractionSettings {
            bindings: Bindings::new(vec![
                ("Materialize Plan", Combo2::new(&[Return], &[])),
                ("Undo Step", Combo2::new(&[LControl, Z], &[LWin, Z])),
                (
                    "Redo Step",
                    Combo2::new(&[LControl, LShift, Z], &[LWin, LShift, Z])
                ),
                ("Spawn Cars", Combo2::new(&[C], &[])),
                ("Create Small Grid", Combo2::new(&[G], &[])),
                ("Create Large Grid", Combo2::new(&[LShift, G], &[])),
                ("Delete Selection", Combo2::new(&[Back], &[Delete])),
            ]),
        }
    }
}

use monet::{RendererID, EyeListener, Eye, Movement, EyeListenerID, MSG_EyeListener_eye_moved};
use stagemaster::UserInterfaceID;
use transport::lane::Lane;

impl Interaction {
    pub fn init(
        world: &mut World,
        user_interface: UserInterfaceID,
        renderer_id: RendererID,
        id: CurrentPlanID,
    ) -> Interaction {
        user_interface.add(id.into(), AnyShape::Everywhere, 0, world);
        user_interface.add_2d(id.into(), world);
        user_interface.focus(id.into(), world);
        renderer_id.add_eye_listener(0, id.into(), world);
        Interaction {
            settings: External::new(::ENV.load_settings("Plan Editing")),
            selectables: CVec::new(),
            addables: CVec::new(),
            draggables: CVec::new(),
            stroke_canvas: StrokeCanvasID::spawn(user_interface, id, world),
            deselecter: None,
            user_interface,
        }
    }
}

impl EyeListener for CurrentPlan {
    fn eye_moved(&mut self, eye: Eye, _movement: Movement, _: &mut World) {
        if eye.position.z < 100.0 {
            self.settings.select_parallel = false;
            self.settings.select_opposite = false;
        } else if eye.position.z < 130.0 {
            self.settings.select_parallel = true;
            self.settings.select_opposite = false;
        } else {
            self.settings.select_parallel = true;
            self.settings.select_opposite = true;
        }
    }
}

impl CurrentPlan {
    pub fn invalidate_interactables(&mut self) {
        self.interactables_valid = false;
    }

    pub fn update_interactables(&mut self, world: &mut World) {
        for selectable in self.interaction.selectables.drain() {
            selectable.clear(self.interaction.user_interface, world);
        }
        for draggable in self.interaction.selectables.drain() {
            draggable.clear(self.interaction.user_interface, world);
        }
        for addable in self.interaction.selectables.drain() {
            addable.clear(self.interaction.user_interface, world);
        }
        if let Some(deselecter) = self.interaction.deselecter.take() {
            deselecter.clear(self.interaction.user_interface, world);
        }

        self.interaction.deselecter = if self.current.selections.is_empty() {
            None
        } else {
            Some(DeselecterID::spawn(
                self.interaction.user_interface,
                self.id,
                world,
            ))
        };

        if let Some(still_built_strokes) = self.still_built_strokes() {
            match self.current.intent {
                Intent::ContinueRoad(..) |
                Intent::NewRoad(..) |
                Intent::ContinueRoadAround(..) => {}
                _ => {
                    for (i, stroke) in self.current.plan_delta.new_strokes.iter().enumerate() {
                        self.interaction.selectables.push(SelectableID::spawn(
                            SelectableStrokeRef::New(i),
                            stroke.path().clone(),
                            self.interaction.user_interface,
                            self.id,
                            world,
                        ));
                    }
                    for (old_stroke_ref, stroke) in still_built_strokes.mapping.pairs() {
                        self.interaction.selectables.push(SelectableID::spawn(
                            SelectableStrokeRef::Built(*old_stroke_ref),
                            stroke.path().clone(),
                            self.interaction.user_interface,
                            self.id,
                            world,
                        ));
                    }
                }
            }
            for (&selection_ref, &(start, end)) in self.current.selections.pairs() {
                let stroke =
                    selection_ref.get_stroke(&self.current.plan_delta, &still_built_strokes);
                if let Some(subsection) = stroke.path().subsection(start, end) {
                    self.interaction.draggables.push(DraggableID::spawn(
                        selection_ref,
                        subsection.clone(),
                        self.interaction.user_interface,
                        self.id,
                        world,
                    ));
                    if let Some(next_lane_path) = subsection.shift_orthogonally(5.0) {
                        self.interaction.addables.push(AddableID::spawn(
                            next_lane_path,
                            self.interaction.user_interface,
                            self.id,
                            world,
                        ));
                    }
                }
            }
            self.interactables_valid = true;
        }
    }
}

use stagemaster::{Interactable3d, Interactable3dID, Interactable2d, Interactable2dID, Event3d,
                  MSG_Interactable3d_on_event, MSG_Interactable2d_draw_ui_2d};
use super::{Intent, IntentProgress};

impl Interactable3d for CurrentPlan {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        match event {
            Event3d::Combos(combos) => {
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
                    use kay::swarm::ToRandom;
                    use descartes::P3;
                    let all_lanes = world.global_broadcast::<Swarm<Lane>>();
                    world.send(
                        all_lanes,
                        ToRandom {
                            n_recipients: 5000,
                            message: Event3d::DragFinished {
                                from: P3::new(0.0, 0.0, 0.0),
                                from2d: P2::new(0.0, 0.0),
                                to: P3::new(0.0, 0.0, 0.0),
                                to2d: P2::new(0.0, 0.0),
                            },
                        },
                    );
                }

                let maybe_grid_size = if bindings["Create Large Grid"].is_freshly_in(&combos) {
                    Some(15usize)
                } else if bindings["Create Small Grid"].is_freshly_in(&combos) {
                    Some(10usize)
                } else {
                    None
                };

                if let Some(grid_size) = maybe_grid_size {
                    const GRID_SPACING: N = 1000.0;
                    for x in 0..grid_size {
                        self.id.on_stroke(
                            vec![
                                P2::new((x as f32 + 0.5) * GRID_SPACING, 0.0),
                                P2::new(
                                    (x as f32 + 0.5) * GRID_SPACING,
                                    grid_size as f32 * GRID_SPACING
                                ),
                            ].into(),
                            StrokeState::Finished,
                            world,
                        );

                    }
                    for y in 0..grid_size {
                        self.id.on_stroke(
                            vec![
                                P2::new(0.0, (y as f32 + 0.5) * GRID_SPACING),
                                P2::new(
                                    grid_size as f32 * GRID_SPACING,
                                    (y as f32 + 0.5) * GRID_SPACING
                                ),
                            ].into(),
                            StrokeState::Finished,
                            world,
                        );
                    }
                }

                if bindings["Delete Selection"].is_freshly_in(&combos) {
                    self.id.change_intent(
                        Intent::DeleteSelection,
                        IntentProgress::Immediate,
                        world,
                    );
                }
            }
            Event3d::ButtonDown(NumberKey(num)) => {
                if num == 0 {
                    self.id.toggle_both_sides(world);
                } else {
                    self.id.set_n_lanes(num as usize, world);
                }
            }
            _ => {}
        };
    }
}

impl Interactable2d for CurrentPlan {
    fn draw_ui_2d(
        &mut self,
        imgui_ui: &External<::imgui::Ui<'static>>,
        return_to: UserInterfaceID,
        world: &mut World,
    ) {
        let ui = imgui_ui.steal();

        ui.window(im_str!("Controls")).build(|| {
            ui.text(im_str!("Plan Editing"));
            ui.separator();

            if self.interaction.settings.bindings.settings_ui(&ui) {
                ::ENV.write_settings("Plan Editing", &*self.interaction.settings)
            }

            ui.spacing();
        });

        return_to.ui_drawn(ui, world);
    }
}

pub fn setup(system: &mut ActorSystem) {
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
