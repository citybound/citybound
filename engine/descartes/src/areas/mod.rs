use {P2, V2, N, THICKNESS, RoughEq, VecLike, PI};
use curves::{Segment, Curve, FiniteCurve, Circle};
use path::Path;
use intersect::{Intersect, IntersectionResult};
use ordered_float::OrderedFloat;

const EQUIVALENCE_TOLERANCE: N = THICKNESS * 10.0;

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

        n_intersections <= 1
            && other
                .boundary
                .segments
                .iter()
                .all(|other_segment| self.contains(other_segment.start()))
    }

    pub fn winding_number(&self, point: P2) -> f32 {
        (self
            .boundary
            .segments
            .iter()
            .map(|segment| segment.winding_angle(point))
            .sum::<f32>() / (2.0 * PI))
            .round()
    }
}

impl PointContainer for PrimitiveArea {
    fn location_of(&self, point: P2) -> AreaLocation {
        if self.boundary.includes(point) {
            AreaLocation::Boundary
        } else if self.winding_number(point) == 0.0 {
            AreaLocation::Outside
        } else {
            AreaLocation::Inside
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
                let mut unintersected_pieces = Vec::new();

                for (primitive_i, primitive_distances) in
                    intersection_distances[subject].iter_mut().enumerate()
                {
                    if primitive_distances.len() <= 1 {
                        primitive_distances.clear();
                        unintersected_pieces
                            .push(ab[subject].primitives[primitive_i].boundary.clone())
                    } else {
                        primitive_distances.sort_unstable_by_key(|&along| OrderedFloat(along));

                        primitive_distances.dedup_by(|a, b| (*b - *a).abs() < THICKNESS);

                        // to close the loop when taking piece-cutting windows
                        let first = primitive_distances[0];
                        primitive_distances.push(first);
                    }
                }

                let mut boundary_pieces_initial = unintersected_pieces
                    .into_iter()
                    .chain(intersection_distances[subject].iter().enumerate().flat_map(
                        |(primitive_i, primitive_distances)| {
                            primitive_distances
                                .windows(2)
                                .filter_map(|distance_pair| {
                                    ab[subject].primitives[primitive_i]
                                        .boundary
                                        .subsection(distance_pair[0], distance_pair[1])
                                })
                                .collect::<Vec<_>>()
                        },
                    ))
                    .map(|path| BoundaryPiece {
                        path,
                        left_inside: [false, false],
                        right_inside: [subject == SUBJECT_A, subject == SUBJECT_B],
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
                //       - wait, this should never happen?
                // TODO: any way to not make this O(n^2) ?
                let maybe_equivalent = unique_boundary_pieces
                    .iter_mut()
                    .map(|other_piece| {
                        let forward_equivalent = other_piece
                            .path
                            .start()
                            .rough_eq_by(boundary_piece.path.start(), EQUIVALENCE_TOLERANCE)
                            && other_piece
                                .path
                                .end()
                                .rough_eq_by(boundary_piece.path.end(), EQUIVALENCE_TOLERANCE)
                            && other_piece
                                .path
                                .midpoint()
                                .rough_eq_by(boundary_piece.path.midpoint(), EQUIVALENCE_TOLERANCE);
                        let backward_equivalent = other_piece
                            .path
                            .start()
                            .rough_eq_by(boundary_piece.path.end(), EQUIVALENCE_TOLERANCE)
                            && other_piece
                                .path
                                .end()
                                .rough_eq_by(boundary_piece.path.start(), EQUIVALENCE_TOLERANCE)
                            && other_piece
                                .path
                                .midpoint()
                                .rough_eq_by(boundary_piece.path.midpoint(), EQUIVALENCE_TOLERANCE);
                        (other_piece, forward_equivalent, backward_equivalent)
                    })
                    .find(|(_, forward_equivalent, backward_equivalent)| {
                        *forward_equivalent || *backward_equivalent
                    });

                if let Some((equivalent_piece, forward_equivalent, _)) = maybe_equivalent {
                    if forward_equivalent {
                        equivalent_piece.left_inside[SUBJECT_A] |=
                            boundary_piece.left_inside[SUBJECT_A];
                        equivalent_piece.left_inside[SUBJECT_B] |=
                            boundary_piece.left_inside[SUBJECT_B];
                        equivalent_piece.right_inside[SUBJECT_A] |=
                            boundary_piece.right_inside[SUBJECT_A];
                        equivalent_piece.right_inside[SUBJECT_B] |=
                            boundary_piece.right_inside[SUBJECT_B];
                    } else {
                        equivalent_piece.left_inside[SUBJECT_A] |=
                            boundary_piece.right_inside[SUBJECT_A];
                        equivalent_piece.left_inside[SUBJECT_B] |=
                            boundary_piece.right_inside[SUBJECT_B];
                        equivalent_piece.right_inside[SUBJECT_A] |=
                            boundary_piece.left_inside[SUBJECT_A];
                        equivalent_piece.right_inside[SUBJECT_B] |=
                            boundary_piece.left_inside[SUBJECT_B];
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

        AreaSplitResult {
            pieces: unique_boundary_pieces,
        }
    }

    pub fn disjoint(&self) -> Vec<Area> {
        // TODO: this is not quite correct yet
        let mut groups = Vec::<VecLike<PrimitiveArea>>::new();

        for primitive in self.primitives.iter().cloned() {
            if let Some(surrounding_group_i) = groups
                .iter()
                .position(|group| group[0].fully_contains(&primitive))
            {
                groups[surrounding_group_i].push(primitive);
            } else if let Some(surrounded_group_i) = groups
                .iter()
                .position(|group| primitive.fully_contains(&group[0]))
            {
                groups[surrounded_group_i].insert(0, primitive);
            } else {
                groups.push(Some(primitive).into_iter().collect());
            }
        }

        groups.into_iter().map(Area::new).collect()
    }

    fn winding_number(&self, point: P2) -> f32 {
        self.primitives
            .iter()
            .map(|primitive| primitive.winding_number(point))
            .sum()
    }
}

impl PointContainer for Area {
    fn location_of(&self, point: P2) -> AreaLocation {
        if self
            .primitives
            .iter()
            .any(|primtive| primtive.boundary.includes(point))
        {
            AreaLocation::Boundary
        } else if self.winding_number(point) == 0.0 {
            AreaLocation::Outside
        } else {
            AreaLocation::Inside
        }
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

#[derive(Clone, Debug)]
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

fn join_within_vec<I, E, F: Fn(&I, &I) -> Result<Option<I>, E>>(
    vector: &mut Vec<I>,
    joiner: F,
) -> Result<(), E> {
    // do-until
    while {
        let mut could_join = false;

        let mut i = 0;

        while i + 1 < vector.len() {
            let mut j = i + 1;

            while j < vector.len() {
                if let Some(joined) = joiner(&vector[i], &vector[j])? {
                    vector[i] = joined;
                    vector.swap_remove(j);
                    could_join = true;
                } else {
                    j += 1;
                }
            }

            i += 1;
        }

        could_join
    } {}

    Ok(())
}

use path::PathError;

#[derive(Debug)]
pub enum AreaError {
    ABWeldingShouldWork(PathError),
    BAWeldingShouldWork(PathError),
    LeftOver(String),
}

impl AreaSplitResult {
    pub fn get_area<F: Fn(&BoundaryPiece) -> PieceRole>(
        &self,
        piece_filter: F,
    ) -> Result<Area, AreaError> {
        let mut paths = self
            .pieces
            .iter()
            .filter_map(|piece| match piece_filter(piece) {
                PieceRole::Forward => Some(piece.path.clone()),
                PieceRole::Backward => Some(piece.path.reverse()),
                PieceRole::NonContributing => None,
            })
            .collect::<Vec<_>>();
        let mut complete_paths = Vec::<Path>::new();

        let mut combining_tolerance = THICKNESS;

        while !paths.is_empty() && combining_tolerance < 1.0 {
            join_within_vec(&mut paths, |path_a, path_b| {
                if path_b
                    .start()
                    .rough_eq_by(path_a.end(), combining_tolerance)
                {
                    Ok(Some(
                        path_a
                            .concat_weld(&path_b, combining_tolerance)
                            .map_err(AreaError::ABWeldingShouldWork)?,
                    ))
                } else if path_b
                    .end()
                    .rough_eq_by(path_a.start(), combining_tolerance)
                {
                    Ok(Some(
                        path_b
                            .concat_weld(&path_a, combining_tolerance)
                            .map_err(AreaError::BAWeldingShouldWork)?,
                    ))
                } else {
                    Ok(None)
                }
            })?;

            paths.retain(|path| {
                if path.length() < combining_tolerance {
                    false
                } else if path.is_closed() {
                    complete_paths.push(path.clone());
                    false
                } else if (path.start() - path.end()).norm() <= combining_tolerance {
                    if let Ok(welded_path) = path.with_new_start_and_end(path.start(), path.start())
                    {
                        complete_paths.push(welded_path);
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            });

            combining_tolerance *= 2.0;
        }

        if !paths.is_empty() {
            let min_distance = paths
                .iter()
                .map(|other| OrderedFloat((paths[0].start() - other.end()).norm()))
                .min()
                .expect("should have a min");

            return Err(AreaError::LeftOver(format!(
                "Start to closest end: {}\n{}\n\n{}",
                min_distance,
                self.debug_svg(),
                format!(
                    r#"<path d="{}" stroke="rgba(0, 255, 0, 0.8)"/>"#,
                    paths[0].to_svg()
                )
            )));
        }

        Ok(Area::new(
            complete_paths
                .into_iter()
                .map(|path| PrimitiveArea::new(path).unwrap())
                .collect(),
        ))
    }

    pub fn intersection(&self) -> Result<Area, AreaError> {
        self.get_area(|piece| {
            if piece.right_inside[SUBJECT_A] && piece.right_inside[SUBJECT_B] {
                PieceRole::Forward
            } else {
                PieceRole::NonContributing
            }
        })
    }

    pub fn union(&self) -> Result<Area, AreaError> {
        self.get_area(|piece| {
            if (piece.right_inside[SUBJECT_A] || piece.right_inside[SUBJECT_B])
                && !(piece.left_inside[SUBJECT_A] || piece.left_inside[SUBJECT_B])
            {
                PieceRole::Forward
            } else {
                PieceRole::NonContributing
            }
        })
    }

    pub fn a_minus_b(&self) -> Result<Area, AreaError> {
        self.get_area(|piece| {
            if piece.right_inside[SUBJECT_A] && !piece.right_inside[SUBJECT_B] {
                PieceRole::Forward
            } else if piece.left_inside[SUBJECT_A] && !piece.left_inside[SUBJECT_B] {
                PieceRole::Backward
            } else {
                PieceRole::NonContributing
            }
        })
    }

    pub fn b_minus_a(&self) -> Result<Area, AreaError> {
        self.get_area(|piece| {
            if piece.right_inside[SUBJECT_B] && !piece.right_inside[SUBJECT_A] {
                PieceRole::Forward
            } else if piece.left_inside[SUBJECT_B] && !piece.left_inside[SUBJECT_A] {
                PieceRole::Backward
            } else {
                PieceRole::NonContributing
            }
        })
    }

    pub fn debug_svg(&self) -> String {
        let piece_points = self
            .pieces
            .iter()
            .flat_map(|piece| vec![piece.path.start(), piece.path.midpoint(), piece.path.end()])
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
                .map(|piece| format!(r#"<path d="{}"/>"#, piece.path.to_svg()))
                .collect::<Vec<_>>()
                .join(" "),
            stroke_width,
            self.pieces
                .iter()
                .flat_map(|piece| {
                    let mut side_paths = vec![];

                    if piece.left_inside[SUBJECT_A] {
                        side_paths.push((-stroke_width, "rgba(0, 0, 255, 0.3)"));
                    }

                    if piece.left_inside[SUBJECT_B] {
                        side_paths.push((-stroke_width, "rgba(255, 0, 0, 0.3)"));
                    }

                    if piece.right_inside[SUBJECT_A] {
                        side_paths.push((stroke_width, "rgba(0, 0, 255, 0.3)"));
                    }

                    if piece.right_inside[SUBJECT_B] {
                        side_paths.push((stroke_width, "rgba(255, 0, 0, 0.3)"));
                    }

                    side_paths
                        .into_iter()
                        .flat_map(|(shift, color)| {
                            piece
                                .path
                                .segments
                                .iter()
                                .filter_map(|segment| segment.shift_orthogonally(shift))
                                .map(|segment| {
                                    format!(
                                        r#"<path d="{}" stroke="{}"/>"#,
                                        Path::new_unchecked(Some(segment).into_iter().collect())
                                            .to_svg(),
                                        color
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>()
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
        Band {
            path,
            width_left,
            width_right,
        }
    }

    pub fn outline(&self) -> Path {
        let left_path = self
            .path
            .shift_orthogonally(-self.width_left)
            .unwrap_or_else(|| self.path.clone());
        let right_path = self
            .path
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

        if let (Some(left_path_length), Some(right_path_length)) = (
            self.path
                .shift_orthogonally(-self.width_left)
                .map(|p| p.length()),
            self.path
                .shift_orthogonally(self.width_right)
                .map(|p| p.length()),
        ) {
            if distance > left_path_length + full_width + right_path_length {
                // on connector2
                0.0
            } else if distance > left_path_length + full_width {
                // on right side
                (1.0 - (distance - left_path_length - full_width) / right_path_length)
                    * self.path.length()
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
        let right_segment =
            Segment::arc_with_direction(top, V2::new(1.0, 0.0), bottom).expect("Circle too small");
        let left_segment =
            Segment::arc_with_direction(bottom, V2::new(-1.0, 0.0), top).expect("Circle too small");

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
