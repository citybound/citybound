use {P2, N, THICKNESS, RoughlyComparable, VecLike};
use curves::{Segment, Curve, FiniteCurve};
use path::Path;
use intersect::Intersect;
use ordered_float::OrderedFloat;

#[derive(Debug)]
pub struct UnclosedPathError;

// represents a filled area bounded by a clockwise boundary
// everything "right of" the boundary is considered "inside"
pub struct PrimitiveArea {
    pub boundary: Path,
}

impl PrimitiveArea {
    pub fn new_unchecked(boundary: Path) -> Self {
        PrimitiveArea { boundary }
    }

    pub fn new(boundary: Path) -> Result<Self, UnclosedPathError> {
        if boundary.is_closed() {
            Ok(Self::new_unchecked(boundary))
        } else {
            Err(UnclosedPathError)
        }
    }

    pub fn location_of(&self, point: P2) -> AreaLocation {
        if self.boundary.includes(point) {
            AreaLocation::Boundary
        } else {
            let ray = Segment::line(point, P2::new(point.x + 10_000_000_000.0, point.y))
                .expect("Ray should be valid");

            let n_intersections = (&Path::new_unchecked(vec![ray]), &self.boundary)
                .intersect()
                .len();

            if n_intersections % 2 == 1 {
                AreaLocation::Inside
            } else {
                AreaLocation::Outside
            }
        }
    }
}

impl<'a> RoughlyComparable for &'a PrimitiveArea {
    fn is_roughly_within(&self, other: Self, tolerance: N) -> bool {
        (&self.boundary).is_roughly_within(&other.boundary, tolerance)
    }
}

pub enum AreaLocation {
    Inside,
    Boundary,
    Outside,
}

const SUBJECT_A: usize = 0;
const SUBJECT_B: usize = 1;
const SUBJECTS: [usize; 2] = [SUBJECT_A, SUBJECT_B];
fn other_subject(subject: usize) -> usize {
    if subject == SUBJECT_A {
        SUBJECT_B
    } else {
        SUBJECT_A
    }
}

pub struct Area {
    pub primitives: VecLike<PrimitiveArea>,
}

impl Area {
    pub fn new(primitives: VecLike<PrimitiveArea>) -> Self {
        Area { primitives }
    }

    pub fn location_of(&self, point: P2) -> AreaLocation {
        if self.primitives.iter().any(|primitive| {
            primitive.boundary.includes(point)
        })
        {
            AreaLocation::Boundary
        } else {
            let ray = Segment::line(point, P2::new(point.x + 10_000_000_000.0, point.y))
                .expect("Ray should be valid");

            // TODO: allow for ccw holes by checking intersection direction
            let mut n_intersections = 0;

            for primitive in &self.primitives {
                n_intersections += (&Path::new_unchecked(vec![ray]), &primitive.boundary)
                    .intersect()
                    .len();
            }

            if n_intersections % 2 == 1 {
                AreaLocation::Inside
            } else {
                AreaLocation::Outside
            }
        }
    }

