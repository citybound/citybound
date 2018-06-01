use {P2, V2, N, THICKNESS, RoughEq, VecLike};
use curves::{Segment, Curve, FiniteCurve, Circle};
use path::Path;
use intersect::{Intersect, IntersectionResult};
use ordered_float::OrderedFloat;

#[derive(Debug)]
pub struct UnclosedPathError;

pub trait PointContainer {
    fn location_of(&self, point: P2) -> AreaLocation;

    fn contains(&self, point: P2) -> bool {
        self.location_of(point) != AreaLocation::Outside
    }
}

// represents a filled area bounded by a clockwise boundary
// everything "right of" the boundary is considered "inside"
#[derive(Clone)]
#[cfg_attr(feature = "compact_containers", derive(Compact))]
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

    pub fn fully_contains(&self, other: &PrimitiveArea) -> bool {
        let n_intersections = match (&self.boundary, &other.boundary).intersect() {
            IntersectionResult::Apart => 0,
            IntersectionResult::Intersecting(intersections) => intersections.len(),
            IntersectionResult::Coincident => unreachable!(),
        };

        n_intersections <= 1 &&
            other.boundary.segments.iter().all(|other_segment| {
                self.contains(other_segment.start())
            })
    }
}

impl PointContainer for PrimitiveArea {
    fn location_of(&self, point: P2) -> AreaLocation {
        if self.boundary.includes(point) {
            AreaLocation::Boundary
        } else {
            let ray = Segment::line(point, P2::new(point.x + 10_000_000_000.0, point.y))
                .expect("Ray should be valid");

            let n_intersections = match (
                &Path::new_unchecked(Some(ray).into_iter().collect()),
                &self.boundary,
            ).intersect() {
                IntersectionResult::Intersecting(intersections) => intersections.len(),
                IntersectionResult::Apart => 0,
                IntersectionResult::Coincident => unreachable!(),
            };

            if n_intersections % 2 == 1 {
                AreaLocation::Inside
            } else {
                AreaLocation::Outside
            }
        }
    }
}

impl<'a> RoughEq for &'a PrimitiveArea {
    fn rough_eq_by(&self, other: Self, tolerance: N) -> bool {
        (&self.boundary).rough_eq_by(&other.boundary, tolerance)
    }
}

#[derive(PartialEq)]
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

#[derive(Clone)]
#[cfg_attr(feature = "compact_containers", derive(Compact))]
pub struct Area {
    pub primitives: VecLike<PrimitiveArea>,
}

impl Area {
    pub fn new(primitives: VecLike<PrimitiveArea>) -> Self {
        Area { primitives }
    }

    pub fn new_simple(boundary: Path) -> Result<Self, UnclosedPathError> {
        Ok(Area {
            primitives: Some(PrimitiveArea::new(boundary)?).into_iter().collect(),
        })
    }

