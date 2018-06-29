use {N, P2, THICKNESS};
use rough_eq::RoughEq;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Intersection {
    pub along_a: N,
    pub along_b: N,
    pub position: P2,
}

pub trait Intersect {
    fn intersect(&self) -> Vec<Intersection>;
}

use line_path::LineSegment;
use bbox::HasBoundingBox;

impl Intersect for (LineSegment, LineSegment) {
    fn intersect(&self) -> Vec<Intersection> {
        let (a, b) = *self;

        if !a.bounding_box().overlaps(&b.bounding_box()) {
            return vec![];
        }

        let a_direction = a.direction();
        let b_direction = b.direction();

        match (
            b.project_with_max_distance(a.start(), THICKNESS, THICKNESS),
            b.project_with_max_distance(a.end(), THICKNESS, THICKNESS),
            a.project_with_max_distance(b.start(), THICKNESS, THICKNESS),
            a.project_with_max_distance(b.end(), THICKNESS, THICKNESS),
        ) {
            // a rough subset of b
            (Some((a_start_along_b, _)), Some((a_end_along_b, _)), ..) => vec![
                Intersection {
                    along_a: 0.0,
                    along_b: a_start_along_b,
                    position: a.start(),
                },
                Intersection {
                    along_a: a.length(),
                    along_b: a_end_along_b,
                    position: a.end(),
                },
            ],
            // b rough subset of a
            (_, _, Some((b_start_along_a, _)), Some((b_end_along_a, _))) => vec![
                Intersection {
                    along_b: 0.0,
                    along_a: b_start_along_a,
                    position: b.start(),
                },
                Intersection {
                    along_b: b.length(),
                    along_a: b_end_along_a,
                    position: b.end(),
                },
            ],
            // single point touches at ends
            (Some((a_start_along_b, _)), None, ..) => vec![Intersection {
                along_a: 0.0,
                along_b: a_start_along_b,
                position: a.start(),
            }],
            (None, Some((a_end_along_b, _)), ..) => vec![Intersection {
                along_a: a.length(),
                along_b: a_end_along_b,
                position: a.end(),
            }],
            (_, _, Some((b_start_along_a, _)), None) => vec![Intersection {
                along_b: 0.0,
                along_a: b_start_along_a,
                position: b.start(),
            }],
            (_, _, None, Some((b_end_along_a, _))) => vec![Intersection {
                along_b: b.length(),
                along_a: b_end_along_a,
                position: b.end(),
            }],
            (None, None, None, None) => {
                let det = b_direction.x * a_direction.y - b_direction.y * a_direction.x;
                let parallel = det.rough_eq(0.0);
                if parallel {
                    // parallel and apart
                    vec![]
                } else {
                    let delta = b.start() - a.start();
                    let along_a = (delta.y * b_direction.x - delta.x * b_direction.y) / det;
                    let along_b = (delta.y * a_direction.x - delta.x * a_direction.y) / det;

                    if along_a > -THICKNESS
                        && along_b > -THICKNESS
                        && along_a < a.length() + THICKNESS
                        && along_b < b.length() + THICKNESS
                    {
                        // single point roughly within segments
                        vec![Intersection {
                            along_a,
                            along_b,
                            position: a.start() + a_direction * along_a,
                        }]
                    } else {
                        // single point outside of segments
                        vec![]
                    }
                }
            }
        }
    }
}

use line_path::LinePath;

impl<'a, 'b> Intersect for (&'a LinePath, &'b LinePath) {
    fn intersect(&self) -> Vec<Intersection> {
        let mut intersections = Vec::<Intersection>::new();
        let (a, b) = *self;

        for (segment_a, a_distance_pair) in a.segments_with_distances() {
            for (segment_b, b_distance_pair) in b.segments_with_distances() {
                for intersection in (segment_a, segment_b).intersect() {
                    let close_exists = intersections.iter().any(|existing| {
                        existing
                            .position
                            .rough_eq_by(intersection.position, THICKNESS)
                    });

                    if !close_exists {
                        intersections.push(Intersection {
                            along_a: a_distance_pair[0] + intersection.along_a,
                            along_b: b_distance_pair[0] + intersection.along_b,
                            position: intersection.position,
                        })
                    }
                }
            }
        }

        intersections
    }
}

#[cfg(test)]
const TINY_BIT: N = THICKNESS / 3.0;

