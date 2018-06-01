use super::{P2, N, THICKNESS, Intersect, Intersection, IntersectionResult, Segment, Path};

const TINY_BIT: N = THICKNESS / 3.0;

#[test]
fn line_segments_barely_intersecting() {
    // |
    // (----
    // |

    assert_eq!(
        (
            &Segment::line(P2::new(0.0, 0.0), P2::new(1.0, 0.0)).unwrap(),
            &Segment::line(P2::new(-TINY_BIT, 1.0), P2::new(-TINY_BIT, -1.0)).unwrap(),
        ).intersect(),
        IntersectionResult::Intersecting(vec![
            Intersection {
                along_a: 0.0,
                along_b: 1.0,
                position: P2::new(-TINY_BIT, 0.0),
            },
        ])
    );
}

#[test]
fn path_intersecting_at_curved_segment_start() {
    use V2;
    //     |
    // ----|-.
    //     |  \
    assert_eq!(
        (
            &Path::new(vec![
                Segment::line(P2::new(0.0, 0.0), P2::new(1.0, 0.0))
                    .unwrap(),
                Segment::arc_with_direction(
                    P2::new(1.0, 0.0),
                    V2::new(1.0, 0.0),
                    P2::new(2.0, 1.0)
                ).unwrap(),
            ]).unwrap(),
            &Path::new(vec![
                Segment::line(P2::new(1.0, 1.0), P2::new(1.0, -1.0))
                    .unwrap(),
            ]).unwrap(),
        ).intersect(),
        IntersectionResult::Intersecting(vec![
            Intersection {
                along_a: 1.0,
                along_b: 1.0,
                position: P2::new(1.0, 0.0),
            },
        ])
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
            &Path::new(vec![
                Segment::line(P2::new(0.0, 0.0), P2::new(1.0, 0.0))
                    .unwrap(),
                Segment::arc_with_direction(
                    P2::new(1.0, 0.0),
                    V2::new(1.0, 0.0),
                    P2::new(2.0, 1.0)
                ).unwrap(),
            ]).unwrap(),
            &Path::new(vec![
                Segment::line(
                    P2::new(1.0 - TINY_BIT, 1.0),
                    P2::new(1.0 - TINY_BIT, -1.0)
                ).unwrap(),
            ]).unwrap(),
        ).intersect(),
        IntersectionResult::Intersecting(vec![
            Intersection {
                along_a: 1.0 - TINY_BIT,
                along_b: 1.0,
                position: P2::new(1.0 - TINY_BIT, 0.0),
            },
        ])
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
            &Path::new(vec![
                Segment::line(P2::new(0.0, 0.0), P2::new(100.0, 0.0))
                    .unwrap(),
                Segment::arc_with_direction(
                    P2::new(100.0, 0.0),
                    V2::new(1.0, 0.0),
                    P2::new(200.0, 100.0)
                ).unwrap(),
            ]).unwrap(),
            &Path::new(vec![
                Segment::line(
                    P2::new(100.0 + TINY_BIT, 100.0),
                    P2::new(100.0 + TINY_BIT, -100.0)
                ).unwrap(),
            ]).unwrap(),
        ).intersect(),
        IntersectionResult::Intersecting(vec![
            Intersection {
                along_a: 100.0,
                along_b: 100.0,
                position: P2::new(100.0 + TINY_BIT, 0.0),
            },
        ])
    );
}

#[test]
fn path_intersecting_at_curved_road() {
    use V2;
    //     _
    //    | |
    // ,--|-|--.
    // '--|-|.  \
    //    |_| `--'

    assert_eq!(
        (
            &Path::new(vec![
                Segment::line(P2::new(0.0, 0.0), P2::new(2.0, 0.0))
                    .unwrap(),
                Segment::arc_with_direction(
                    P2::new(2.0, 0.0),
                    V2::new(1.0, 0.0),
                    P2::new(3.0, 1.0)
                ).unwrap(),
                Segment::line(P2::new(3.0, 1.0), P2::new(2.5, 1.0))
                    .unwrap(),
                Segment::arc_with_direction(
                    P2::new(2.5, 1.0),
                    V2::new(0.0, -1.0),
                    P2::new(2.0, 0.5)
                ).unwrap(),
                Segment::line(P2::new(2.0, 0.5), P2::new(0.0, 0.5))
                    .unwrap(),
                Segment::line(P2::new(0.0, 0.5), P2::new(0.0, 0.0))
                    .unwrap(),
            ]).unwrap(),
            &Path::new(vec![
                Segment::line(P2::new(1.0, 1.0), P2::new(1.0, -1.0))
                    .unwrap(),
                Segment::line(P2::new(1.0, -1.0), P2::new(2.0, -1.0))
                    .unwrap(),
                Segment::line(P2::new(2.0, -1.0), P2::new(2.0, 1.0))
                    .unwrap(),
                Segment::line(P2::new(2.0, 1.0), P2::new(1.0, 1.0))
                    .unwrap(),
            ]).unwrap(),
        ).intersect(),
        IntersectionResult::Intersecting(vec![
            Intersection {
                along_a: 1.0,
                along_b: 1.0,
                position: P2::new(1.0, 0.0),
            },
            Intersection {
                along_a: 2.0,
                along_b: 4.0,
                position: P2::new(2.0, 0.0),
            },
            Intersection {
                along_a: 4.8561945,
                along_b: 4.5,
                position: P2::new(2.0, 0.5),
            },
            Intersection {
                along_a: 5.8561945,
                along_b: 0.5,
                position: P2::new(1.0, 0.5),
            },
        ])
    );
}

