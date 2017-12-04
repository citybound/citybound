use compact::CVec;
use kay::World;
use planning::plan_manager::{PlanManagerID, PlanManager, Intent, IntentProgress};
use super::{RoadIntent, RoadSelections, SelectableStrokeRef};
use super::super::road_plan::RoadPlanDelta;
use super::super::materialized_roads::BuiltStrokes;
use stagemaster::combo::{Bindings, Combo2};
use stagemaster::user_interface::Event3d;
use descartes::{N, P2, V2, FiniteCurve, Norm};
use rand::{Rng, thread_rng};

use super::helper_interactables::{DeselecterID, AddableID, DraggableID, SelectableID,
                                  StrokeCanvasID, StrokeState};

#[derive(Compact, Clone)]
pub struct RoadInteraction {
    selectables: CVec<SelectableID>,
    addables: CVec<AddableID>,
    draggables: CVec<DraggableID>,
    pub stroke_canvas: StrokeCanvasID,
    deselecter: Option<DeselecterID>,
}

pub fn default_road_planning_bindings() -> Vec<(&'static str, Combo2)> {
    use stagemaster::combo::Button::*;

    vec![
        ("Create Rural Roads", Combo2::new(&[R], &[])),
        ("Create Small Grid", Combo2::new(&[G], &[])),
        ("Create Large Grid", Combo2::new(&[LShift, G], &[])),
        ("Delete Selection", Combo2::new(&[Back], &[Delete])),
    ]
}

use monet::{RendererID, EyeListener, Eye, Movement, EyeListenerID};
use stagemaster::UserInterfaceID;

impl RoadInteraction {
    pub fn init(
        world: &mut World,
        renderer_id: RendererID,
        user_interface: UserInterfaceID,
        plan_manager: PlanManagerID,
    ) -> RoadInteraction {
        renderer_id.add_eye_listener(0, plan_manager.into(), world);
        RoadInteraction {
            selectables: CVec::new(),
            addables: CVec::new(),
            draggables: CVec::new(),
            stroke_canvas: StrokeCanvasID::spawn(user_interface, plan_manager, world),
            deselecter: None,
        }
    }
}