    pub fn split(&self, b: &Self) -> AreaSplitResult {
        let ab = [self, b];

        let mut intersection_distances = [
            vec![Vec::<N>::new(); self.primitives.len()],
            vec![Vec::<N>::new(); b.primitives.len()],
        ];

        for (i_a, primitive_a) in self.primitives.iter().enumerate() {
            for (i_b, primitive_b) in b.primitives.iter().enumerate() {
                for intersection in (&primitive_a.boundary, &primitive_b.boundary).intersect() {
                    intersection_distances[SUBJECT_A][i_a].push(intersection.along_a);
                    intersection_distances[SUBJECT_B][i_b].push(intersection.along_b);
                }
            }
        }

        let boundary_pieces = SUBJECTS.iter().flat_map(|&subject| {

            for primitive_distances in &mut intersection_distances[subject] {
                primitive_distances.sort_unstable_by_key(|&along|
                    OrderedFloat(along)
                );

                // to close the loop when taking piece-cutting windows
                let first = primitive_distances[0];
                primitive_distances.push(first);
            }

            let mut boundary_pieces_initial = intersection_distances[subject].iter().enumerate().flat_map(|(primitive_i, primitive_distances)|
                primitive_distances.windows(2)
                .filter_map(|distance_pair| {
                    if let Some(subsection) = ab[subject].primitives[primitive_i].boundary.subsection(
                            distance_pair[0],
                            distance_pair[1],
                        ) {
                            Some(BoundaryPiece {
                        path: subsection,
                        left_inside: [false, false],
                        right_inside: [subject == SUBJECT_A, subject == SUBJECT_B],
                    })
                        } else {
                            None
                        }
                }).collect::<Vec<_>>()
            ).collect::<Vec<_>>();

            for boundary_piece in boundary_pieces_initial.iter_mut() {
                let midpoint = boundary_piece.path.midpoint();

                match ab[other_subject(subject)].location_of(midpoint) {
                    AreaLocation::Inside => {
                        boundary_piece.left_inside[other_subject(subject)] = true;
                        boundary_piece.right_inside[other_subject(subject)] = true;
                    }
                    AreaLocation::Outside => {
                        // already correctly initialized
                    }
                    AreaLocation::Boundary => {
                        // both boundary pieces are coincident, but might be opposed

                        let coincident_boundary = ab[other_subject(subject)].primitives.iter()
                            .map(|primitive| &primitive.boundary)
                            .find(|boundary| boundary.includes(midpoint))
                            .expect("Since the midpoint was reported as on boundary, it should be on one!");

                        let distance = coincident_boundary.project(midpoint)
                            .expect("Since the midpoint was reported as on boundary, it should have a projection");

                        let coincident_direction = coincident_boundary.direction_along(distance);

                        if boundary_piece.path.midpoint_direction().dot(&coincident_direction) > 0.0 {
                            boundary_piece.right_inside[other_subject(subject)] = true;
                        } else {
                            boundary_piece.left_inside[other_subject(subject)] = true;
                        }
                    }
                }
            }

            boundary_pieces_initial
        }).collect();

        AreaSplitResult { pieces: boundary_pieces }
    }
}

impl<'a> RoughlyComparable for &'a Area {
    fn is_roughly_within(&self, other: Self, tolerance: N) -> bool {
        self.primitives.len() == other.primitives.len() &&
            self.primitives.iter().all(|own_primitive| {
                other.primitives.iter().any(|other_primitive| {
                    own_primitive.is_roughly_within(other_primitive, tolerance)
                })
            })
    }
}

pub struct BoundaryPiece {
    path: Path,
    left_inside: [bool; 2],
    right_inside: [bool; 2],
}

pub struct AreaSplitResult {
    pieces: VecLike<BoundaryPiece>,
}

pub enum PieceRole {
    Forward,
    Backward,
    NonContributing,
}

