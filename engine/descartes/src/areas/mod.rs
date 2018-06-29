use {P2, N, THICKNESS, RoughEq, VecLike, PI, signed_angle_to};
use line_path::{LinePath, LineSegment};
use closed_line_path::{ClosedLinePath};
use intersect::{Intersect};
use ordered_float::OrderedFloat;

mod debug;

#[cfg(test)]
mod tests;

const EQUIVALENCE_TOLERANCE: N = THICKNESS * 10.0;

#[derive(Debug)]
pub struct UnclosedPathError;

#[derive(PartialEq)]
pub enum AreaLocation {
    Inside,
    Boundary,
    Outside,
}

pub trait PointContainer {
    fn location_of(&self, point: P2) -> AreaLocation;

    fn contains(&self, point: P2) -> bool {
        self.location_of(point) != AreaLocation::Outside
    }
}

impl LineSegment {
    fn winding_angle(&self, point: P2) -> N {
        signed_angle_to(self.start() - point, self.end() - point)
    }
}

/// Represents a filled area bounded by a clockwise boundary.
/// Everything "right of" the boundary is considered "inside"
#[derive(Clone)]
#[cfg_attr(feature = "compact_containers", derive(Compact))]
pub struct PrimitiveArea {
    pub boundary: ClosedLinePath,
}

impl PrimitiveArea {
    pub fn new(boundary: ClosedLinePath) -> PrimitiveArea {
        PrimitiveArea { boundary }
    }

    pub fn fully_contains(&self, other: &PrimitiveArea) -> bool {
        let n_intersections = (&self.boundary, &other.boundary).intersect().len();

        n_intersections <= 1
            && other
                .boundary
                .path()
                .segments()
                .all(|other_segment| self.contains(other_segment.start()))
    }

    pub fn winding_number(&self, point: P2) -> f32 {
        (self
            .boundary
            .path()
            .segments()
            .map(|segment| segment.winding_angle(point))
            .sum::<f32>() / (2.0 * PI))
            .round()
    }
}