impl EyeListener for PlanManager {
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

impl PlanManager {
    pub fn on_stroke(&mut self, points: &CVec<P2>, state: StrokeState, _: &mut World) {
        let maybe_new_intent = match self.current.intent {
            Intent::RoadIntent(RoadIntent::ContinueRoad(ref continue_from,
                                                        _,
                                                        start_reference_point)) => {
                Some(Intent::RoadIntent(RoadIntent::ContinueRoad(
                    continue_from.clone(),
                    points.clone(),
                    start_reference_point,
                )))
            }
            Intent::None |
            Intent::RoadIntent(..) => {
                if points.len() >= 2 {
                    self.invalidate_interactables();
                    Some(Intent::RoadIntent(RoadIntent::NewRoad(points.clone())))
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
}

impl RoadInteraction {
    #[allow(too_many_arguments)]
    pub fn update_interactables(
        &mut self,
        world: &mut World,
        intent: Option<&RoadIntent>,
        road_delta: &RoadPlanDelta,
        selections: &RoadSelections,
        built_strokes_after_delta: &BuiltStrokes,
        user_interface: UserInterfaceID,
        plan_manager: PlanManagerID,
    ) {
        for selectable in self.selectables.drain() {
            selectable.clear(user_interface, world);
        }
        for draggable in self.selectables.drain() {
            draggable.clear(user_interface, world);
        }
        for addable in self.selectables.drain() {
            addable.clear(user_interface, world);
        }
        if let Some(deselecter) = self.deselecter.take() {
            deselecter.clear(user_interface, world);
        }

        self.deselecter = if selections.is_empty() {
            None
        } else {
            Some(DeselecterID::spawn(user_interface, plan_manager, world))
        };

        match intent {
            Some(&RoadIntent::ContinueRoad(..)) |
            Some(&RoadIntent::NewRoad(..)) |
            Some(&RoadIntent::ContinueRoadAround(..)) => {}
            _ => {
                for (i, stroke) in road_delta.new_strokes.iter().enumerate() {
                    self.selectables.push(SelectableID::spawn(
                        SelectableStrokeRef::New(i),
                        stroke.path().clone(),
                        user_interface,
                        plan_manager,
                        world,
                    ));
                }
                for (old_stroke_ref, stroke) in built_strokes_after_delta.mapping.pairs() {
                    self.selectables.push(SelectableID::spawn(
                        SelectableStrokeRef::Built(*old_stroke_ref),
                        stroke.path().clone(),
                        user_interface,
                        plan_manager,
                        world,
                    ));
                }
            }
        }
        for (&selection_ref, &(start, end)) in selections.pairs() {
            let stroke = selection_ref.get_stroke(road_delta, built_strokes_after_delta);
            if let Some(subsection) = stroke.path().subsection(start, end) {
                self.draggables.push(DraggableID::spawn(
                    selection_ref,
                    subsection.clone(),
                    user_interface,
                    plan_manager,
                    world,
                ));
                if let Some(next_lane_path) = subsection.shift_orthogonally(5.0) {
                    self.addables.push(AddableID::spawn(
                        next_lane_path,
                        user_interface,
                        plan_manager,
                        world,
                    ));
                }
            }
        }
    }

    pub fn on_step(&mut self, world: &mut World, intent: Option<&RoadIntent>) {
        let points = match intent {
            Some(&RoadIntent::ContinueRoad(_, ref points, _)) |
            Some(&RoadIntent::NewRoad(ref points)) => points.clone(),
            _ => CVec::new(),
        };

        self.stroke_canvas.set_points(points, world);
    }

    pub fn handle_event(
        &mut self,
        world: &mut World,
        plan_manager: PlanManagerID,
        event: Event3d,
        bindings: &Bindings,
    ) {
        match event {
            Event3d::Combos(combos) => {
                let maybe_grid_size = if bindings["Create Large Grid"].is_freshly_in(&combos) {
                    Some(15usize)
                } else if bindings["Create Small Grid"].is_freshly_in(&combos) {
                    Some(10usize)
                } else {
                    None
                };

                if let Some(grid_size) = maybe_grid_size {
                    const GRID_SPACING: N = 1000.0;
                    let half_grid_extent = grid_size as f32 * GRID_SPACING / 2.0;
                    for x in 0..grid_size {
                        plan_manager.on_stroke(
                            vec![
                                P2::new(
                                    (x as f32 + 0.5) * GRID_SPACING - half_grid_extent,
                                    -half_grid_extent
                                ),
                                P2::new(
                                    (x as f32 + 0.5) * GRID_SPACING - half_grid_extent,
                                    grid_size as f32 * GRID_SPACING - half_grid_extent
                                ),
                            ].into(),
                            StrokeState::Finished,
                            world,
                        );

                    }
                    for y in 0..grid_size {
                        plan_manager.on_stroke(
                            vec![
                                P2::new(
                                    -half_grid_extent,
                                    (y as f32 + 0.5) * GRID_SPACING - half_grid_extent
                                ),
                                P2::new(
                                    grid_size as f32 * GRID_SPACING - half_grid_extent,
                                    (y as f32 + 0.5) * GRID_SPACING - half_grid_extent
                                ),
                            ].into(),
                            StrokeState::Finished,
                            world,
                        );
                    }
                }

                if bindings["Create Rural Roads"].is_freshly_in(&combos) {
                    let mut rnd = thread_rng();
                    let mut start_point = P2::new(
                        rnd.gen_range(-2000.0, 2000.0),
                        rnd.gen_range(-2000.0, 2000.0),
                    );
                    let mut rough_angle = rnd.next_f32() * 2.0 * ::std::f32::consts::PI;
                    for i in 0..rnd.gen_range(15, 30) {
                        let is_major = i < 6;
                        plan_manager.set_n_lanes(
                            if is_major {
                                2
                            } else {
                                *rnd.choose(&[1, 2]).unwrap()
                            },
                            world,
                        );
                        rough_angle += rnd.gen_range(0.4, 0.6) * ::std::f32::consts::PI;
                        let mut rough_direction = V2::new(rough_angle.sin(), rough_angle.cos());
                        let mut points = Vec::new();
                        let length = if is_major {
                            rnd.gen_range(6000.0, 8000.0)
                        } else {
                            rnd.gen_range(300.0, 2000.0)
                        };
                        let mut point = start_point -
                            rnd.gen_range(0.0, 0.5) * length * rough_direction;
                        for p in 0..rnd.gen_range(10, 15) {
                            points.push(point);
                            point += length / 10.0 * rough_direction;
                            if p == 9 {
                                start_point = point;
                            }
                            if rnd.gen_weighted_bool(7) {
                                rough_angle += rnd.gen_range(-0.1, 0.1);
                                rough_direction = V2::new(rough_angle.sin(), rough_angle.cos());
                            }
                        }
                        plan_manager.on_stroke(points.into(), StrokeState::Finished, world);
                    }
                    plan_manager.set_n_lanes(2, world);
                }

                if bindings["Delete Selection"].is_freshly_in(&combos) {
                    plan_manager.change_intent(
                        Intent::RoadIntent(RoadIntent::DeleteSelection),
                        IntentProgress::Immediate,
                        world,
                    );
                }
            }
            Event3d::ButtonDown(::stagemaster::combo::Button::NumberKey(num)) => {
                if num == 0 {
                    plan_manager.toggle_both_sides(world);
                } else {
                    plan_manager.set_n_lanes(num as usize, world);
                }
            }
            _ => {}
        };
    }
}

mod kay_auto;
pub use self::kay_auto::*;
