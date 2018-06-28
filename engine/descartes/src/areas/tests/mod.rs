use super::{N, P2, LinePath, ClosedLinePath, Area, AreaSplitResult, PrimitiveArea,
THICKNESS, VecLike, SUBJECT_A, SUBJECT_B};
use curved_path::CurvedPath;
use rough_eq::RoughEq;
use ordered_float::OrderedFloat;

impl LinePath {
    fn from_svg(string: &str) -> Option<Self> {
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

        Self::new(points)
    }

    fn to_svg(&self) -> String {
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
            .flat_map(|piece| piece.to_path().map(|path| path.points.clone()).unwrap_or(Vec::new()))
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

impl<'a> RoughEq for &'a ClosedLinePath {
    fn rough_eq_by(&self, other: Self, tolerance: N) -> bool {
        // TODO: is this really equality?
        self.path().points.len() == other.path().points.len() &&
        self.path().segments().all(|self_segment|
            other.path().segments().any(|other_segment|
                self_segment.start().rough_eq_by(other_segment.start(), tolerance) && self_segment.end().rough_eq_by(other_segment.end(), tolerance)
            )
        )
    }
}

impl<'a> RoughEq for &'a PrimitiveArea {
    fn rough_eq_by(&self, other: Self, tolerance: N) -> bool {
        (&self.boundary)
            .rough_eq_by(&other.boundary, tolerance)
    }
}

impl<'a> RoughEq for &'a Area {
    fn rough_eq_by(&self, other: Self, tolerance: N) -> bool {
        self.primitives.len() == other.primitives.len()
            && self.primitives.iter().all(|own_primitive| {
                other
                    .primitives
                    .iter()
                    .any(|other_primitive| own_primitive.rough_eq_by(other_primitive, tolerance))
            })
    }
}

fn svg_test(file_path: &str) {
    use std::fs;
    use std::io::Read;
    use {THICKNESS, RoughEq};

    let mut file = fs::File::open(file_path).unwrap();

    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let mut clip_area = None;
    let mut subject_area = None;
    let mut expected_result_primitive_areas = VecLike::new();

    let path_substrs = contents
        .split("<path")
        .filter(|string| string.contains("d="));

    for path_substr in path_substrs {
        let path_commands_idx = path_substr.find(" d=").unwrap();
        let path_commands = path_substr[path_commands_idx + 4..]
            .splitn(2, '"')
            .next()
            .unwrap();

        let id_idx = path_substr.find(" id=").unwrap();
        let id = path_substr[id_idx + 5..].splitn(2, '"').next().unwrap();

        println!("Found path {} with id {}", path_commands, id);

        let area = PrimitiveArea::new(
            ClosedLinePath::new(LinePath::from_svg(path_commands).unwrap()).unwrap(),
        );

        if id == "subject" {
            subject_area = Some(Area::new(Some(area).into_iter().collect()));
        } else if id == "clip" {
            clip_area = Some(Area::new(Some(area).into_iter().collect()));
        } else if id.starts_with("result") {
            expected_result_primitive_areas.push(area);
        }
    }

    let expected_result_area = Area::new(expected_result_primitive_areas);

    let subject_area = subject_area.expect("should have subject");
    let clip_area = clip_area.expect("should have clip");

    let split_result = subject_area.split(&clip_area);

    println!("{}", split_result.debug_svg());

    let result_area = if file_path.ends_with("intersection.svg") {
        split_result.intersection()
    } else if file_path.ends_with("union.svg") {
        split_result.union()
    } else if file_path.ends_with("difference.svg") {
        split_result.a_minus_b()
    } else if file_path.ends_with("not.svg") {
        split_result.b_minus_a()
    } else {
        panic!("unsupported file ending");
    }.expect("Operation should work");

    assert_eq!(
        expected_result_area.primitives.len(),
        result_area.primitives.len()
    );

    assert!((&result_area).rough_eq_by(&expected_result_area, THICKNESS));
}

#[test]
fn clip_1_difference() {
    svg_test("./src/areas/tests/1_difference.svg");
}

#[test]
fn clip_1_intersection() {
    svg_test("./src/areas/tests/1_intersection.svg");
}

#[test]
fn clip_1_union() {
    svg_test("./src/areas/tests/1_union.svg");
}

#[test]
fn clip_2_difference() {
    svg_test("./src/areas/tests/2_difference.svg");
}

#[test]
fn clip_2_intersection() {
    svg_test("./src/areas/tests/2_intersection.svg");
}

#[test]
fn clip_3_difference() {
    svg_test("./src/areas/tests/3_difference.svg");
}

#[test]
fn area_intersecting_at_curved_road() {
    use V2;
    //     _
    //    | |
    // ,--|-|--.
    // '--|-|.  \
    //    |_| `--'

    assert_eq!(
        Area::new_simple(
            ClosedLinePath::new(
                CurvedPath::line(P2::new(0.0, 0.0), P2::new(2.0, 0.0))
                    .unwrap()
                    .concat(
                        &CurvedPath::arc(P2::new(2.0, 0.0), V2::new(1.0, 0.0), P2::new(3.0, 1.0),)
                            .unwrap()
                    )
                    .unwrap()
                    .concat(&CurvedPath::line(P2::new(3.0, 1.0), P2::new(2.5, 1.0)).unwrap())
                    .unwrap()
                    .concat(
                        &CurvedPath::arc(P2::new(2.5, 1.0), V2::new(0.0, -1.0), P2::new(2.0, 0.5),)
                            .unwrap()
                    )
                    .unwrap()
                    .concat(&CurvedPath::line(P2::new(2.0, 0.5), P2::new(0.0, 0.5)).unwrap())
                    .unwrap()
                    .concat(&CurvedPath::line(P2::new(0.0, 0.5), P2::new(0.0, 0.0)).unwrap())
                    .unwrap()
                    .to_line_path()
            ).unwrap(),
        ).split(&Area::new_simple(
            ClosedLinePath::new(
                LinePath::new(vec![
                    P2::new(1.0, 1.0),
                    P2::new(1.0, -1.0),
                    P2::new(2.0, -1.0),
                    P2::new(2.0, 1.0),
                    P2::new(1.0, 1.0),
                ]).unwrap(),
            ).unwrap(),
        ))
            .intersection()
            .expect("Intersection should work")
            .primitives[0]
            .boundary
            .path(),
        &LinePath::new(vec![
            P2::new(2.0, 0.5),
            P2::new(1.0, 0.5),
            P2::new(1.0, 0.0),
            P2::new(2.0, 0.0),
            P2::new(2.0, 0.5),
        ]).unwrap()
    );
}

const TINY_BIT: N = THICKNESS / 3.0;

#[test]
fn area_intersecting_before_curved_road() {
    use V2;
    //     _
    //    | |
    // ,--(-(--.
    // '--(-(.  \
    //    |_| `--'

    assert_eq!(
        Area::new_simple(
            ClosedLinePath::new(
                CurvedPath::line(P2::new(0.0, 0.0), P2::new(2.0, 0.0))
                    .unwrap()
                    .concat(
                        &CurvedPath::arc(P2::new(2.0, 0.0), V2::new(1.0, 0.0), P2::new(3.0, 1.0),)
                            .unwrap()
                    )
                    .unwrap()
                    .concat(&CurvedPath::line(P2::new(3.0, 1.0), P2::new(2.5, 1.0)).unwrap())
                    .unwrap()
                    .concat(
                        &CurvedPath::arc(P2::new(2.5, 1.0), V2::new(0.0, -1.0), P2::new(2.0, 0.5),)
                            .unwrap()
                    )
                    .unwrap()
                    .concat(&CurvedPath::line(P2::new(2.0, 0.5), P2::new(0.0, 0.5)).unwrap())
                    .unwrap()
                    .concat(&CurvedPath::line(P2::new(0.0, 0.5), P2::new(0.0, 0.0)).unwrap())
                    .unwrap()
                    .to_line_path()
            ).unwrap(),
        ).split(&Area::new_simple(
            ClosedLinePath::new(
                LinePath::new(vec![
                    P2::new(1.0 - TINY_BIT, 1.0),
                    P2::new(1.0 - TINY_BIT, -1.0),
                    P2::new(2.0 - TINY_BIT, -1.0),
                    P2::new(2.0 - TINY_BIT, 1.0),
                    P2::new(1.0 - TINY_BIT, 1.0),
                ]).unwrap(),
            ).unwrap(),
        ))
            .intersection()
            .expect("Intersection should work")
            .primitives[0]
            .boundary
            .path(),
        &LinePath::new(vec![
            P2::new(2.0, 0.5),
            P2::new(1.0, 0.5),
            P2::new(1.0, 0.0),
            P2::new(2.0, 0.0),
            P2::new(2.0, 0.5),
        ]).unwrap()
    );
}