impl AreaSplitResult {
    pub fn into_area<F: Fn(&BoundaryPiece) -> PieceRole>(&self, piece_filter: F) -> Area {
        let mut paths = Vec::<Path>::new();
        let mut complete_paths = Vec::<Path>::new();

        for oriented_path in self.pieces.iter().filter_map(
            |piece| match piece_filter(piece) {
                PieceRole::Forward => Some(piece.path.clone()),
                PieceRole::Backward => Some(piece.path.reverse()),
                PieceRole::NonContributing => None,
            },
        )
        {
            let mut maybe_path_before = None;
            let mut maybe_path_after = None;

            for (path_i, path) in paths.iter().enumerate() {
                if path.end().is_roughly_within(
                    oriented_path.start(),
                    THICKNESS,
                )
                {
                    maybe_path_before = Some(path_i)
                }
                if path.start().is_roughly_within(
                    oriented_path.end(),
                    THICKNESS,
                )
                {
                    maybe_path_after = Some(path_i)
                }
            }

            match (maybe_path_before, maybe_path_after) {
                (Some(before_i), Some(after_i)) => {
                    if before_i == after_i {
                        let joined_path = paths[before_i].concat(&oriented_path).expect(
                            "Concat must work at this point (J1)",
                        );

                        paths.remove(before_i);
                        complete_paths.push(joined_path);
                    } else {
                        let joined_path = paths[before_i]
                            .concat(&oriented_path)
                            .expect("Concat must work at this point (J2)")
                            .concat(&paths[after_i])
                            .expect("Concat must work at this point (J3)");

                        paths.remove(before_i.max(after_i));
                        paths[before_i.min(after_i)] = joined_path;
                    }
                }
                (Some(before_i), None) => {
                    let extended_path = paths[before_i].concat(&oriented_path).expect(
                        "Concat must work at this point (B)",
                    );

                    paths[before_i] = extended_path;
                }
                (None, Some(after_i)) => {
                    let extended_path = oriented_path.concat(&paths[after_i]).expect(
                        "Concat must work at this point (A)",
                    );

                    paths[after_i] = extended_path;
                }
                (None, None) => paths.push(oriented_path),
            }
        }

        Area::new(
            complete_paths
                .into_iter()
                .map(|path| PrimitiveArea::new(path).unwrap())
                .collect(),
        )
    }

    pub fn intersection(&self) -> Area {
        self.into_area(|piece| if piece.right_inside[SUBJECT_A] &&
            piece.right_inside[SUBJECT_B]
        {
            PieceRole::Forward
        } else {
            PieceRole::NonContributing
        })
    }

    pub fn union(&self) -> Area {
        self.into_area(|piece| if piece.right_inside[SUBJECT_A] ^
            piece.right_inside[SUBJECT_B]
        {
            PieceRole::Forward
        } else {
            PieceRole::NonContributing
        })
    }

    pub fn a_minus_b(&self) -> Area {
        self.into_area(|piece| if piece.right_inside[SUBJECT_A] &&
            !piece.right_inside[SUBJECT_B]
        {
            PieceRole::Forward
        } else if piece.left_inside[SUBJECT_A] && !piece.left_inside[SUBJECT_B] {
            PieceRole::Backward
        } else {
            PieceRole::NonContributing
        })
    }

    pub fn b_minus_a(&self) -> Area {
        self.into_area(|piece| if piece.right_inside[SUBJECT_B] &&
            !piece.right_inside[SUBJECT_A]
        {
            PieceRole::Forward
        } else if piece.left_inside[SUBJECT_B] && !piece.left_inside[SUBJECT_A] {
            PieceRole::Backward
        } else {
            PieceRole::NonContributing
        })
    }

