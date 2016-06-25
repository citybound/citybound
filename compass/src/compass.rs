// #[cfg(test)]
// mod tests {
//     #[test]
//     fn it_works() {
//     }
// }

extern crate nalgebra;
extern crate smallvec;
use self::nalgebra::{Vector2, Point2, Dot, Norm};
use std::f32::consts::PI;
use self::smallvec::SmallVec;
pub type N = f32;
pub type V2 = Vector2<N>;
pub type P2 = Point2<N>;
pub type IntersectionList = SmallVec<[Intersection; 2]>;

// Thickness radius
const THICKNESS: N = 0.0001;
const ROUGH_TOLERANCE: N = 0.0000001;

fn angle_between(a: V2, b: V2) -> N {
    let theta: N = a.dot(&b) / (a.norm() * b.norm());
    return theta.min(1.0).max(-1.0).acos();
}

fn angle_between_with_direction(a: V2, a_direction: V2, b: V2) -> N {
    let simple_angle = angle_between(a, b);
    let linear_direction = (b - a).normalize();

    if a_direction.dot(&linear_direction) >= 0.0 {
        return simple_angle;
    } else {
        return 2.0 * PI - simple_angle;
    };
}

trait WithUniqueOrthogonal {
    fn orthogonal(&self) -> Self;
}

impl WithUniqueOrthogonal for V2 {
    fn orthogonal(&self) -> V2 {
        V2::new(self.y, -self.x)
    }
}

trait RoughlyComparable {
    fn is_roughly(&self, other: Self) -> bool;
}

impl RoughlyComparable for N {
    fn is_roughly(&self, other: N) -> bool {
        return (self - other).abs() <= ROUGH_TOLERANCE;
    }
}

trait Curve {
    fn project(&self, point: P2) -> Option<N>;
    fn contains(&self, point: P2) -> bool;
    fn distance_to(&self, point: P2) -> N;
}

#[derive(Debug)]
pub struct Circle {
    pub center: P2,
    pub radius: N,
}

impl Curve for Circle {
    fn project(&self, point: P2) -> Option<N> {
        Some(angle_between_with_direction(V2::new(1.0, 0.0),
                                            V2::new(0.0, 1.0),
                                            (point - self.center) * self.radius))
    }

    fn contains(&self, point: P2) -> bool {
        return (point - self.center).norm() <= self.radius + THICKNESS;
    }

    fn distance_to(&self, point: P2) -> N {
        0.0
    }
}

#[derive(Debug)]
pub struct Line {
    pub start: P2,
    pub direction: V2,
}

impl Curve for Line {
    fn project(&self, point: P2) -> Option<N> {
        Some((point - self.start).dot(&self.direction))
    }

    fn contains(&self, point: P2) -> bool {
        let distance = (point - self.start).dot(&self.direction.orthogonal());
        distance < THICKNESS
    }

    fn distance_to(&self, point: P2) -> N {
        0.0
    }
}

pub struct Ray {
    pub start: P2,
    pub direction: V2
}

impl Ray {
    fn as_line(&self) -> &Line {
        unsafe{::std::mem::transmute(self)}
    }
}

impl Curve for Ray {
    fn project(&self, point: P2) -> Option<N> {
        let line_offset = self.as_line().project(point).unwrap();
        if line_offset > -THICKNESS {Some(line_offset)} else {None}
    }

    fn contains(&self, point: P2) -> bool {
        self.as_line().contains(point) && match self.project(point) {
             Some(offset) => offset > -THICKNESS,
             None => false
        }
    }

    fn distance_to(&self, point: P2) -> N {
        0.0
    }
}

trait FiniteCurve : Curve {
    fn length(&self) -> N;
}

pub struct Segment {
    pub start: P2,
    pub center_or_direction: V2,
    pub end: P2,
    length: N,
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
        let angle_span = angle_between_with_direction(start - center, direction, end - center);
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
        let angle_span = angle_between_with_direction(start - center, direction, end - center);
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

    fn center(&self) -> P2 {
        *self.center_or_direction.as_point()
    }

    fn radius(&self) -> N {
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
            let angle_start_to_point = angle_between_with_direction(
                self.start - self.center(), self.start_direction_of_arc(), point - self.center());
            let angle_end_to_point = angle_between_with_direction(
                self.end - self.center(), -self.end_direction_of_arc(), point - self.center());

            let tolerance = THICKNESS / self.radius();
            let angle_span = self.length / self.radius();

            if angle_start_to_point <= angle_span + tolerance &&
                angle_end_to_point <= angle_span + tolerance {
                Some(angle_start_to_point.max(0.0).min(angle_span) * self.radius())
            } else {None}
        }
    }

    fn contains(&self, point: P2) -> bool {
        let primitive_contains_point = if self.is_linear() {
            Line{start: self.start, direction: self.center_or_direction}.contains(point)
        } else {
            Circle{center: self.center(), radius: self.radius()}.contains(point)
        };

        primitive_contains_point && self.project(point).is_some()
    }

    fn distance_to(&self, point: P2) -> N {
        0.0
    }
}

