use super::{
    N, P2, V2, THICKNESS,
    Curve, FiniteCurve,
    WithUniqueOrthogonal, angle_along_to
};
use ::nalgebra::{Dot, Norm, rotate, Vector1, Rotation2};

#[derive(Copy, Clone, Debug)]
pub struct Circle {
    pub center: P2,
    pub radius: N,
}

impl Curve for Circle {
    fn project(&self, point: P2) -> Option<N> {
        Some(angle_along_to(
            V2::new(1.0, 0.0),
            V2::new(0.0, 1.0),
            (point - self.center) * self.radius))
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
    fn project(&self, point: P2) -> Option<N> {
        Some((point - self.start).dot(&self.direction))
    }

    fn distance_to(&self, point: P2) -> N {
        (point - self.start).dot(&self.direction.orthogonal())
    }
}

#[derive(Copy, Clone)]
pub struct Segment {
    pub start: P2,
    pub center_or_direction: V2,
    pub end: P2,
    pub length: N,
    signed_radius: N
}

impl Segment {
    pub fn line(start: P2, end: P2) -> Segment {
        Segment{
            start: start,
            center_or_direction: (end - start).normalize(),
            end: end,
            length: (end - start).norm(),
            signed_radius: 0.0
        }
    }

    pub fn arc(start: P2, center: P2, end: P2) -> Segment {
        let start_to_center = center - start;
        let signed_radius = (start_to_center).norm() * start_to_center.dot(&(end - start)).signum();
        let direction = start_to_center.normalize().orthogonal() * -signed_radius.signum();
        let angle_span = angle_along_to(start - center, direction, end - center);
        Segment{
            start: start,
            center_or_direction: center.to_vector(),
            end: end,
            length: angle_span * signed_radius.abs(),
            signed_radius: signed_radius
        }
    }

    pub fn arc_with_direction(start: P2, direction: V2, end: P2) -> Segment {
        let signed_radius = {
            let half_chord = (end - start) / 2.0;
            half_chord.norm_squared() / direction.orthogonal().dot(&half_chord)
        };
        let center = start + signed_radius * direction.orthogonal();
        let angle_span = angle_along_to(start - center, direction, end - center);
        Segment{
            start: start,
            center_or_direction: center.to_vector(),
            end: end,
            length: angle_span * signed_radius.abs(),
            signed_radius: signed_radius
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

    fn start_direction_of_arc(&self) -> V2 {
        let start_to_center = self.center() - self.start;
        start_to_center.normalize().orthogonal() * -self.signed_radius.signum()
    }

    fn end_direction_of_arc(&self) -> V2 {
        let end_to_center = self.center() - self.end;
        end_to_center.normalize().orthogonal() * -self.signed_radius.signum()
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
            let center_to_point = rotate(&Rotation2::new(Vector1::new(angle_to_rotate)), &center_to_start);
            self.center() + center_to_point
        }
    }

    fn direction_along(&self, distance: N) -> V2 {
        if self.is_linear() {
            self.center_or_direction
        } else {
            let center_to_start = self.start - self.center();
            let angle_to_rotate = distance / -self.signed_radius;
            let center_to_point = rotate(&Rotation2::new(Vector1::new(angle_to_rotate)), &center_to_start);
            center_to_point.normalize().orthogonal() * self.signed_radius.signum()
        }
    }

    fn start(&self) -> P2 {self.start}

    fn start_direction(&self) -> V2 {
        if self.is_linear() {
            self.center_or_direction
        } else {
            let center_to_start = self.start - self.center();
            center_to_start.normalize().orthogonal() * self.signed_radius.signum()
        }
    }

    fn end(&self) -> P2 {self.end}

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
            Segment::arc(self.end, self.center(), self.end)
        }
    }

    fn subsection(&self, start: N, end: N) -> Segment {
        let true_start = start.max(0.0);
        let true_end = end.min(self.length);

        if self.is_linear() {
            Segment::line(self.along(true_start), self.along(true_end))
        } else {
            Segment::arc(self.along(true_start), self.center(), self.along(true_end))
        }
    }

    fn shift_orthogonally(&self, shift_to_right: N) -> Segment {
        if self.is_linear() {
            let offset = self.start_direction().orthogonal() * shift_to_right;
            Segment::line(self.start + offset, self.end + offset)
        } else {
            let start_offset = self.start_direction().orthogonal() * shift_to_right;
            let end_offset = self.end_direction().orthogonal() * shift_to_right;
            Segment::arc(self.start + start_offset, self.center(), self.end + end_offset)
        }
    }
}

impl Curve for Segment {
    fn project(&self, point: P2) -> Option<N> {
        if self.is_linear() {
            let direction = self.center_or_direction;
            let line_offset = direction.dot(&(point - self.end));
            if line_offset > -THICKNESS && line_offset < self.length + THICKNESS {
                Some(line_offset)
            } else {None}
        } else {
            let angle_start_to_point = angle_along_to(
                self.start - self.center(), self.start_direction_of_arc(), point - self.center());
            let angle_end_to_point = angle_along_to(
                self.end - self.center(), -self.end_direction_of_arc(), point - self.center());

            let tolerance = THICKNESS / self.radius();
            let angle_span = self.length / self.radius();

            if angle_start_to_point <= angle_span + tolerance &&
                angle_end_to_point <= angle_span + tolerance {
                Some(angle_start_to_point.max(0.0).min(angle_span) * self.radius())
            } else {None}
        }
    }

    // TODO: optimize this
    fn includes(&self, point: P2) -> bool {
        let primitive_includes_point = if self.is_linear() {
            Line{start: self.start, direction: self.center_or_direction}.includes(point)
        } else {
            Circle{center: self.center(), radius: self.radius()}.includes(point)
        };

        primitive_includes_point && self.project(point).is_some()
    }

    fn distance_to(&self, _point: P2) -> N {
        unimplemented!()
    }
}