#[test]
fn line_segments_apart() {
    // ----
    // ----

    assert_eq!(
        (
            LineSegment(P2::new(0.0, 0.0), P2::new(1.0, 0.0)),
            LineSegment(P2::new(0.0, 1.0), P2::new(1.0, 1.0)),
        ).intersect(),
        vec![]
    );

    // ----  /
    //      /

    assert_eq!(
        (
            LineSegment(P2::new(0.0, 0.0), P2::new(1.0, 0.0)),
            LineSegment(P2::new(0.0, 1.0), P2::new(2.0, 0.0)),
        ).intersect(),
        vec![]
    );

    // ----  ----

    assert_eq!(
        (
            LineSegment(P2::new(0.0, 0.0), P2::new(1.0, 0.0)),
            LineSegment(P2::new(2.0, 0.0), P2::new(3.0, 0.0)),
        ).intersect(),
        vec![]
    );
}

use closed_line_path::ClosedLinePath;

#[test]
fn line_segments_intersecting() {
    //    /
    // --/--
    //  /

    assert_eq!(
        (
            LineSegment(P2::new(0.0, 0.0), P2::new(1.0, 0.0)),
            LineSegment(P2::new(0.0, 1.0), P2::new(1.0, -1.0)),
        ).intersect(),
        vec![Intersection {
            along_a: 0.5,
            along_b: 1.118034,
            position: P2::new(0.5, 0.0),
        }]
    );

    // |
    // |----
    // |

    assert_eq!(
        (
            LineSegment(P2::new(0.0, 0.0), P2::new(1.0, 0.0)),
            LineSegment(P2::new(0.0, 1.0), P2::new(0.0, -1.0)),
        ).intersect(),
        vec![Intersection {
            along_a: 0.0,
            along_b: 1.0,
            position: P2::new(0.0, 0.0),
        }]
    );
}

#[test]
fn line_segments_barely_intersecting() {
    // |
    // (----
    // |

    assert_eq!(
        (
            LineSegment(P2::new(0.0, 0.0), P2::new(1.0, 0.0)),
            LineSegment(P2::new(-TINY_BIT, 1.0), P2::new(-TINY_BIT, -1.0)),
        ).intersect(),
        vec![Intersection {
            along_a: 0.0,
            along_b: 1.0,
            position: P2::new(0.0, 0.0),
        }]
    );
}

#[cfg(test)]
use curved_path::CurvedPath;

impl<'a, 'b> Intersect for (&'a ClosedLinePath, &'b ClosedLinePath) {
    fn intersect(&self) -> Vec<Intersection> {
        let (a, b) = *self;
        (a.path(), b.path()).intersect()
    }
}

#[test]
fn path_intersecting_at_curved_segment_start() {
    use V2;
    //     |
    // ----|-.
    //     |  \
    assert_eq!(
        (
            &CurvedPath::line(P2::new(0.0, 0.0), P2::new(1.0, 0.0))
                .unwrap()
                .concat(
                    &CurvedPath::arc(P2::new(1.0, 0.0), V2::new(1.0, 0.0), P2::new(2.0, 1.0),)
                        .unwrap()
                )
                .unwrap()
                .to_line_path(),
            &LinePath::new(vec![P2::new(1.0, 1.0), P2::new(1.0, -1.0)]).unwrap(),
        ).intersect(),
        vec![Intersection {
            along_a: 1.0,
            along_b: 1.0,
            position: P2::new(1.0, 0.0),
        }]
    );
}

#[test]
fn path_intersecting_close_before_curved_segment_start() {
    use V2;
    //     |
    // ----(-.
    //     |  \
    assert_eq!(
        (
            &CurvedPath::line(P2::new(0.0, 0.0), P2::new(1.0, 0.0))
                .unwrap()
                .concat(
                    &CurvedPath::arc(P2::new(1.0, 0.0), V2::new(1.0, 0.0), P2::new(2.0, 1.0),)
                        .unwrap()
                )
                .unwrap()
                .to_line_path(),
            &LinePath::new(vec![
                P2::new(1.0 - TINY_BIT, 1.0),
                P2::new(1.0 - TINY_BIT, -1.0),
            ]).unwrap(),
        ).intersect(),
        vec![Intersection {
            along_a: 1.0,
            along_b: 1.0,
            position: P2::new(1.0, 0.0),
        }]
    );
}

#[test]
fn path_intersecting_close_after_curved_segment_start() {
    use V2;
    //     |
    // ----)-.
    //     |  \
    assert_eq!(
        (
            &CurvedPath::line(P2::new(0.0, 0.0), P2::new(1.0, 0.0))
                .unwrap()
                .concat(
                    &CurvedPath::arc(P2::new(1.0, 0.0), V2::new(1.0, 0.0), P2::new(2.0, 1.0),)
                        .unwrap()
                )
                .unwrap()
                .to_line_path(),
            &LinePath::new(vec![
                P2::new(1.0 - TINY_BIT, 1.0),
                P2::new(1.0 - TINY_BIT, -1.0),
            ]).unwrap(),
        ).intersect(),
        vec![Intersection {
            along_a: 1.0,
            along_b: 1.0,
            position: P2::new(1.0, 0.0),
        }]
    );
}