struct Path {
    segments: [Segment]
}

struct StartOffsetState(N);

impl<'a> Path {
    fn scan_segments (start_offset: &mut StartOffsetState, segment: &'a Segment) -> Option<(&'a Segment, N)> {
        let pair = (segment, start_offset.0);
        start_offset.0 += segment.length;
        Some(pair)
    }

    fn segments_with_start_offsets(&'a self)
    -> ::std::iter::Scan<
        ::std::slice::Iter<'a, Segment>,
        StartOffsetState,
        fn(&mut StartOffsetState, &'a Segment) -> Option<(&'a Segment, N)>
    > {
        return self.segments.into_iter().scan(StartOffsetState(0.0), Self::scan_segments);
    }
}

impl FiniteCurve for Path {
    fn length(&self) -> N {
        self.segments.into_iter().map(|segment| segment.length()).fold(0.0, ::std::ops::Add::add)
    }
}

impl Curve for Path {
    fn project(&self, point: P2) -> Option<N> {
        self.segments_with_start_offsets().filter_map(|pair: (&Segment, N)| {
            let (segment, start_offset) = pair;
            let offset_on_segment = segment.project(point);
            match offset_on_segment {
                Some(offset) => Some(start_offset + offset),
                None => None
            }
        }).next()
    }

    fn contains(&self, point: P2) -> bool {
        self.segments.into_iter().any(|segment| segment.contains(point))
    }

    fn distance_to(&self, point: P2) -> N {
        self.segments.into_iter().fold(None, |min, segment| {
            let distance = segment.distance_to(point);
            if min.is_some() && distance < min.unwrap() {Some(distance)} else {min}
        }).unwrap()
    }
}

#[derive(Debug)]
pub struct Intersection {
    pub along_a: N,
    pub along_b: N,
    pub position: P2,
}

impl Intersection {
	fn swapped(self) -> Intersection {
		return Intersection{
			along_a: self.along_b,
			along_b: self.along_a,
			position: self.position
		}
	}
}

pub trait Intersect {
    fn intersect(&self) -> IntersectionList;
}

impl<'a> Intersect for (&'a Line, &'a Line) {
    fn intersect(&self) -> IntersectionList {
        let (ref a, ref b) = *self;
        let mut intersection_list = SmallVec::new();

        let det = b.direction.x * a.direction.y - b.direction.y * a.direction.x;

        if !det.is_roughly(0.0) {
            let delta = b.start - a.start;
            let along_a = (delta.y * b.direction.x - delta.x * b.direction.y) / det;
            intersection_list.push(Intersection{
                along_a: along_a,
                along_b: (delta.y * a.direction.x - delta.x * a.direction.y) / det,
                position: a.start + a.direction * along_a
            });
        };

        return intersection_list;
    }
}

impl<'a> Intersect for (&'a Circle, &'a Line) {
    fn intersect(&self) -> IntersectionList {
        let (ref c, ref l) = *self;
        let mut intersection_list = SmallVec::new();

        let delta = l.start - c.center;
        let direction_dot_delta = l.direction.dot(&delta);
        let det = direction_dot_delta.powi(2) - (delta.norm_squared() - c.radius.powi(2));

        if det >= 0.0 {

            let t1 = -direction_dot_delta - det.sqrt();
            let solution1_position = l.start + t1 * l.direction;
            let solution1 = Intersection {
                along_a: t1,
                along_b: c.project(solution1_position).unwrap(),
                position: solution1_position,
            };

            intersection_list.push(solution1);

            if det > 0.0 {
                let t2 = -direction_dot_delta + det.sqrt();
                let solution2_position = l.start + t2 * l.direction;
                let solution2 = Intersection {
                    along_a: t2,
                    along_b: c.project(solution2_position).unwrap(),
                    position: solution2_position,
                };

                intersection_list.push(solution2);
            };
        }

        return intersection_list;
    }
}

impl<'a> Intersect for (&'a Line, &'a Circle) {
    fn intersect(&self) -> IntersectionList {
        let (l, c) = *self;
        let mut intersection_list = (c, l).intersect();
        return intersection_list.into_iter().map(|i| i.swapped()).collect::<IntersectionList>();
    }
}
