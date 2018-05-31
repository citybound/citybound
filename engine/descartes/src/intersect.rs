use super::{N, P2, RoughEq, Curve, FiniteCurve, THICKNESS, WithUniqueOrthogonal, HasBoundingBox};
use super::curves::{Line, Circle, Segment};
use super::path::Path;

#[derive(Copy, Clone, Debug)]
pub struct Intersection {
    pub along_a: N,
    pub along_b: N,
    pub position: P2,
}

impl Intersection {
    fn swapped(self) -> Intersection {
        Intersection {
            along_a: self.along_b,
            along_b: self.along_a,
            position: self.position,
        }
    }
}

#[derive(Clone)]
pub enum IntersectionResult {
    Coincident,
    Apart,
    Intersecting(Vec<Intersection>),
}

impl IntersectionResult {
    fn swap(self) -> Self {
        match self {
            IntersectionResult::Intersecting(intersections) => IntersectionResult::Intersecting(
                intersections
                    .into_iter()
                    .map(Intersection::swapped)
                    .collect(),
            ),
            o => o,
        }
    }
}

pub trait Intersect {
    fn intersect(&self) -> IntersectionResult;
}

impl<'a> Intersect for (&'a Line, &'a Line) {
    fn intersect(&self) -> IntersectionResult {
        let (a, b) = *self;

        let det = b.direction.x * a.direction.y - b.direction.y * a.direction.x;

        if !det.rough_eq(0.0) {
            let delta = b.start - a.start;
            let along_a = (delta.y * b.direction.x - delta.x * b.direction.y) / det;
            IntersectionResult::Intersecting(vec![
                Intersection {
                    along_a,
                    along_b: (delta.y * a.direction.x - delta.x * a.direction.y) / det,
                    position: a.start + a.direction * along_a,
                },
            ])
        } else {
            if a.includes(b.start) {
                IntersectionResult::Coincident
            } else {
                IntersectionResult::Apart
            }
        }
    }
}

impl<'a> Intersect for (&'a Line, &'a Circle) {
    fn intersect(&self) -> IntersectionResult {
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

                IntersectionResult::Intersecting(vec![solution1, solution2])
            } else {
                IntersectionResult::Intersecting(vec![solution1])
            }
        } else {
            IntersectionResult::Apart
        }
    }
}

impl<'a> Intersect for (&'a Circle, &'a Line) {
    fn intersect(&self) -> IntersectionResult {
        let (c, l) = *self;
        (l, c).intersect().swap()
    }
}

impl<'a> Intersect for (&'a Circle, &'a Circle) {
    fn intersect(&self) -> IntersectionResult {
        let (a, b) = *self;
        let a_to_b = b.center - a.center;
        let a_to_b_dist = a_to_b.norm();

        if a_to_b_dist.rough_eq(0.0) && a.radius.rough_eq(b.radius) {
            IntersectionResult::Coincident
        } else if a_to_b_dist > a.radius + b.radius + THICKNESS ||
                   a_to_b_dist < (a.radius - b.radius).abs() - THICKNESS
        {
            IntersectionResult::Apart
        } else {
            let a_to_centroid_dist = (a.radius.powi(2) - b.radius.powi(2) + a_to_b_dist.powi(2)) /
                (2.0 * a_to_b_dist);
            let intersection_to_centroid_dist = (a.radius.powi(2) - a_to_centroid_dist.powi(2))
                .sqrt();

            let centroid = a.center + (a_to_b * a_to_centroid_dist / a_to_b_dist);

            let centroid_to_intersection = a_to_b.normalize().orthogonal() *
                intersection_to_centroid_dist;

            let solution_1_position = centroid + centroid_to_intersection;
            let solution_1 = Intersection {
                along_a: a.project(solution_1_position).unwrap(),
                along_b: b.project(solution_1_position).unwrap(),
                position: solution_1_position,
            };

            if (centroid - a.center).norm().rough_eq(a.radius) {
                IntersectionResult::Intersecting(vec![solution_1])
            } else {
                let solution_2_position = centroid - centroid_to_intersection;
                let solution_2 = Intersection {
                    along_a: a.project(solution_2_position).unwrap(),
                    along_b: b.project(solution_2_position).unwrap(),
                    position: solution_2_position,
                };
                IntersectionResult::Intersecting(vec![solution_1, solution_2])
            }
        }
    }
}

// TODO: optimize: use something better than .includes()
impl<'a> Intersect for (&'a Segment, &'a Segment) {
    fn intersect(&self) -> IntersectionResult {
        let (a, b) = *self;
        if !a.bounding_box().overlaps(&b.bounding_box()) {
            return IntersectionResult::Apart;
        }

        let primitive_intersections = match (a.is_linear(), b.is_linear()) {
            (true, true) => {
                (
                    &Line {
                        start: a.start(),
                        direction: a.start_direction(),
                    },
                    &Line {
                        start: b.start(),
                        direction: b.start_direction(),
                    },
                ).intersect()
            }
            (true, false) => {
                (
                    &Line {
                        start: a.start(),
                        direction: a.start_direction(),
                    },
                    &Circle { center: b.center(), radius: b.radius() },
                ).intersect()
            }
            (false, true) => {
                (
                    &Circle { center: a.center(), radius: a.radius() },
                    &Line {
                        start: b.start(),
                        direction: b.start_direction(),
                    },
                ).intersect()
            }
            (false, false) => {
                (
                    &Circle { center: a.center(), radius: a.radius() },
                    &Circle { center: b.center(), radius: b.radius() },
                ).intersect()
            }
        };

        let mut points_to_consider = match primitive_intersections {
            IntersectionResult::Apart => {
                return IntersectionResult::Apart;
            }
            IntersectionResult::Intersecting(intersections) => {
                intersections
                    .into_iter()
                    .map(|intersection| intersection.position)
                    .collect()
            }
            IntersectionResult::Coincident => vec![],
        };

        points_to_consider.extend(&[a.start(), a.end(), b.start(), b.end()]);

        IntersectionResult::Intersecting(
            points_to_consider
                .into_iter()
                .filter_map(|point| if let (Some(along_a), Some(along_b)) =
                    (
                        a.project_with_max_distance(point, THICKNESS, THICKNESS),
                        b.project_with_max_distance(point, THICKNESS, THICKNESS),
                    )
                {
                    Some(Intersection { along_a, along_b, position: point })
                } else {
                    None
                })
                .collect(),
        )
    }
}

impl<'a> Intersect for (&'a Path, &'a Path) {
    fn intersect(&self) -> IntersectionResult {
        let (a, b) = *self;
        let mut intersection_list = Vec::new();
        for (segment_a, offset_a) in a.segments_with_start_offsets() {
            for (segment_b, offset_b) in b.segments_with_start_offsets() {
                match (segment_a, segment_b).intersect() {
                    IntersectionResult::Intersecting(intersections) => {
                        for intersection in intersections {
                            let identical_to_previous =
                                intersection_list.iter().any(|previous_intersection| {
                                    (previous_intersection as &Intersection)
                                        .position
                                        .rough_eq_by(intersection.position, THICKNESS)
                                });
                            if !identical_to_previous {
                                intersection_list.push(Intersection {
                                    along_a: intersection.along_a + offset_a,
                                    along_b: intersection.along_b + offset_b,
                                    position: intersection.position,
                                });
                            }
                        }
                    }
                    IntersectionResult::Apart => {}
                    IntersectionResult::Coincident => unreachable!(),
                }

            }
        }
        if intersection_list.is_empty() {
            IntersectionResult::Apart
        } else {
            IntersectionResult::Intersecting(intersection_list)
        }
    }
}
