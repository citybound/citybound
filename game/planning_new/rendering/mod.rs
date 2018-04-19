use kay::{World, TypedID};
use descartes::{P2, V2, Band, Segment, Path};
use monet::{RendererID, Renderable, RenderableID, Instance, Geometry};
use stagemaster::geometry::{band_to_geometry, CPath};
use style::colors;

use super::{PlanManager, PlanManagerID};
use super::interaction::{ControlPointRef, CONTROL_POINT_HANDLE_RADIUS};

impl Renderable for PlanManager {
    fn setup_in_scene(&mut self, renderer_id: RendererID, scene_id: usize, world: &mut World) {
        let dot_geometry = band_to_geometry(
            &Band::new(
                CPath::new(
                    Segment::arc_with_direction(
                        P2::new(-CONTROL_POINT_HANDLE_RADIUS / 2.0, 0.0),
                        V2::new(0.0, 1.0),
                        P2::new(CONTROL_POINT_HANDLE_RADIUS / 2.0, 0.0),
                    ).into_iter()
                        .chain(Segment::arc_with_direction(
                            P2::new(CONTROL_POINT_HANDLE_RADIUS / 2.0, 0.0),
                            V2::new(0.0, -1.0),
                            P2::new(-CONTROL_POINT_HANDLE_RADIUS / 2.0, 0.0),
                        ))
                        .collect(),
                ).unwrap(),
                0.3,
            ),
            1.0,
        );

        renderer_id.add_batch(scene_id, 20_000, dot_geometry, world);
    }

    fn render_to_scene(
        &mut self,
        renderer_id: RendererID,
        scene_id: usize,
        frame: usize,
        world: &mut World,
    ) {
        // TODO: clean up this mess
        let proposal_id = self.ui_state
            .get(renderer_id.as_raw().machine)
            .expect("should have ui state for this renderer")
            .current_proposal;
        let (preview, result_preview) =
            self.ensure_preview(renderer_id.as_raw().machine, proposal_id);

        super::transport_planning_new::render_preview(
            result_preview,
            renderer_id,
            scene_id,
            frame,
            world,
        );

        for (i, gesture) in preview.gestures.values().enumerate() {
            if gesture.points.len() >= 2 {
                let line_geometry = if let Ok(line_path) = CPath::new(
                    gesture
                        .points
                        .windows(2)
                        .filter_map(|window| Segment::line(window[0], window[1]))
                        .collect(),
                )
                {
                    band_to_geometry(&Band::new(line_path, 0.3), 1.0)
                } else {
                    Geometry::empty()
                };
                renderer_id.update_individual(
                    scene_id,
                    19_000 + i as u16,
                    line_geometry,
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

        renderer_id.add_several_instances(scene_id, 20_000, frame, control_point_instances, world);
    }
}

pub mod kay_auto;
pub use self::kay_auto::auto_setup;