    pub fn split(&self, b: &Self) -> AreaSplitResult {
        let ab = [self, b];

        let mut intersection_distances = [
            vec![Vec::<N>::new(); self.primitives.len()],
            vec![Vec::<N>::new(); b.primitives.len()],
        ];

        for (i_a, primitive_a) in self.primitives.iter().enumerate() {
            for (i_b, primitive_b) in b.primitives.iter().enumerate() {
                if let IntersectionResult::Intersecting(intersections) =
                    (&primitive_a.boundary, &primitive_b.boundary).intersect()
                {
                    for intersection in intersections {
                        intersection_distances[SUBJECT_A][i_a].push(intersection.along_a);
                        intersection_distances[SUBJECT_B][i_b].push(intersection.along_b);
                    }
                }
            }
        }

        let boundary_pieces = SUBJECTS
            .iter()
            .flat_map(|&subject| {

                for (primitive_i, primitive_distances) in
                    intersection_distances[subject].iter_mut().enumerate()
                {
                    if primitive_distances.len() <= 1 {
                        primitive_distances.clear();
                        primitive_distances.push(0.0);
                        primitive_distances.push(
                            ab[subject].primitives[primitive_i]
                                .boundary
                                .length(),
                        );
                    } else {
                        primitive_distances.sort_unstable_by_key(|&along| OrderedFloat(along));

                        primitive_distances.dedup_by(|a, b| (*b - *a).abs() < THICKNESS);

                        // to close the loop when taking piece-cutting windows
                        let first = primitive_distances[0];
                        primitive_distances.push(first);
                    }
                }

                let mut boundary_pieces_initial = intersection_distances[subject]
                    .iter()
                    .enumerate()
                    .flat_map(|(primitive_i, primitive_distances)| {
                        primitive_distances
                            .windows(2)
                            .filter_map(|distance_pair| if let Some(subsection) =
                                ab[subject].primitives[primitive_i].boundary.subsection(
                                    distance_pair[0],
                                    distance_pair[1],
                                )
                            {
                                Some(BoundaryPiece {
                                    path: subsection,
                                    left_inside: [false, false],
                                    right_inside: [subject == SUBJECT_A, subject == SUBJECT_B],
                                })
                            } else {
                                None
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();

                for boundary_piece in &mut boundary_pieces_initial {
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
                            // there will be a coincident boundary piece
                            // we will merge inside info in uniqueness step
                        }
                    }
                }

                boundary_pieces_initial
            })
            .collect::<Vec<_>>();

        let mut unique_boundary_pieces = VecLike::<BoundaryPiece>::new();

        for boundary_piece in boundary_pieces {
            let found_merge = {
                // TODO: detect if several pieces are equivalent to one longer one
                //       - maybe we need to simplify paths sometimes to prevent this?
                // TODO: any way to not make this O(n^2) ?
                let maybe_equivalent = unique_boundary_pieces
                    .iter_mut()
                    .map(|other_piece| {
                        let forward_equivalent =
                            other_piece.path.start().rough_eq_by(
                                boundary_piece.path.start(),
                                THICKNESS,
                            ) &&
                                other_piece.path.end().rough_eq_by(
                                    boundary_piece.path.end(),
                                    THICKNESS,
                                ) &&
                                other_piece.path.midpoint().rough_eq_by(
                                    boundary_piece.path.midpoint(),
                                    THICKNESS,
                                );
                        let backward_equivalent =
                            other_piece.path.start().rough_eq_by(
                                boundary_piece.path.end(),
                                THICKNESS,
                            ) &&
                                other_piece.path.end().rough_eq_by(
                                    boundary_piece.path.start(),
                                    THICKNESS,
                                ) &&
                                other_piece.path.midpoint().rough_eq_by(
                                    boundary_piece.path.midpoint(),
                                    THICKNESS,
                                );
                        (other_piece, forward_equivalent, backward_equivalent)
                    })
                    .find(|(_, forward_equivalent, backward_equivalent)| {
                        *forward_equivalent || *backward_equivalent
                    });


                if let Some((equivalent_piece, forward_equivalent, _)) = maybe_equivalent {
                    if forward_equivalent {
                        equivalent_piece.left_inside[SUBJECT_A] |= boundary_piece.left_inside
                            [SUBJECT_A];
                        equivalent_piece.left_inside[SUBJECT_B] |= boundary_piece.left_inside
                            [SUBJECT_B];
                        equivalent_piece.right_inside[SUBJECT_A] |= boundary_piece.right_inside
                            [SUBJECT_A];
                        equivalent_piece.right_inside[SUBJECT_B] |= boundary_piece.right_inside
                            [SUBJECT_B];
                    } else {
                        equivalent_piece.left_inside[SUBJECT_A] |= boundary_piece.right_inside
                            [SUBJECT_A];
                        equivalent_piece.left_inside[SUBJECT_B] |= boundary_piece.right_inside
                            [SUBJECT_B];
                        equivalent_piece.right_inside[SUBJECT_A] |= boundary_piece.left_inside
                            [SUBJECT_A];
                        equivalent_piece.right_inside[SUBJECT_B] |= boundary_piece.left_inside
                            [SUBJECT_B];
                    }
                    true
                } else {
                    false
                }
            };

            if !found_merge {
                unique_boundary_pieces.push(boundary_piece);
            }
        }

        AreaSplitResult { pieces: unique_boundary_pieces }
    }

    pub fn disjoint(&self) -> Vec<Area> {
        // TODO: this is not quite correct yet
        let mut groups = Vec::<VecLike<PrimitiveArea>>::new();

        for primitive in self.primitives.iter().cloned() {
            if let Some(surrounding_group_i) =
                groups.iter().position(
                    |group| group[0].fully_contains(&primitive),
                )
            {
                groups[surrounding_group_i].push(primitive);
            } else if let Some(surrounded_group_i) =
                groups.iter().position(
                    |group| primitive.fully_contains(&group[0]),
                )
            {
                groups[surrounded_group_i].insert(0, primitive);
            } else {
                groups.push(Some(primitive).into_iter().collect());
            }
        }

        groups.into_iter().map(Area::new).collect()
    }
}

impl PointContainer for Area {
    fn location_of(&self, point: P2) -> AreaLocation {
        let point_on_primitive = self.primitives.iter().any(|primitive| {
            primitive.boundary.includes(point)
        });
        if point_on_primitive {
            AreaLocation::Boundary
        } else {
            let ray = Segment::line(point, P2::new(point.x + 10_000_000_000.0, point.y))
                .expect("Ray should be valid");

            // TODO: allow for ccw holes by checking intersection direction
            let mut n_intersections = 0;

            for primitive in &self.primitives {
                n_intersections += match (
                    &Path::new_unchecked(Some(ray).into_iter().collect()),
                    &primitive.boundary,
                ).intersect() {
                    IntersectionResult::Intersecting(intersections) => intersections.len(),
                    IntersectionResult::Apart => 0,
                    IntersectionResult::Coincident => unreachable!(),
                };
            }

            if n_intersections % 2 == 1 {
                AreaLocation::Inside
            } else {
                AreaLocation::Outside
            }
        }
    }
}

impl<'a> RoughEq for &'a Area {
    fn rough_eq_by(&self, other: Self, tolerance: N) -> bool {
        self.primitives.len() == other.primitives.len() &&
            self.primitives.iter().all(|own_primitive| {
                other.primitives.iter().any(|other_primitive| {
                    own_primitive.rough_eq_by(other_primitive, tolerance)
                })
            })
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "compact_containers", derive(Compact))]
pub struct BoundaryPiece {
    path: Path,
    left_inside: [bool; 2],
    right_inside: [bool; 2],
}

#[derive(Clone)]
#[cfg_attr(feature = "compact_containers", derive(Compact))]
pub struct AreaSplitResult {
    pieces: VecLike<BoundaryPiece>,
}

pub enum PieceRole {
    Forward,
    Backward,
    NonContributing,
}

impl AreaSplitResult {
    pub fn get_area<F: Fn(&BoundaryPiece) -> PieceRole>(&self, piece_filter: F) -> Area {
        const WELDING_TOLERANCE: N = THICKNESS * 30.0;

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
                if path.end().rough_eq_by(
                    oriented_path.start(),
                    WELDING_TOLERANCE,
                )
                {
                    maybe_path_before = Some(path_i)
                }
                if path.start().rough_eq_by(
                    oriented_path.end(),
                    WELDING_TOLERANCE,
                )
                {
                    maybe_path_after = Some(path_i)
                }
            }

            match (maybe_path_before, maybe_path_after) {
                (Some(before_i), Some(after_i)) => {
                    if before_i == after_i {
                        let joined_path = paths[before_i]
                            .concat_weld(&oriented_path, WELDING_TOLERANCE)
                            .expect("Concat must work at this point (J1)");

                        paths.remove(before_i);
                        complete_paths.push(joined_path);
                    } else {
                        let joined_path = paths[before_i]
                            .concat_weld(&oriented_path, WELDING_TOLERANCE)
                            .expect("Concat must work at this point (J2)")
                            .concat_weld(&paths[after_i], WELDING_TOLERANCE)
                            .expect("Concat must work at this point (J3)");

                        paths.remove(before_i.max(after_i));
                        paths[before_i.min(after_i)] = joined_path;
                    }
                }
                (Some(before_i), None) => {
                    let extended_path = paths[before_i]
                        .concat_weld(&oriented_path, WELDING_TOLERANCE)
                        .expect("Concat must work at this point (B)");

                    paths[before_i] = extended_path;
                }
                (None, Some(after_i)) => {
                    let extended_path = oriented_path
                        .concat_weld(&paths[after_i], WELDING_TOLERANCE)
                        .expect("Concat must work at this point (A)");

                    paths[after_i] = extended_path;
                }
                (None, None) => {
                    if oriented_path.is_closed() {
                        complete_paths.push(oriented_path)
                    } else {
                        paths.push(oriented_path)
                    }
                }
            }
        }


        if !paths.is_empty() {
            println!("{} left over paths", paths.len());
            println!("{}", self.debug_svg());
            for path in &paths {
                println!(
                    r#"<path d="{}" stroke="rgba(0, 255, 0, 0.8)"/>"#,
                    path.to_svg()
                );
            }

            for path in &paths {
                println!(
                    "Start to closest end: {}",
                    paths
                        .iter()
                        .map(|other| OrderedFloat((path.start() - other.end()).norm()))
                        .min()
                        .expect("should have a min")
                );
            }
            assert!(paths.is_empty());
        }

        Area::new(
            complete_paths
                .into_iter()
                .map(|path| PrimitiveArea::new(path).unwrap())
                .collect(),
        )
    }