    pub fn debug_svg(&self) -> String {
        format!(
            r#"
        <svg width="1000" height="1000" viewbox="0 0 500 500" xmlns="http://www.w3.org/2000/svg">
            <g fill="none" stroke="rgba(0, 0, 0, 0.3)"
            stroke-width="1" marker-end="url(#subj_marker)">
                <marker id="subj_marker" viewBox="0 0 6 6"
                        refX="6" refY="3" markerUnits="strokeWidth" orient="auto">
                    <path d="M 0 0 L 6 3 L 0 6 z" fill="rgba(0, 0, 0, 0.3)"/>
                </marker>
                {}
            </g>
            <g fill="none" stroke-width="1">
                {}
            </g>
        </svg>
        "#,
            self.pieces
                .iter()
                .map(|piece| format!(r#"<path d="{}"/>"#, piece.path.to_svg()))
                .collect::<Vec<_>>()
                .join(" "),
            self.pieces
                .iter()
                .flat_map(|piece| {
                    let mut side_paths = vec![];

                    if piece.left_inside[SUBJECT_A] {
                        side_paths.push(format!(
                            r#"<path d="{}" stroke="rgba(0, 0, 255, 0.3)"/>"#,
                            piece
                                .path
                                .shift_orthogonally(1.0) // svg coords are flipped
                                .expect("should be able to shift")
                                .to_svg()
                        ))
                    }

                    if piece.left_inside[SUBJECT_B] {
                        side_paths.push(format!(
                            r#"<path d="{}" stroke="rgba(255, 0, 0, 0.3)"/>"#,
                            piece
                                .path
                                .shift_orthogonally(1.0) // svg coords are flipped
                                .expect("should be able to shift")
                                .to_svg()
                        ))
                    }

                    if piece.right_inside[SUBJECT_A] {
                        side_paths.push(format!(
                            r#"<path d="{}" stroke="rgba(0, 0, 255, 0.3)"/>"#,
                            piece
                                .path
                                .shift_orthogonally(-1.0) // svg coords are flipped
                                .expect("should be able to shift")
                                .to_svg()
                        ))
                    }

                    if piece.right_inside[SUBJECT_B] {
                        side_paths.push(format!(
                            r#"<path d="{}" stroke="rgba(255, 0, 0, 0.3)"/>"#,
                            piece
                                .path
                                .shift_orthogonally(-1.0) // svg coords are flipped
                                .expect("should be able to shift")
                                .to_svg()
                        ))
                    }

                    side_paths
                })
                .collect::<Vec<_>>()
                .join(" "),
        )
    }
}

#[test]
fn svg_tests() {
    use std::fs;
    use std::io::Read;
    use {THICKNESS, RoughlyComparable};

    for dir_entry in fs::read_dir("./src/clipper_testcases").unwrap() {
        let path = dir_entry.unwrap().path();
        let path_str = path.to_str().unwrap();

        if !path_str.ends_with(".svg") {
            continue;
        }

        println!("Testing svg case {}", path.display());

        let mut file = fs::File::open(path.clone()).unwrap();

        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        let mut clip_area = None;
        let mut subject_area = None;
        let mut expected_result_primitive_areas = vec![];

        let path_substrs = contents.split("<path").filter(
            |string| string.contains("d="),
        );

        for path_substr in path_substrs {
            let path_commands_idx = path_substr.find(" d=").unwrap();
            let path_commands = path_substr[path_commands_idx + 4..]
                .splitn(2, '"')
                .next()
                .unwrap();

            let id_idx = path_substr.find(" id=").unwrap();
            let id = path_substr[id_idx + 5..].splitn(2, '"').next().unwrap();

            println!("Found path {} with id {}", path_commands, id);

            let area = PrimitiveArea::new(Path::from_svg(path_commands).unwrap()).unwrap();

            if id == "subject" {
                subject_area = Some(Area::new(vec![area]));
            } else if id == "clip" {
                clip_area = Some(Area::new(vec![area]));
            } else if id.starts_with("result") {
                expected_result_primitive_areas.push(area);
            }
        }

        let expected_result_area = Area::new(expected_result_primitive_areas);

        let subject_area = subject_area.expect("should have subject");
        let clip_area = clip_area.expect("should have clip");

        let split_result = subject_area.split(&clip_area);

        println!("{}", split_result.debug_svg());

        let result_area = if path_str.ends_with("intersection.svg") {
            split_result.intersection()
        } else if path_str.ends_with("union.svg") {
            split_result.union()
        } else if path_str.ends_with("difference.svg") {
            split_result.a_minus_b()
        } else if path_str.ends_with("not.svg") {
            split_result.b_minus_a()
        } else {
            panic!("unsupported file ending");
        };

        assert_eq!(
            expected_result_area.primitives.len(),
            result_area.primitives.len()
        );

        assert!((&result_area).is_roughly_within(
            &expected_result_area,
            THICKNESS,
        ));
    }
}