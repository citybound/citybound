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
pub type V2 = Vector2<f32>;
pub type P2 = Point2<f32>;
pub type IntersectionList = SmallVec<[Intersection; 2]>;

const THICKNESS: f32 = 0.0001;
const ROUGH_TOLERANCE: f32 = 0.0000001;

fn angle_between(a: V2, b: V2) -> f32 {
    let theta: f32 = a.dot(&b) / (a.norm() * b.norm());
    return theta.min(1.0).max(-1.0).acos();
}

fn angle_between_with_direction(a: V2, a_direction: V2, b: V2) -> f32 {
    let simple_angle = angle_between(a, b);
    let linear_direction = (b - a).normalize();

    if a_direction.dot(&linear_direction) >= 0.0 {
        return simple_angle;
    } else {
        return 2.0 * PI - simple_angle;
    };
}

trait RoughlyComparable {
    fn is_roughly(&self, other: Self) -> bool;
}

impl RoughlyComparable for f32 {
    fn is_roughly(&self, other: f32) -> bool {
        return (self - other).abs() <= ROUGH_TOLERANCE;
    }
}

trait Curve {
    fn offset_at(&self, point: P2) -> f32;
    fn contains(&self, point: P2) -> bool;
}

#[derive(Debug)]
pub struct Circle {
    pub center: P2,
    pub radius: f32,
}

impl Curve for Circle {
    fn offset_at(&self, point: P2) -> f32 {
        return angle_between_with_direction(V2::new(1.0, 0.0),
                                            V2::new(0.0, 1.0),
                                            (point - self.center) * self.radius);
    }

    fn contains(&self, point: P2) -> bool {
        return (point - self.center).norm() <= self.radius + THICKNESS / 2.0;
    }
}

#[derive(Debug)]
pub struct Line {
    pub start: P2,
    pub direction: V2,
}

#[derive(Debug)]
pub struct Intersection {
    pub along_a: f32,
    pub along_b: f32,
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
                along_b: c.offset_at(solution1_position),
                position: solution1_position,
            };

            intersection_list.push(solution1);

            if det > 0.0 {
                let t2 = -direction_dot_delta + det.sqrt();
                let solution2_position = l.start + t2 * l.direction;
                let solution2 = Intersection {
                    along_a: t2,
                    along_b: c.offset_at(solution2_position),
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