    pub fn intersection(&self) -> Area {
        self.get_area(|piece| if piece.right_inside[SUBJECT_A] &&
            piece.right_inside[SUBJECT_B]
        {
            PieceRole::Forward
        } else {
            PieceRole::NonContributing
        })
    }

    pub fn union(&self) -> Area {
        self.get_area(
            |piece| if (piece.right_inside[SUBJECT_A] || piece.right_inside[SUBJECT_B]) &&
                !(piece.left_inside[SUBJECT_A] || piece.left_inside[SUBJECT_B])
            {
                PieceRole::Forward
            } else {
                PieceRole::NonContributing
            },
        )
    }

    pub fn a_minus_b(&self) -> Area {
        self.get_area(|piece| if piece.right_inside[SUBJECT_A] &&
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
        self.get_area(|piece| if piece.right_inside[SUBJECT_B] &&
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
                        side_paths.push((-1.0, "rgba(0, 0, 255, 0.3)"));
                    }

                    if piece.left_inside[SUBJECT_B] {
                        side_paths.push((-1.0, "rgba(255, 0, 0, 0.3)"));
                    }

                    if piece.right_inside[SUBJECT_A] {
                        side_paths.push((1.0, "rgba(0, 0, 255, 0.3)"));
                    }

                    if piece.right_inside[SUBJECT_B] {
                        side_paths.push((1.0, "rgba(255, 0, 0, 0.3)"));
                    }

                    side_paths.into_iter().flat_map(|(shift, color)|
                        piece.path.segments.iter().filter_map(|segment|
                            segment.shift_orthogonally(shift)
                        ).map(|segment|
                            format!(
                            r#"<path d="{}" stroke="{}"/>"#,
                            Path::new_unchecked(Some(segment).into_iter().collect()).to_svg(),
                            color
                        )).collect::<Vec<_>>()
                    ).collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
                .join(" "),
        )
    }
}



