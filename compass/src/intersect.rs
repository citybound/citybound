use ::{N, P2, RoughlyComparable, Curve};
use ::primitives::{Line, Circle};
use nalgebra::{Dot, Norm};
use smallvec::SmallVec;

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

pub type IntersectionList = SmallVec<[Intersection; 2]>;

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