use kay::{World, TypedID};
use descartes::{P2, Band, Segment, Circle, Path, AsArea};
use monet::{RendererID, Renderable, RenderableID, Instance, Mesh};
use style::colors;
use style::dimensions::CONTROL_POINT_HANDLE_RADIUS;
use render_layers::RenderLayers;

use super::{PlanManager, PlanManagerID};
use super::interaction::ControlPointRef;

impl Renderable for PlanManager {
    fn init(&mut self, renderer_id: RendererID, world: &mut World) {
        let dot_mesh = Mesh::from_band(
            &Band::new(
                Circle {
                    center: P2::new(0.0, 0.0),
                    radius: CONTROL_POINT_HANDLE_RADIUS,
                }.as_area()
                    .primitives
                    [0]
                    .boundary
                    .clone(),
                0.3,
            ),
            1.0,
        );

        renderer_id.add_batch(RenderLayers::PlanningGestureDots as u32, dot_mesh, world);
    }

    fn prepare_render(&mut self, renderer_id: RendererID, _frame: usize, world: &mut World) {
        let proposal_id = self.ui_state
            .get(renderer_id.as_raw().machine)
            .expect("should have ui state for this renderer")
            .current_proposal;
        self.ensure_preview(renderer_id.as_raw().machine, proposal_id, world);
    }

    fn render(&mut self, renderer_id: RendererID, frame: usize, world: &mut World) {
        // TODO: clean up this mess
        let proposal_id = self.ui_state
            .get(renderer_id.as_raw().machine)
            .expect("should have ui state for this renderer")
            .current_proposal;
        let (preview, result_preview, maybe_actions_preview) =
            self.ensure_preview(renderer_id.as_raw().machine, proposal_id, world);

        for render_fn in &[
            ::transport::transport_planning::interaction::render_preview,
            ::land_use::zone_planning::interaction::render_preview,
        ]
        {
            render_fn(
                result_preview,
                maybe_actions_preview,
                renderer_id,
                frame,
                world,
            );
        }

        for (i, gesture) in preview.gestures.values().enumerate() {
            if gesture.points.len() >= 2 {
                let line_mesh = if let Ok(line_path) = Path::new(
                    gesture
                        .points
                        .windows(2)
                        .filter_map(|window| Segment::line(window[0], window[1]))
                        .collect(),
                )
                {
                    Mesh::from_band(&Band::new(line_path, 0.3), 1.0)
                } else {
                    Mesh::empty()
                };
                renderer_id.update_individual(
                    RenderLayers::PlanningGestureLines as u32 + i as u32,
                    line_mesh,
                    Instance::with_color(colors::GESTURE_LINES),
                    true,
                    world,
                );
            }
        }

        let selected_points = &self.ui_state
            .get(renderer_id.as_raw().machine)
            .expect("should have ui state for this renderer")
            .selected_points;

        let control_point_instances = preview
            .gestures
            .pairs()
            .flat_map(|(gesture_id, gesture)| {
                gesture.points.iter().enumerate().map(move |(point_index,
                       point)| {
                    Instance {
                        instance_position: [point.x, point.y, 0.0],
                        instance_direction: [1.0, 0.0],
                        instance_color: if selected_points.contains(&ControlPointRef(
                            *gesture_id,
                            point_index,
                        ))
                        {
                            colors::CONTROL_POINT_SELECTED
                        } else {
                            colors::CONTROL_POINT
                        },
                    }
                })
            })
            .collect();

        renderer_id.add_several_instances(
            RenderLayers::PlanningGestureDots as u32,
            frame,
            control_point_instances,
            world,
        );
    }
}

pub mod kay_auto;
pub use self::kay_auto::auto_setup;
