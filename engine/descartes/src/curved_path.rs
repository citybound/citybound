use {N, P2, V2, VecLike, signed_angle_to, Rotation2};
use line_path::{LinePath, ConcatError, LineSegment};
use intersect::{Intersect, Intersection};
use rough_eq::{RoughEq, THICKNESS};
use angles::WithUniqueOrthogonal;

#[derive(Copy, Clone, Debug)]
pub enum CurvedSegment {
    Line(P2, P2),
    Arc(P2, P2, P2),
}

impl CurvedSegment {
    pub fn start(&self) -> P2 {
        match *self {
            CurvedSegment::Line(start, _) | CurvedSegment::Arc(start, ..) => start,
        }
    }

    pub fn end(&self) -> P2 {
        match *self {
            CurvedSegment::Line(_, end) | CurvedSegment::Arc(_, _, end) => end,
        }
    }

    pub fn start_direction(&self) -> V2 {
        match self {
            CurvedSegment::Line(start, end) => (end - start).normalize(),
            CurvedSegment::Arc(start, center, end) => {
                let center_to_start_orth = (start - center).orthogonal();
                center_to_start_orth * if center_to_start_orth.dot(&(end - start)) > 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }

    pub fn end_direction(&self) -> V2 {
        match self {
            CurvedSegment::Line(start, end) => (end - start).normalize(),
            CurvedSegment::Arc(start, center, end) => {
                let center_to_end_orth = (end - center).orthogonal();
                center_to_end_orth * if center_to_end_orth.dot(&(end - start)) > 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }

    pub fn length(&self) -> N {
        match self {
            CurvedSegment::Line(start, end) => (start - end).norm(),
            CurvedSegment::Arc(start, center, end) => {
                let angle_span = signed_angle_to(end - center, start - center).abs();
                let radius = (start - center).norm();
                radius * angle_span
            }
        }
    }
}

#[cfg_attr(feature = "compact_containers", derive(Compact))]
#[derive(Clone)]
pub struct CurvedPath {
    points: VecLike<P2>,
    is_point_center: VecLike<bool>,
}

const ARC_DIRECTION_TOLERANCE: N = 0.0001;
const CURVE_LINEARIZATION_MAX_ANGLE: N = 0.1;

/// Creation
impl CurvedPath {
    pub fn line(start: P2, end: P2) -> Option<Self> {
        if (end - start).norm() <= THICKNESS {
            None
        } else {
            Some(CurvedPath {
                points: vec![start, end].into(),
                is_point_center: vec![false, false].into(),
            })
        }
    }

    pub fn arc(start: P2, start_direction: V2, end: P2) -> Option<Self> {
        if (end - start).norm() <= THICKNESS {
            None
        } else if start_direction.rough_eq_by((end - start).normalize(), ARC_DIRECTION_TOLERANCE) {
            Self::line(start, end)
        } else {
            let signed_radius = {
                let half_chord = (end - start) / 2.0;
                half_chord.norm_squared() / start_direction.orthogonal().dot(&half_chord)
            };
            let center = start + signed_radius * start_direction.orthogonal();
            Some(CurvedPath {
                points: vec![start, center, end].into(),
                is_point_center: vec![false, true, false].into(),
            })
        }
    }

    pub fn biarc(start: P2, start_direction: V2, end: P2, end_direction: V2) -> Option<Self> {
        const MAX_SIMPLE_LINE_LENGTH: N = 0.1;
        const RAY_LENGTH: N = 10_000.0;

        if (end - start).norm() <= THICKNESS {
            None
        } else if (end - start).norm() < MAX_SIMPLE_LINE_LENGTH {
            Self::line(start, end)
        } else {
            let single_arc = Self::arc(start, start_direction, end)?;
            if single_arc
                .end_direction()
                .rough_eq_by(end_direction, ARC_DIRECTION_TOLERANCE)
            {
                Some(single_arc)
            } else {
                let start_ray = LineSegment(start, start + RAY_LENGTH * start_direction);
                let end_ray = LineSegment(end, end - RAY_LENGTH * end_direction);
                let maybe_linear_intersection = (start_ray, end_ray).intersect().into_iter().find(
                    |intersection| {
                        intersection.along_a < 0.8 * RAY_LENGTH
                            && intersection.along_b < 0.8 * RAY_LENGTH
                    },
                );

                let (connection_position, connection_direction) =
                    if let Some(Intersection { position, .. }) = maybe_linear_intersection {
                        let start_to_intersection_distance = (start - position).norm();
                        let end_to_intersection_distance = (end - position).norm();

                        if start_to_intersection_distance < end_to_intersection_distance {
                            // arc then line
                            (
                                position + start_to_intersection_distance * end_direction,
                                end_direction,
                            )
                        } else {
                            // line then arc
                            (
                                position + end_to_intersection_distance * -start_direction,
                                start_direction,
                            )
                        }
                    } else {
                        // http://www.ryanjuckett.com/programming/biarc-interpolation/
                        let v = end - start;
                        let t = start_direction + end_direction;
                        let same_direction =
                            start_direction.rough_eq_by(end_direction, ARC_DIRECTION_TOLERANCE);
                        let end_orthogonal_of_start = v.dot(&end_direction).rough_eq(0.0);

                        if same_direction && end_orthogonal_of_start {
                            //    __
                            //   /  \
                            //  ^    v    ^
                            //        \__/
                            (
                                P2::from_coordinates((start.coords + end.coords) / 2.0),
                                -start_direction,
                            )
                        } else {
                            let d = if same_direction {
                                v.dot(&v) / (4.0 * v.dot(&end_direction))
                            } else {
                                // magic - I'm pretty sure this can be simplified
                                let v_dot_t = v.dot(&t);
                                (-v_dot_t
                                    + (v_dot_t * v_dot_t
                                        + 2.0
                                            * (1.0 - start_direction.dot(&end_direction))
                                            * v.dot(&v))
                                        .sqrt())
                                    / (2.0 * (1.0 - start_direction.dot(&end_direction)))
                            };

                            let start_offset_point = start + d * start_direction;
                            let end_offset_point = end - d * end_direction;
                            let connection_direction =
                                (end_offset_point - start_offset_point).normalize();

                            (
                                start_offset_point + d * connection_direction,
                                connection_direction,
                            )
                        }
                    };

                match (
                    Self::arc(start, start_direction, connection_position),
                    Self::arc(connection_position, connection_direction, end),
                ) {
                    (Some(first), Some(second)) => first.concat(&second).ok(),
                    (Some(first), None) => Some(first),
                    (None, Some(second)) => Some(second),
                    _ => None,
                }
            }
        }
    }

    pub fn circle(center: P2, radius: N) -> Option<Self> {
        let top = center + V2::new(0.0, radius);
        let bottom = center + V2::new(0.0, -radius);
        let right_segment = Self::arc(top, V2::new(1.0, 0.0), bottom)?;
        let left_segment = Self::arc(bottom, V2::new(-1.0, 0.0), top)?;
        right_segment.concat(&left_segment).ok()
    }
}

/// Inspection
impl CurvedPath {
    pub fn start(&self) -> P2 {
        self.points[0]
    }

    pub fn end(&self) -> P2 {
        *self.points.last().unwrap()
    }

    pub fn length(&self) -> N {
        self.segments().map(|segment| segment.length()).sum()
    }

    pub fn start_direction(&self) -> V2 {
        self.segments().next().unwrap().start_direction()
    }

    pub fn end_direction(&self) -> V2 {
        self.segments().last().unwrap().end_direction()
    }

    pub fn segments<'a>(&'a self) -> impl Iterator<Item = CurvedSegment> + 'a {
        self.points
            .iter()
            .zip(self.is_point_center.iter())
            .scan((None, None), |state, (&point, is_center)| {
                let (new_state, maybe_segment) = match *state {
                    (None, None) => ((Some(point), None), Some(None)),
                    (Some(prev_point), None) => if *is_center {
                        ((Some(prev_point), Some(point)), Some(None))
                    } else {
                        (
                            (Some(point), None),
                            Some(Some(CurvedSegment::Line(prev_point, point))),
                        )
                    },
                    (Some(prev_point), Some(center)) => (
                        (Some(point), None),
                        Some(Some(CurvedSegment::Arc(prev_point, center, point))),
                    ),
                    (None, Some(_)) => unreachable!(),
                };
                *state = new_state;
                maybe_segment
            })
            .filter_map(|maybe_segment| maybe_segment)
    }
}

/// Combination/Modification
impl CurvedPath {
    pub fn concat(&self, other: &Self) -> Result<Self, ConcatError> {
        if self.end().rough_eq(other.start()) {
            Ok(CurvedPath {
                points: self
                    .points
                    .iter()
                    .chain(other.points[1..].iter())
                    .cloned()
                    .collect(),
                is_point_center: self
                    .is_point_center
                    .iter()
                    .chain(other.is_point_center[1..].iter())
                    .cloned()
                    .collect(),
            })
        } else {
            Err(ConcatError)
        }
    }

    pub fn to_line_path_with_max_angle(&self, max_angle: N) -> LinePath {
        let points = self
            .segments()
            .flat_map(|segment| match segment {
                CurvedSegment::Line(start, _end) => vec![start],
                CurvedSegment::Arc(start, center, end) => {
                    let signed_angle_span = signed_angle_to(start - center, end - center);

                    let subdivisions =
                        (signed_angle_span.abs() / max_angle).max(1.0).floor() as usize;
                    let subdivision_angle = signed_angle_span / (subdivisions as f32);

                    let mut pointer = start - center;

                    (0..subdivisions)
                        .into_iter()
                        .map(|_| {
                            let point = center + pointer;
                            pointer = Rotation2::new(subdivision_angle) * pointer;
                            point
                        })
                        .collect::<Vec<_>>()
                }
            })
            .chain(Some(self.end()))
            .collect();

        LinePath::new(points).expect("A valid CurvedPath should always produce a valid LinePath")
    }

    pub fn to_line_path(&self) -> LinePath {
        self.to_line_path_with_max_angle(CURVE_LINEARIZATION_MAX_ANGLE)
    }
}

#[test]
fn to_line_path() {
    use ::{PI};
    let curved_path = CurvedPath::line(P2::new(0.0, 0.0), P2::new(1.0, 0.0))
        .expect("first line should work")
        .concat(
            &CurvedPath::arc(P2::new(1.0, 0.0), V2::new(1.0, 0.0), P2::new(2.0, 1.0))
                .expect("arc should work"),
        )
        .expect("line>arc concat should work")
        .concat(
            &CurvedPath::line(P2::new(2.0, 1.0), P2::new(2.0, 2.0))
                .expect("second line should work"),
        )
        .expect("line-arc>line concat should work");

    println!("{:#?}", curved_path.segments().collect::<Vec<_>>());

    assert_eq!(
        LinePath::new(vec![
            P2::new(0.0, 0.0),
            P2::new(1.0, 0.0),
            P2::new(1.5, 0.13397461),
            P2::new(1.8660254, 0.5),
            P2::new(2.0, 1.0),
            P2::new(2.0, 2.0),
        ]).unwrap(),
        curved_path.to_line_path_with_max_angle(PI / 6.0)
    );
}