impl PointContainer for PrimitiveArea {
    fn location_of(&self, point: P2) -> AreaLocation {
        if self.boundary.path().includes(point) {
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

#[derive(Clone)]
#[cfg_attr(feature = "compact_containers", derive(Compact))]
pub struct Area {
    pub primitives: VecLike<PrimitiveArea>,
}

impl Area {
    pub fn new(primitives: VecLike<PrimitiveArea>) -> Self {
        Area { primitives }
    }

    pub fn new_simple(boundary: ClosedLinePath) -> Self {
        Area {
            primitives: Some(PrimitiveArea::new(boundary)).into_iter().collect(),
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
            .any(|primtive| primtive.boundary.path().includes(point))
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

pub struct BoundaryPiece<'a> {
    on_boundary: &'a ClosedLinePath,
    start: P2,
    start_distance: N,
    end: P2,
    end_distance: N,
    midpoint: P2,
    left_inside: [bool; 2],
    right_inside: [bool; 2],
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum PieceEquivalence {
    Different,
    Forward,
    Backward,
}

impl<'a> BoundaryPiece<'a> {
    fn new(
        on_boundary: &'a ClosedLinePath,
        start: P2,
        start_distance: N,
        end: P2,
        end_distance: N,
        right_inside: [bool; 2],
    ) -> Option<Self> {
        if start_distance.rough_eq_by(end_distance, EQUIVALENCE_TOLERANCE) {
            None
        } else {
            Some(BoundaryPiece {
                on_boundary,
                start,
                start_distance,
                end,
                end_distance,
                right_inside,
                left_inside: [false, false],
                midpoint: on_boundary.midpoint_between(start_distance, end_distance),
            })
        }
    }

    fn new_unintersected(boundary: &'a ClosedLinePath, right_inside: [bool; 2]) -> Self {
        Self::new(
            boundary,
            boundary.path().start(),
            0.0,
            boundary.path().end(),
            boundary.path().length(),
            right_inside,
        ).expect("Unintersected pieces should always be valid")
    }

    fn to_path(&self) -> Option<LinePath> {
        self.on_boundary
            .subsection(self.start_distance, self.end_distance)
    }

    fn equivalence(&self, other: &Self) -> PieceEquivalence {
        let midpoints_eq = self
            .midpoint
            .rough_eq_by(other.midpoint, EQUIVALENCE_TOLERANCE);

        if midpoints_eq {
            if self.start.rough_eq_by(other.start, EQUIVALENCE_TOLERANCE)
                && self.end.rough_eq_by(other.end, EQUIVALENCE_TOLERANCE)
            {
                PieceEquivalence::Forward
            } else if self.start.rough_eq_by(other.end, EQUIVALENCE_TOLERANCE)
                && self.end.rough_eq_by(other.start, EQUIVALENCE_TOLERANCE)
            {
                PieceEquivalence::Backward
            } else {
                PieceEquivalence::Different
            }
        } else {
            PieceEquivalence::Different
        }
    }
}

pub struct AreaSplitResult<'a> {
    pieces: Vec<BoundaryPiece<'a>>,
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

impl Area {
    pub fn split<'a>(&'a self, b: &'a Self) -> AreaSplitResult<'a> {
        let ab = [self, b];

        let mut intersection_distances_points = [
            vec![Vec::<(N, P2)>::new(); self.primitives.len()],
            vec![Vec::<(N, P2)>::new(); b.primitives.len()],
        ];

        for (i_a, primitive_a) in self.primitives.iter().enumerate() {
            for (i_b, primitive_b) in b.primitives.iter().enumerate() {
                for intersection in (&primitive_a.boundary, &primitive_b.boundary).intersect() {
                    intersection_distances_points[SUBJECT_A][i_a]
                        .push((intersection.along_a, intersection.position));
                    intersection_distances_points[SUBJECT_B][i_b]
                        .push((intersection.along_b, intersection.position));
                }
            }
        }

        let boundary_pieces = SUBJECTS
            .iter()
            .flat_map(|&subject| {
                let mut unintersected_pieces = Vec::<BoundaryPiece>::new();

                for (primitive_i, primitive_distances_points) in intersection_distances_points
                    [subject]
                    .iter_mut()
                    .enumerate()
                {
                    if primitive_distances_points.len() <= 1 {
                        primitive_distances_points.clear();
                        unintersected_pieces.push(BoundaryPiece::new_unintersected(
                            &ab[subject].primitives[primitive_i].boundary,
                            [subject == SUBJECT_A, subject == SUBJECT_B],
                        ))
                    } else {
                        primitive_distances_points
                            .sort_unstable_by_key(|&(along, _)| OrderedFloat(along));

                        primitive_distances_points.dedup_by(|(along_a, _), (along_b, _)| {
                            (*along_b - *along_a).abs() < THICKNESS
                        });

                        // to close the loop when taking piece-cutting windows
                        let first = primitive_distances_points[0];
                        primitive_distances_points.push(first);
                    }
                }

                // println!(
                //     "INTERSECTION POINTS DISTANCES \n{:?}",
                //     intersection_distances_points
                // );

                let mut boundary_pieces_initial = unintersected_pieces
                    .into_iter()
                    .chain(
                        intersection_distances_points[subject]
                            .iter()
                            .enumerate()
                            .flat_map(|(primitive_i, primitive_distances_points)| {
                                primitive_distances_points
                                    .windows(2)
                                    .filter_map(|distance_point_pair| {
                                        let (start_distance, start) = distance_point_pair[0];
                                        let (end_distance, end) = distance_point_pair[1];

                                        BoundaryPiece::new(
                                            &ab[subject].primitives[primitive_i].boundary,
                                            start,
                                            start_distance,
                                            end,
                                            end_distance,
                                            [subject == SUBJECT_A, subject == SUBJECT_B],
                                        )
                                    })
                                    .collect::<Vec<_>>()
                            }),
                    )
                    .collect::<Vec<_>>();

                // println!(
                //     "BOUNDARY PIECES INITIAL \n{:#?}",
                //     boundary_pieces_initial
                //         .iter()
                //         .map(|piece| format!(
                //             "({}/{} - {} - {}/{}",
                //             piece.start,
                //             piece.start_distance,
                //             piece.midpoint,
                //             piece.end,
                //             piece.end_distance
                //         ))
                //         .collect::<Vec<_>>()
                // );

                for boundary_piece in &mut boundary_pieces_initial {
                    match ab[other_subject(subject)].location_of(boundary_piece.midpoint) {
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

        let mut unique_boundary_pieces = Vec::<BoundaryPiece>::new();

        for boundary_piece in boundary_pieces {
            let found_merge = {
                // TODO: detect if several pieces are equivalent to one longer one
                //       - maybe we need to simplify paths sometimes to prevent this?
                //       - wait, this should never happen?
                // TODO: any way to not make this O(n^2) ?
                let maybe_equivalent = unique_boundary_pieces
                    .iter_mut()
                    .map(|other_piece| (boundary_piece.equivalence(other_piece), other_piece))
                    .find(|(equivalence, _)| *equivalence != PieceEquivalence::Different);

                if let Some((equivalence, equivalent_piece)) = maybe_equivalent {
                    if equivalence == PieceEquivalence::Forward {
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

        // println!(
        //     "UNIQUE PIECES \n{:#?}",
        //     unique_boundary_pieces
        //         .iter()
        //         .map(|piece| format!(
        //             "({}/{} - {} - {}/{}, Path: {:?}",
        //             piece.start,
        //             piece.start_distance,
        //             piece.midpoint,
        //             piece.end,
        //             piece.end_distance,
        //             piece
        //                 .to_path()
        //                 .unwrap()
        //                 .points
        //                 .iter()
        //                 .map(|p| format!("{}", p))
        //                 .collect::<Vec<_>>()
        //         ))
        //         .collect::<Vec<_>>()
        // );

        AreaSplitResult {
            pieces: unique_boundary_pieces,
        }
    }
}

pub enum PieceRole {
    Forward,
    Backward,
    NonContributing,
}

fn join_within_vec<I, E, F: Fn(&I, &I) -> Result<Option<I>, E>>(
    items: &mut Vec<I>,
    joiner: F,
) -> Result<(), E> {
    // do-until
    while {
        let mut could_join = false;

        let mut i = 0;

        while i + 1 < items.len() {
            let mut j = i + 1;

            while j < items.len() {
                if let Some(joined) =
                    joiner(&items[i], &items[j])?.or(joiner(&items[j], &items[i])?)
                {
                    items[i] = joined;
                    items.swap_remove(j);
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

use line_path::ConcatError;

#[derive(Debug)]
pub enum AreaError {
    WeldingShouldWork(ConcatError),
    LeftOver(String),
}

impl<'a> AreaSplitResult<'a> {
    pub fn get_area<F: Fn(&BoundaryPiece<'a>) -> PieceRole>(
        &self,
        piece_filter: F,
    ) -> Result<Area, AreaError> {
        let mut paths = self
            .pieces
            .iter()
            .filter_map(|piece| match piece_filter(piece) {
                PieceRole::Forward => piece.to_path(),
                PieceRole::Backward => piece.to_path().map(|path| path.reverse()),
                PieceRole::NonContributing => None,
            })
            .collect::<Vec<_>>();

        // println!(
        //     "PATHS \n{:#?}",
        //     paths
        //         .iter()
        //         .map(|path| format!(
        //             "Path: {:?}",
        //             path.points
        //                 .iter()
        //                 .map(|p| format!("{}", p))
        //                 .collect::<Vec<_>>()
        //         ))
        //         .collect::<Vec<_>>()
        // );

        let mut complete_paths = Vec::<ClosedLinePath>::new();

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
                            .map_err(AreaError::WeldingShouldWork)?,
                    ))
                } else {
                    Ok(None)
                }
            })?;

            paths.retain(|path| {
                if path.length() < combining_tolerance {
                    false
                } else if let Some(closed) = ClosedLinePath::try_clone_from(path) {
                    complete_paths.push(closed);
                    false
                } else if (path.start() - path.end()).norm() <= combining_tolerance {
                    if let Some(welded_path) =
                        path.with_new_start_and_end(path.start(), path.start())
                    {
                        complete_paths.push(
                            ClosedLinePath::new(welded_path).expect("Welded should be closed"),
                        );
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

            return Err(AreaError::LeftOver(
                format!(
                    "Start to closest end: {}\n{}\n\n{}",
                    min_distance,
                    self.debug_svg(),
                    format!(
                        r#"<path d="{}" stroke="rgba(0, 255, 0, 0.8)"/>"#,
                        paths[0].to_svg()
                    )
                )
            ));
        }

        Ok(Area::new(
            complete_paths.into_iter().map(PrimitiveArea::new).collect(),
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
}
