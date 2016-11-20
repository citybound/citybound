use super::{N, P2, RoughlyComparable, Curve, FiniteCurve, THICKNESS, WithUniqueOrthogonal};
use super::primitives::{Line, Circle, Segment};
use super::nalgebra::{Dot, Norm};
use super::path::Path;

#[derive(Copy, Clone, Debug)]
pub struct Intersection {
    pub along_a: N,
    pub along_b: N,
    pub position: P2,
}

impl Intersection {
	fn swapped(&self) -> Intersection {
		Intersection{
			along_a: self.along_b,
			along_b: self.along_a,
			position: self.position
		}
	}
}

pub trait Intersect {
    fn intersect(&self) -> Vec<Intersection>;
}

impl<'a> Intersect for (&'a Line, &'a Line) {
    fn intersect(&self) -> Vec<Intersection> {
        let (a, b) = *self;

        let det = b.direction.x * a.direction.y - b.direction.y * a.direction.x;

        if !det.is_roughly(0.0) {
            let delta = b.start - a.start;
            let along_a = (delta.y * b.direction.x - delta.x * b.direction.y) / det;
            vec![Intersection{
                along_a: along_a,
                along_b: (delta.y * a.direction.x - delta.x * a.direction.y) / det,
                position: a.start + a.direction * along_a
            }]
        } else {
            vec![]
        }
    }
}

impl<'a> Intersect for (&'a Line, &'a Circle) {
    fn intersect(&self) -> Vec<Intersection> {
        let (l, c) = *self;

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

            if det > 0.0 {
                let t2 = -direction_dot_delta + det.sqrt();
                let solution2_position = l.start + t2 * l.direction;
                let solution2 = Intersection {
                    along_a: t2,
                    along_b: c.project(solution2_position).unwrap(),
                    position: solution2_position,
                };

                vec![solution1, solution2]
            } else {
                vec![solution1]
            }
        } else {
            vec![]
        }
    }
}

impl<'a> Intersect for (&'a Circle, &'a Line) {
    fn intersect(&self) -> Vec<Intersection> {
        let (c, l) = *self;
        (l, c).intersect().iter().map(Intersection::swapped).collect()
    }
}

impl<'a> Intersect for (&'a Circle, &'a Circle) {
    fn intersect(&self) -> Vec<Intersection> {
        let (a, b) = *self;
        let a_to_b = b.center - a.center;
        let a_to_b_dist = a_to_b.norm();

        if (a_to_b_dist.is_roughly(0.0) && a.radius.is_roughly(b.radius))
        ||  a_to_b_dist > a.radius + b.radius + THICKNESS
        ||  a_to_b_dist < (a.radius - b.radius).abs() - THICKNESS {
            vec![]
        } else {
            let a_to_centroid_dist = (a.radius.powi(2) - b.radius.powi(2) + a_to_b_dist.powi(2)) / (2.0 * a_to_b_dist);
            let intersection_to_centroid_dist = (a.radius.powi(2) - a_to_centroid_dist.powi(2)).sqrt();

            let centroid = a.center + (a_to_b * a_to_centroid_dist / a_to_b_dist); 

            let centroid_to_intersection = a_to_b.normalize().orthogonal() * intersection_to_centroid_dist;

            let solution_1_position = centroid + centroid_to_intersection;
            let solution_1 = Intersection{
                along_a: a.project(solution_1_position).unwrap(),
                along_b: b.project(solution_1_position).unwrap(),
                position: solution_1_position
            };

            if (centroid - a.center).norm().is_roughly(a.radius) {
                vec![solution_1]
            } else {
                let solution_2_position = centroid - centroid_to_intersection;
                let solution_2 = Intersection{
                    along_a: a.project(solution_2_position).unwrap(),
                    along_b: b.project(solution_2_position).unwrap(),
                    position: solution_2_position
                };
                vec![solution_1, solution_2]
            }
        }
    }
}

// TODO: optimize: use something better than .includes()
impl<'a> Intersect for (&'a Segment, &'a Segment) {
    fn intersect(&self) -> Vec<Intersection> {
        let (a, b) = *self;
        match (a.is_linear(), b.is_linear()) {
            (true, true) => (
                    &Line{start: a.start(), direction: a.start_direction()},
                    &Line{start: b.start(), direction: b.start_direction()},
                ).intersect().iter().filter(|intersection|
                    intersection.along_a >= 0.0 && intersection.along_a <= a.length() &&
                    intersection.along_b >= 0.0 && intersection.along_b <= b.length()
                ).cloned().collect(),
            (true, false) => (
                    &Line{start: a.start(), direction: a.start_direction()},
                    &Circle{center: b.center(), radius: b.radius()}
                ).intersect().iter().filter(|intersection|
                    intersection.along_a >= 0.0 && intersection.along_a <= a.length() &&
                    b.includes(intersection.position)
                ).map(|intersection|
                    Intersection{
                        along_b: b.project(intersection.position).unwrap(),
                        ..*intersection
                    }
                ).collect(),
            (false, true) => (b, a).intersect().iter().map(Intersection::swapped).collect(),
            (false, false) => (
                    &Circle{center: a.center(), radius: a.radius()},
                    &Circle{center: b.center(), radius: b.radius()}
                ).intersect().iter().filter(|intersection|
                    a.includes(intersection.position) &&
                    b.includes(intersection.position)
                ).map(|intersection|
                    Intersection{
                        along_a: a.project(intersection.position).unwrap(),
                        along_b: b.project(intersection.position).unwrap(),
                        ..*intersection
                    }
                ).collect()
        }
    }
}

impl<'a, P: Path> Intersect for (&'a P, &'a P) {
    fn intersect(&self) -> Vec<Intersection> {
        let (a, b) = *self;
        let mut intersection_list = Vec::new();
        for (segment_a, offset_a) in a.segments_with_start_offsets() {
            for (segment_b, offset_b) in b.segments_with_start_offsets() {
                for intersection in (segment_a, segment_b).intersect() {
                    let identical_to_previous = if let Some(previous_intersection) = intersection_list.last() {
                        (previous_intersection as &Intersection).position.is_roughly_within(intersection.position, THICKNESS)
                    } else {false};
                    if !identical_to_previous {
                        intersection_list.push(Intersection{
                            along_a: intersection.along_a + offset_a,
                            along_b: intersection.along_b + offset_b,
                            position: intersection.position
                        });
                    }
                }
            }
        }
        intersection_list
    }
} 