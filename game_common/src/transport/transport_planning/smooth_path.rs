use descartes::{P2, CurvedPath};

pub fn smooth_path_from(points: &[P2]) -> Option<CurvedPath> {
    let center_points = points
        .windows(2)
        .map(|point_pair| P2::from_coordinates((point_pair[0].coords + point_pair[1].coords) / 2.0))
        .collect::<Vec<_>>();

    // for each straight line segment, we have first: a point called END,
    // marking the end of the circular arc that smoothes the first corner of
    // this line segment and then second: a point called START,
    // marking the beginning of the circular arc that smoothes the second corner
    // of this line segments. Also, we remember the direction of the line segment

    let mut end_start_directions = Vec::new();

    for (i, point_pair) in points.windows(2).enumerate() {
        let first_corner = point_pair[0];
        let second_corner = point_pair[1];
        let previous_center_point = if i == 0 {
            &first_corner
        } else {
            &center_points[i - 1]
        };
        let this_center_point = center_points[i];
        let next_center_point = center_points.get(i + 1).unwrap_or(&second_corner);
        let line_direction = (second_corner - first_corner).normalize();

        let shorter_distance_to_first_corner = (first_corner - previous_center_point)
            .norm()
            .min((first_corner - this_center_point).norm());
        let shorter_distance_to_second_corner = (second_corner - this_center_point)
            .norm()
            .min((second_corner - next_center_point).norm());

        let end = first_corner + line_direction * shorter_distance_to_first_corner;
        let start = second_corner - line_direction * shorter_distance_to_second_corner;

        end_start_directions.push((end, start, line_direction));
    }

    let mut collected_path: Option<CurvedPath> = None;
    let mut previous_point = points[0];
    let mut previous_direction = (points[1] - points[0]).normalize();

    for (end, start, direction) in end_start_directions {
        if let Some(valid_incoming_arc) = CurvedPath::arc(previous_point, previous_direction, end) {
            collected_path = collected_path.map_or(Some(valid_incoming_arc.clone()), |path| {
                path.concat(&valid_incoming_arc).ok()
            });
        }

        if let Some(valid_connecting_line) = CurvedPath::line(end, start) {
            collected_path = collected_path.map_or(Some(valid_connecting_line.clone()), |path| {
                path.concat(&valid_connecting_line).ok()
            });
            previous_point = start;
        } else {
            previous_point = end;
        }

        previous_direction = direction;
    }

    collected_path
}
