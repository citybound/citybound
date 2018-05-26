use {P2, N, THICKNESS, RoughlyComparable};
use curves::{Segment, Curve, FiniteCurve};
use path::Path;
use intersect::Intersect;
use ordered_float::OrderedFloat;

pub struct UnclosedPathError;

// represents a filled area bounded by a clockwise boundary
// everything "right of" the boundary is considered "inside"
pub trait PrimitiveArea: Sized {
    type P: Path;
    fn boundary(&self) -> &Self::P;
    fn new_unchecked(boundary: Self::P) -> Self;
    fn new(boundary: Self::P) -> Result<Self, UnclosedPathError> {
        if boundary.is_closed() {
            Ok(Self::new_unchecked(boundary))
        } else {
            Err(UnclosedPathError)
        }
    }

    fn location_of(&self, point: P2) -> AreaLocation {
        if self.boundary().contains(point) {
            AreaLocation::Boundary
        } else {
            let ray = Segment::line(point, P2::new(point.x + 10_000_000_000.0, point.y))
                .expect("Ray should be valid");

            let n_intersections = (&Self::P::new_unchecked(vec![ray]), self.boundary())
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

pub trait Area: Sized {
    type PA: PrimitiveArea;
    fn primitives(&self) -> &[Self::PA];
    fn new(primitives: Vec<Self::PA>) -> Self;

    fn location_of(&self, point: P2) -> AreaLocation {
        if self.primitives().iter().any(|primitive| {
            primitive.boundary().contains(point)
        })
        {
            AreaLocation::Boundary
        } else {
            let ray = Segment::line(point, P2::new(point.x + 10_000_000_000.0, point.y))
                .expect("Ray should be valid");

            // TODO: allow for ccw holes by checking intersection direction
            let mut n_intersections = 0;

            for primitive in self.primitives() {
                n_intersections += (
                    &<Self::PA as PrimitiveArea>::P::new_unchecked(vec![ray]),
                    primitive.boundary(),
                ).intersect()
                    .len();
            }

            if n_intersections % 2 == 1 {
                AreaLocation::Inside
            } else {
                AreaLocation::Outside
            }
        }
    }

    fn split(a: &Self, b: &Self) -> AreaSplitResult<Self> {
        let ab = [a, b];

        let mut intersection_distances = [Vec::<(usize, N)>::new(), Vec::<(usize, N)>::new()];

        for (i_a, primitive_a) in a.primitives().iter().enumerate() {
            for (i_b, primitive_b) in b.primitives().iter().enumerate() {
                for intersection in (primitive_a.boundary(), primitive_b.boundary()).intersect() {
                    intersection_distances[SUBJECT_A].push((i_a, intersection.along_a));
                    intersection_distances[SUBJECT_B].push((i_b, intersection.along_b));
                }
            }
        }

        let boundary_pieces = SUBJECTS.iter().flat_map(|&subject| {

            // sort first by distance
            intersection_distances[subject].sort_unstable_by_key(
                |&(_i, along)| {
                    OrderedFloat(along)
                },
            );
            // and then stably sort by primitive index to end up with a list of
            // continous intersection distances chunked by primitive
            intersection_distances[subject].sort_by_key(|&(i, _along)| i);

            let mut boundary_pieces_initial = intersection_distances[subject]
                .windows(2)
                .filter_map(|distance_pair| {
                    let (i_0, along_0) = distance_pair[0];
                    let (i_1, along_1) = distance_pair[1];

                    if i_0 == i_1 {
                        ab[subject].primitives()[i_0].boundary().subsection(
                            along_0,
                            along_1,
                        )
                    } else {
                        None
                    }
                })
                .map(|subsection| {
                    BoundaryPiece {
                        path: subsection,
                        left_inside: [false, false],
                        right_inside: [subject == SUBJECT_A, subject == SUBJECT_B],
                    }
                }).collect::<Vec<_>>();

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

                        let coincident_boundary = ab[other_subject(subject)].primitives().iter()
                            .map(|primitive| primitive.boundary())
                            .find(|boundary| boundary.contains(midpoint))
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

pub struct BoundaryPiece<P: Path> {
    path: P,
    left_inside: [bool; 2],
    right_inside: [bool; 2],
}

pub struct AreaSplitResult<A: Area> {
    pieces: Vec<BoundaryPiece<<A::PA as PrimitiveArea>::P>>,
}

pub enum PieceRole {
    Forward,
    Backward,
    NonContributing,
}

impl<A: Area> AreaSplitResult<A> {
    fn into_area<F: Fn(&BoundaryPiece<<A::PA as PrimitiveArea>::P>) -> PieceRole>(
        &self,
        piece_filter: F,
    ) -> A {
        let mut paths = Vec::<<A::PA as PrimitiveArea>::P>::new();

        for oriented_path in self.pieces.iter().filter_map(
            |piece| match piece_filter(piece) {
                PieceRole::Forward => Some(piece.path.clone()),
                PieceRole::Backward => Some(piece.path.reverse()),
                PieceRole::NonContributing => None,
            },
        )
        {
            let mut found_concat_partner = false;

            for path in &mut paths {
                if path.end().is_roughly_within(
                    oriented_path.start(),
                    THICKNESS,
                )
                {
                    *path = path.concat(&oriented_path).expect(
                        "already checked concatability",
                    );
                    found_concat_partner = true;
                    break;
                } else if path.start().is_roughly_within(
                    oriented_path.end(),
                    THICKNESS,
                )
                {
                    *path = oriented_path.concat(path).expect(
                        "already checked concatability",
                    );
                    found_concat_partner = true;
                    break;
                }
            }
            if !found_concat_partner {
                paths.push(oriented_path);
            }
        }

        A::new(
            paths
                .into_iter()
                .filter_map(|path| A::PA::new(path).ok())
                .collect(),
        )
    }

    fn intersection(&self) -> A {
        self.into_area(|piece| if piece.right_inside[SUBJECT_A] &&
            piece.right_inside[SUBJECT_B]
        {
            PieceRole::Forward
        } else {
            PieceRole::NonContributing
        })
    }

    fn union(&self) -> A {
        self.into_area(|piece| if piece.right_inside[SUBJECT_A] ^
            piece.right_inside[SUBJECT_B]
        {
            PieceRole::Forward
        } else {
            PieceRole::NonContributing
        })
    }

    fn a_minus_b(&self) -> A {
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
}