pub trait AsArea {
    fn as_area(&self) -> Area;
}

#[derive(Clone)]
#[cfg_attr(feature = "compact_containers", derive(Compact))]
pub struct Band {
    pub path: Path,
    pub width_left: N,
    pub width_right: N,
}

impl Band {
    pub fn new(path: Path, width: N) -> Band {
        Band {
            path,
            width_left: width / 2.0,
            width_right: width / 2.0,
        }
    }

    pub fn new_asymmetric(path: Path, width_left: N, width_right: N) -> Band {
        Band { path, width_left, width_right }
    }

    pub fn outline(&self) -> Path {
        let left_path = self.path
            .shift_orthogonally(-self.width_left)
            .unwrap_or_else(|| self.path.clone());
        let right_path = self.path
            .shift_orthogonally(self.width_right)
            .unwrap_or_else(|| self.path.clone())
            .reverse();

        let end_connector = Segment::line(left_path.end(), right_path.start());
        let start_connector = Segment::line(right_path.end(), left_path.start());

        Path::new(
            left_path
                .segments
                .into_iter()
                .chain(end_connector)
                .chain(right_path.segments.into_iter())
                .chain(start_connector)
                .collect(),
        ).expect("Band path should always be valid")
    }

    pub fn outline_distance_to_path_distance(&self, distance: N) -> N {
        let full_width = self.width_left + self.width_right;

        if let (Some(left_path_length), Some(right_path_length)) =
            (
                self.path.shift_orthogonally(-self.width_left).map(
                    |p| p.length(),
                ),
                self.path.shift_orthogonally(self.width_right).map(
                    |p| p.length(),
                ),
            )
        {
            if distance > left_path_length + full_width + right_path_length {
                // on connector2
                0.0
            } else if distance > left_path_length + full_width {
                // on right side
                (1.0 - (distance - left_path_length - full_width) / right_path_length) *
                    self.path.length()
            } else if distance > left_path_length {
                // on connector1
                self.path.length()
            } else {
                // on left side
                (distance / left_path_length) * self.path.length()
            }
        } else {
            distance
        }
    }
}

impl AsArea for Band {
    fn as_area(&self) -> Area {
        Area::new_simple(self.outline()).expect("Band boundary should always be valid")
    }
}

impl AsArea for Circle {
    fn as_area(&self) -> Area {
        let top = self.center + V2::new(0.0, self.radius);
        let bottom = self.center + V2::new(0.0, -self.radius);
        let right_segment = Segment::arc_with_direction(top, V2::new(1.0, 0.0), bottom)
            .expect("Circle too small");
        let left_segment = Segment::arc_with_direction(bottom, V2::new(-1.0, 0.0), top)
            .expect("Circle too small");

        Area::new_simple(Path::new_unchecked(
            Some(right_segment)
                .into_iter()
                .chain(Some(left_segment))
                .collect(),
        )).expect("Circle is always closed")
    }
}

#[cfg(test)]
mod tests;
