use super::{N, P2, LinePath,   AreaSplitResult, VecLike, SUBJECT_A, SUBJECT_B};
use ordered_float::OrderedFloat;

impl LinePath {
    pub fn from_svg(string: &str) -> Option<Self> {
        let mut tokens = string.split_whitespace();
        let mut points = vec![];

        while let Some(command) = tokens.next() {
            if command == "M" || command == "L" {
                let x: N = tokens
                    .next()
                    .expect("Expected 1st token after M/L")
                    .parse()
                    .expect("Can't parse 1st token after M/L");
                let y: N = tokens
                    .next()
                    .expect("Expected 2nd token after M/L")
                    .parse()
                    .expect("Can't parse 2nd token after M/L");

                points.push(P2::new(x, y));
            } else if command == "Z" {
                let first_point = points[0];
                points.push(first_point)
            }
        }

        Self::new(points.into())
    }

    pub fn to_svg(&self) -> String {
        format!(
            "M {}",
            self.points
                .iter()
                .map(|point| format!("{} {}", point.x, point.y))
                .collect::<Vec<_>>()
                .join(" L ")
        )
    }
}

impl<'a> AreaSplitResult<'a> {
    pub fn debug_svg(&self) -> String {
        let piece_points = self
            .pieces
            .iter()
            .flat_map(|piece| {
                piece
                    .to_path()
                    .map(|path| path.points.clone())
                    .unwrap_or(VecLike::new())
            })
            .collect::<Vec<_>>();

        let min_x = *piece_points
            .iter()
            .map(|p| OrderedFloat(p.x))
            .min()
            .unwrap();
        let max_x = *piece_points
            .iter()
            .map(|p| OrderedFloat(p.x))
            .max()
            .unwrap();
        let min_y = *piece_points
            .iter()
            .map(|p| OrderedFloat(p.y))
            .min()
            .unwrap();
        let max_y = *piece_points
            .iter()
            .map(|p| OrderedFloat(p.y))
            .max()
            .unwrap();

        let width = max_x - min_x;
        let height = max_y - min_y;

        let stroke_width = width.max(height) / 200.0;

        format!(
            r#"
        <svg width="700" height="700" viewbox="{} {} {} {}" xmlns="http://www.w3.org/2000/svg">
            <g fill="none" stroke="rgba(0, 0, 0, 0.3)"
            stroke-width="{}" marker-end="url(#subj_marker)">
                <marker id="subj_marker" viewBox="0 0 6 6"
                        refX="6" refY="3" markerUnits="strokeWidth" orient="auto">
                    <path d="M 0 0 L 6 3 L 0 6 z" stroke-width="1"/>
                </marker>
                {}
            </g>
            <g fill="none" stroke-width="{}">
                {}
            </g>
        </svg>
        "#,
            min_x - width * 0.1,
            min_y - height * 0.1,
            width * 1.2,
            height * 1.2,
            stroke_width,
            self.pieces
                .iter()
                .filter_map(|piece| piece
                    .to_path()
                    .map(|path| format!(r#"<path d="{}"/>"#, path.to_svg())))
                .collect::<Vec<_>>()
                .join(" "),
            stroke_width,
            self.pieces
                .iter()
                .flat_map(|piece| {
                    let mut side_paths = vec![];

                    if piece.left_inside[SUBJECT_A] {
                        side_paths.push((stroke_width, "rgba(0, 0, 255, 0.3)"));
                    }

                    if piece.left_inside[SUBJECT_B] {
                        side_paths.push((stroke_width, "rgba(255, 0, 0, 0.3)"));
                    }

                    if piece.right_inside[SUBJECT_A] {
                        side_paths.push((-stroke_width, "rgba(0, 0, 255, 0.3)"));
                    }

                    if piece.right_inside[SUBJECT_B] {
                        side_paths.push((-stroke_width, "rgba(255, 0, 0, 0.3)"));
                    }

                    side_paths
                        .into_iter()
                        .filter_map(|(shift, color)| {
                            piece.to_path().and_then(|path| {
                                path.shift_orthogonally(shift).map(|path| {
                                    format!(r#"<path d="{}" stroke="{}"/>"#, path.to_svg(), color)
                                })
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
                .join(" "),
        )
    }
}