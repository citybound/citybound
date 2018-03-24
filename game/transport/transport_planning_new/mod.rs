use kay::World;
use compact::CVec;
use descartes::{N, P2, Band, Segment, Path, FiniteCurve};
use monet::{RendererID, Instance};
use stagemaster::geometry::{band_to_geometry, CPath};
use style::colors;

use planning_new::{Plan, GestureIntent, PlanResult, Prototype};

#[derive(Compact, Clone)]
pub struct RoadIntent {
    n_lanes_forward: u8,
    n_lanes_backward: u8,
}

impl RoadIntent {
    pub fn new(n_lanes_forward: u8, n_lanes_backward: u8) -> Self {
        RoadIntent { n_lanes_forward, n_lanes_backward }
    }
}

#[derive(Compact, Clone)]
pub enum RoadPrototype {
    Lane(LanePrototype),
    Intersection(IntersectionPrototype),
}

#[derive(Compact, Clone)]
pub struct LanePrototype(CPath);

#[derive(Compact, Clone)]
pub struct IntersectionPrototype {
    connecting_lanes: CVec<LanePrototype>,
    timings: CVec<CVec<bool>>,
}

const LANE_WIDTH: N = 6.0;
const LANE_DISTANCE: N = 0.8 * LANE_WIDTH;
const CENTER_LANE_DISTANCE: N = LANE_DISTANCE;

pub fn calculate_prototypes(plan: &Plan) -> Vec<Prototype> {
    plan.gestures
        .values()
        .flat_map(|gesture| if let GestureIntent::Road(ref road_intent) =
            gesture.intent
        {
            if gesture.points.len() >= 2 {
                let center_points = gesture
                    .points
                    .windows(2)
                    .map(|point_pair| {
                        P2::from_coordinates((point_pair[0].coords + point_pair[1].coords) / 2.0)
                    })
                    .collect::<Vec<_>>();

                // for each straight line segment, we have first: a point called END,
                // marking the end of the circular arc that smoothes the first corner of
                // this line segment and then second: a point called START,
                // marking the beginning of the circular arc that smoothes the second corner
                // of this line segments. Also, we remember the direction of the line segment

                let mut end_start_directions = Vec::new();

                for (i, point_pair) in gesture.points.windows(2).enumerate() {
                    let first_corner = point_pair[0];
                    let second_corner = point_pair[1];
                    let previous_center_point = center_points.get(i - 1).unwrap_or(&first_corner);
                    let this_center_point = center_points[i];
                    let next_center_point = center_points.get(i + 1).unwrap_or(&second_corner);
                    let line_direction = (second_corner - first_corner).normalize();

                    let shorter_distance_to_first_corner =
                        (first_corner - previous_center_point).norm().min(
                            (first_corner - this_center_point).norm(),
                        );
                    let shorter_distance_to_second_corner =
                        (second_corner - this_center_point).norm().min(
                            (second_corner - next_center_point).norm(),
                        );

                    let end = first_corner + line_direction * shorter_distance_to_first_corner;
                    let start = second_corner - line_direction * shorter_distance_to_second_corner;

                    end_start_directions.push((end, start, line_direction));
                }

                let mut segments = Vec::new();
                let mut previous_point = gesture.points[0];
                let mut previous_direction = (gesture.points[1] - gesture.points[0]).normalize();

                for (end, start, direction) in end_start_directions {
                    if let Some(valid_incoming_arc) =
                        Segment::arc_with_direction(previous_point, previous_direction, end)
                    {
                        segments.push(valid_incoming_arc);
                    }

                    if let Some(valid_connecting_line) = Segment::line(end, start) {
                        segments.push(valid_connecting_line);
                    }

                    previous_point = start;
                    previous_direction = direction;
                }

                let path = CPath::new(segments);

                (0..road_intent.n_lanes_forward)
                    .into_iter()
                    .map(|lane_i| {
                        CENTER_LANE_DISTANCE / 2.0 + lane_i as f32 * LANE_DISTANCE
                    })
                    .chain((0..road_intent.n_lanes_backward).into_iter().map(
                        |lane_i| {
                            -(CENTER_LANE_DISTANCE / 2.0 + lane_i as f32 * LANE_DISTANCE)
                        },
                    ))
                    .filter_map(|offset| path.shift_orthogonally(offset))
                    .map(|shifted_path| {
                        Prototype::Road(RoadPrototype::Lane(LanePrototype(shifted_path)))
                    })
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        })
        .collect()
}

pub fn render_preview(
    result_preview: &PlanResult,
    renderer_id: RendererID,
    scene_id: usize,
    frame: usize,
    world: &mut World,
) {
    for (i, prototype) in result_preview.prototypes.iter().enumerate() {
        if let Prototype::Road(RoadPrototype::Lane(LanePrototype(ref lane_path))) = *prototype {
            let line_geometry = band_to_geometry(&Band::new(lane_path.clone(), LANE_WIDTH), 0.1);

            renderer_id.update_individual(
                scene_id,
                18_000 + i as u16,
                line_geometry,
                Instance::with_color(colors::STROKE_BASE),
                true,
                world,
            );
        }
    }
}