#[test]
fn path_intersecting_before_curved_road() {
    use V2;
    //     _
    //    | |
    // ,--(-(--.
    // '--(-(.  \
    //    |_| `--'

    assert_eq!(
        (
            &Path::new(vec![
                Segment::line(P2::new(0.0, 0.0), P2::new(2.0, 0.0))
                    .unwrap(),
                Segment::arc_with_direction(
                    P2::new(2.0, 0.0),
                    V2::new(1.0, 0.0),
                    P2::new(3.0, 1.0)
                ).unwrap(),
                Segment::line(P2::new(3.0, 1.0), P2::new(2.5, 1.0))
                    .unwrap(),
                Segment::arc_with_direction(
                    P2::new(2.5, 1.0),
                    V2::new(0.0, -1.0),
                    P2::new(2.0, 0.5)
                ).unwrap(),
                Segment::line(P2::new(2.0, 0.5), P2::new(0.0, 0.5))
                    .unwrap(),
                Segment::line(P2::new(0.0, 0.5), P2::new(0.0, 0.0))
                    .unwrap(),
            ]).unwrap(),
            &Path::new(vec![
                Segment::line(
                    P2::new(1.0 - TINY_BIT, 1.0),
                    P2::new(1.0 - TINY_BIT, -1.0)
                ).unwrap(),
                Segment::line(
                    P2::new(1.0 - TINY_BIT, -1.0),
                    P2::new(2.0 - TINY_BIT, -1.0)
                ).unwrap(),
                Segment::line(
                    P2::new(2.0 - TINY_BIT, -1.0),
                    P2::new(2.0 - TINY_BIT, 1.0)
                ).unwrap(),
                Segment::line(
                    P2::new(2.0 - TINY_BIT, 1.0),
                    P2::new(1.0 - TINY_BIT, 1.0)
                ).unwrap(),
            ]).unwrap(),
        ).intersect(),
        IntersectionResult::Intersecting(vec![
            Intersection {
                along_a: 1.0 - TINY_BIT,
                along_b: 1.0,
                position: P2::new(1.0 - TINY_BIT, 0.0),
            },
            Intersection {
                along_a: 2.0 - TINY_BIT,
                along_b: 4.0,
                position: P2::new(2.0 - TINY_BIT, 0.0),
            },
            Intersection {
                along_a: 4.8561945 + TINY_BIT,
                along_b: 4.5,
                position: P2::new(2.0 - TINY_BIT, 0.5),
            },
            Intersection {
                along_a: 5.8561945 + TINY_BIT,
                along_b: 0.5,
                position: P2::new(1.0 - TINY_BIT, 0.5),
            },
        ])
    );
}

#[test]
fn path_intersecting_after_curved_road() {
    use V2;
    //     _
    //    | |
    // ,--)-)--.
    // '--)-).  \
    //    |_| `--'

    assert_eq!(
        (
            &Path::new(vec![
                Segment::line(P2::new(0.0, 0.0), P2::new(2.0, 0.0))
                    .unwrap(),
                Segment::arc_with_direction(
                    P2::new(2.0, 0.0),
                    V2::new(1.0, 0.0),
                    P2::new(3.0, 1.0)
                ).unwrap(),
                Segment::line(P2::new(3.0, 1.0), P2::new(2.5, 1.0))
                    .unwrap(),
                Segment::arc_with_direction(
                    P2::new(2.5, 1.0),
                    V2::new(0.0, -1.0),
                    P2::new(2.0, 0.5)
                ).unwrap(),
                Segment::line(P2::new(2.0, 0.5), P2::new(0.0, 0.5))
                    .unwrap(),
                Segment::line(P2::new(0.0, 0.5), P2::new(0.0, 0.0))
                    .unwrap(),
            ]).unwrap(),
            &Path::new(vec![
                Segment::line(
                    P2::new(1.0 + TINY_BIT, 1.0),
                    P2::new(1.0 + TINY_BIT, -1.0)
                ).unwrap(),
                Segment::line(
                    P2::new(1.0 + TINY_BIT, -1.0),
                    P2::new(2.0 + TINY_BIT, -1.0)
                ).unwrap(),
                Segment::line(
                    P2::new(2.0 + TINY_BIT, -1.0),
                    P2::new(2.0 + TINY_BIT, 1.0)
                ).unwrap(),
                Segment::line(
                    P2::new(2.0 + TINY_BIT, 1.0),
                    P2::new(1.0 + TINY_BIT, 1.0)
                ).unwrap(),
            ]).unwrap(),
        ).intersect(),
        IntersectionResult::Intersecting(vec![
            Intersection {
                along_a: 1.0 + TINY_BIT,
                along_b: 1.0,
                position: P2::new(1.0 + TINY_BIT, 0.0),
            },
            Intersection {
                along_a: 2.0066593,
                along_b: 4.000022,
                position: P2::new(2.0 + TINY_BIT, 0.000022172928),
            },
            Intersection {
                along_a: 4.8561945 - TINY_BIT,
                along_b: 4.5000443,
                position: P2::new(2.0 + TINY_BIT, 0.50004435),
            },
            Intersection {
                along_a: 5.8561945 - TINY_BIT,
                along_b: 0.5,
                position: P2::new(1.0 + TINY_BIT, 0.5),
            },
        ])
    );
}
