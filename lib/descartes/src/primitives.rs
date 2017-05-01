use super::{N, P2, V2, Curve, FiniteCurve, WithUniqueOrthogonal, angle_along_to,
            RoughlyComparable, Intersect, Intersection, HasBoundingBox, BoundingBox};
use nalgebra::{Dot, Norm, rotate, Vector1, Rotation2};

#[derive(Copy, Clone, Debug)]
pub struct Circle {
    pub center: P2,
    pub radius: N,
}

impl Curve for Circle {
    fn project_with_tolerance(&self, point: P2, _tolerance: N) -> Option<N> {
        Some(self.radius *
             angle_along_to(V2::new(1.0, 0.0), V2::new(0.0, 1.0), (point - self.center)))
    }

    fn distance_to(&self, point: P2) -> N {
        ((point - self.center).norm() - self.radius).abs()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Line {
    pub start: P2,
    pub direction: V2,
}

impl Curve for Line {
    fn project_with_tolerance(&self, point: P2, _tolerance: N) -> Option<N> {
        Some((point - self.start).dot(&self.direction))
    }

    fn distance_to(&self, point: P2) -> N {
        (point - self.start).dot(&self.direction.orthogonal())
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Segment {
    pub start: P2,
    pub center_or_direction: V2,
    pub end: P2,
    pub length: N,
    signed_radius: N,
}

const DIRECTION_TOLERANCE: f32 = 0.01;
pub const MIN_START_TO_END: f32 = 0.01;
const MAX_SIMPLE_LINE_LENGTH: f32 = 0.5;

impl Segment {
    pub fn line(start: P2, end: P2) -> Segment {
        if start.is_roughly_within(end, MIN_START_TO_END) {
            panic!("invalid segment!");
        }
        Segment {
            start: start,
            center_or_direction: (end - start).normalize(),
            end: end,
            length: (end - start).norm(),
            signed_radius: 0.0,
        }
    }

    pub fn arc_with_direction(start: P2, direction: V2, end: P2) -> Segment {
        if start.is_roughly_within(end, MIN_START_TO_END) {
            panic!("invalid segment!");
        }
        if direction.is_roughly_within((end - start).normalize(), DIRECTION_TOLERANCE) {
            Segment::line(start, end)
        } else {
            let signed_radius = {
                let half_chord = (end - start) / 2.0;
                half_chord.norm_squared() / direction.orthogonal().dot(&half_chord)
            };
            let center = start + signed_radius * direction.orthogonal();
            let angle_span = angle_along_to(start - center, direction, end - center);
            Segment {
                start: start,
                center_or_direction: center.to_vector(),
                end: end,
                length: angle_span * signed_radius.abs(),
                signed_radius: signed_radius,
            }
        }
    }

    pub fn biarc(start: P2, start_direction: V2, end: P2, end_direction: V2) -> Vec<Segment> {
        if start.is_roughly_within(end, MIN_START_TO_END) {
            panic!("invalid biarc! {:?}, {:?} -> {:?}, {:?}",
                   start,
                   start_direction,
                   end,
                   end_direction);
        }
        let simple_curve = Segment::arc_with_direction(start, start_direction, end);
        if simple_curve
               .end_direction()
               .is_roughly_within(end_direction, DIRECTION_TOLERANCE) {
            vec![simple_curve]
        } else if (end - start).norm() < MAX_SIMPLE_LINE_LENGTH {
            vec![Segment::line(start, end)]
        } else {
            let maybe_linear_intersection = (&Line {
                                                  start: start,
                                                  direction: start_direction,
                                              },
                                             &Line {
                                                  start: end,
                                                  direction: -end_direction,
                                              })
                    .intersect()
                    .into_iter()
                    .next()
                    .and_then(|intersection| if intersection.along_a > 0.0 &&
                                                intersection.along_b > 0.0 {
                                  Some(intersection)
                              } else {
                                  None
                              });

            let (connection_position, connection_direction) = if let Some(Intersection {
                                                                              position, ..
                                                                          }) =
                maybe_linear_intersection {
                let start_to_intersection_distance = (start - position).norm();
                let end_to_intersection_distance = (end - position).norm();

                if start_to_intersection_distance < end_to_intersection_distance {
                    // arc then line
                    (position + start_to_intersection_distance * end_direction, end_direction)
                } else {
                    // line then arc
                    (position + end_to_intersection_distance * -start_direction, start_direction)
                }
            } else {
                // http://www.ryanjuckett.com/programming/biarc-interpolation/
                let v = end - start;
                let t = start_direction + end_direction;
                let same_direction =
                    start_direction.is_roughly_within(end_direction, DIRECTION_TOLERANCE);
                let end_orthogonal_of_start = v.dot(&end_direction).is_roughly(0.0);

                if same_direction && end_orthogonal_of_start {
                    //    __
                    //   /  \
                    //  ^    v    ^
                    //        \__/
                    (((start.to_vector() + end.to_vector()) / 2.0).to_point(), -start_direction)
                } else {
                    let d = if same_direction {
                        v.dot(&v) / (4.0 * v.dot(&end_direction))
                    } else {
                        // magic - I'm pretty sure this can be simplified
                        let v_dot_t = v.dot(&t);
                        (-v_dot_t +
                         (v_dot_t * v_dot_t +
                          2.0 * (1.0 - start_direction.dot(&end_direction)) * v.dot(&v))
                                 .sqrt()) /
                        (2.0 * (1.0 - start_direction.dot(&end_direction)))
                    };

                    let start_offset_point = start + d * start_direction;
                    let end_offset_point = end - d * end_direction;
                    let connection_direction = (end_offset_point - start_offset_point).normalize();

                    (start_offset_point + d * connection_direction, connection_direction)
                }

            };

            if start.is_roughly_within(connection_position, MIN_START_TO_END) {
                vec![Segment::arc_with_direction(connection_position, connection_direction, end)]
            } else if end.is_roughly_within(connection_position, MIN_START_TO_END) {
                vec![Segment::arc_with_direction(start, start_direction, connection_position)]
            } else {
                vec![Segment::arc_with_direction(start, start_direction, connection_position),
                     Segment::arc_with_direction(connection_position, connection_direction, end)]
            }
        }
    }

    pub fn is_linear(&self) -> bool {
        self.signed_radius == 0.0
    }

    pub fn center(&self) -> P2 {
        *self.center_or_direction.as_point()
    }

    pub fn radius(&self) -> N {
        self.signed_radius.abs()
    }
}

impl FiniteCurve for Segment {
    fn length(&self) -> N {
        self.length
    }

    fn along(&self, distance: N) -> P2 {
        if self.is_linear() {
            self.start + distance * self.center_or_direction
        } else {
            let center_to_start = self.start - self.center();
            let angle_to_rotate = distance / -self.signed_radius;
            let center_to_point = rotate(&Rotation2::new(Vector1::new(angle_to_rotate)),
                                         &center_to_start);
            self.center() + center_to_point
        }
    }

    fn direction_along(&self, distance: N) -> V2 {
        if self.is_linear() {
            self.center_or_direction
        } else {
            let center_to_start = self.start - self.center();
            let angle_to_rotate = distance / -self.signed_radius;
            let center_to_point = rotate(&Rotation2::new(Vector1::new(angle_to_rotate)),
                                         &center_to_start);
            center_to_point.normalize().orthogonal() * self.signed_radius.signum()
        }
    }

    fn start(&self) -> P2 {
        self.start
    }

    fn start_direction(&self) -> V2 {
        if self.is_linear() {
            self.center_or_direction
        } else {
            let center_to_start = self.start - self.center();
            center_to_start.normalize().orthogonal() * self.signed_radius.signum()
        }
    }

    fn end(&self) -> P2 {
        self.end
    }

    fn end_direction(&self) -> V2 {
        if self.is_linear() {
            self.center_or_direction
        } else {
            let center_to_end = self.end - self.center();
            center_to_end.normalize().orthogonal() * self.signed_radius.signum()
        }
    }

    fn reverse(&self) -> Segment {
        if self.is_linear() {
            Segment::line(self.end, self.start)
        } else {
            Segment::arc_with_direction(self.end, -self.end_direction(), self.start)
        }
    }

    fn subsection(&self, start: N, end: N) -> Option<Segment> {
        let true_start = start.max(0.0);
        let true_end = end.min(self.length);

        if true_end - true_start < MIN_START_TO_END {
            None
        } else if self.is_linear() || true_end.is_roughly(0.0) ||
                  true_start.is_roughly(self.length) {
            Some(Segment::line(self.along(true_start), self.along(true_end)))
        } else {
            Some(Segment::arc_with_direction(self.along(true_start),
                                             self.direction_along(true_start),
                                             self.along(true_end)))
        }
    }

    fn shift_orthogonally(&self, shift_to_right: N) -> Option<Segment> {
        if self.is_linear() {
            let offset = self.start_direction().orthogonal() * shift_to_right;
            Some(Segment::line(self.start + offset, self.end + offset))
        } else {
            let start = self.start + self.start_direction().orthogonal() * shift_to_right;
            let end = self.end + self.end_direction().orthogonal() * shift_to_right;
            if start.is_roughly_within(end, MIN_START_TO_END) {
                None
            } else {
                Some(Segment::arc_with_direction(start, self.start_direction(), end))
            }
        }
    }
}

const MIN_TOLERANCE_ANGLE: N = 0.005;

impl Curve for Segment {
    fn project_with_tolerance(&self, point: P2, tolerance: N) -> Option<N> {
        if self.is_linear() {
            let direction = self.center_or_direction;
            let line_offset = direction.dot(&(point - self.start));
            if line_offset > -tolerance && line_offset < self.length + tolerance {
                Some(line_offset.max(0.0).min(self.length))
            } else {
                None
            }
        } else {
            let angle_start_to_point = angle_along_to(self.start - self.center(),
                                                      self.start_direction(),
                                                      point - self.center());

            let tolerance_angle = (tolerance / self.radius()).max(MIN_TOLERANCE_ANGLE);
            let angle_span = self.length / self.radius();

            if angle_start_to_point <= angle_span + tolerance_angle {
                Some((angle_start_to_point * self.radius()).min(self.length))
            } else if angle_start_to_point >= 2.0 * ::std::f32::consts::PI - tolerance_angle {
                Some(0.0)
            } else {
                None
            }
        }
    }

    // TODO: optimize this
    fn includes(&self, point: P2) -> bool {
        let primitive_includes_point = if self.is_linear() {
            Line {
                    start: self.start,
                    direction: self.center_or_direction,
                }
                .includes(point)
        } else {
            Circle {
                    center: self.center(),
                    radius: self.radius(),
                }
                .includes(point)
        };

        primitive_includes_point && self.project(point).is_some()
    }

    fn distance_to(&self, point: P2) -> N {
        match self.project(point) {
            Some(_offset) => {
                if self.is_linear() {
                    Line {
                            start: self.start,
                            direction: self.center_or_direction,
                        }
                        .distance_to(point)
                } else {
                    Circle {
                            center: self.center(),
                            radius: self.radius(),
                        }
                        .distance_to(point)
                }
            }
            None => {
                (self.start - point)
                    .norm()
                    .min((self.end - point).norm())
            }
        }
    }
}

impl<'a> RoughlyComparable for &'a Segment {
    fn is_roughly_within(&self, other: &Segment, tolerance: N) -> bool {
        self.start.is_roughly_within(other.start, tolerance)
        && self.end.is_roughly_within(other.end, tolerance)
        // much stricter tolerance here!
        && self.start_direction().is_roughly(other.start_direction()) &&
        self.end_direction().is_roughly(other.end_direction())
    }
}

impl HasBoundingBox for Segment {
    fn bounding_box(&self) -> BoundingBox {
        if self.is_linear() {
            BoundingBox {
                min: P2::new(self.start.x.min(self.end.x), self.start.y.min(self.end.y)),
                max: P2::new(self.start.x.max(self.end.x), self.start.y.max(self.end.y)),
            }
        } else {
            let half_diagonal = V2::new(self.radius(), self.radius());
            BoundingBox {
                min: self.center() - half_diagonal,
                max: self.center() + half_diagonal,
            }
        }
    }
}
