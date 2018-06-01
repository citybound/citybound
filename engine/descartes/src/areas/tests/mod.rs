use super::{N, P2, Path, Area, PrimitiveArea, THICKNESS, Segment, VecLike};

fn svg_test(file_path: &str) {
    use std::fs;
    use std::io::Read;
    use {THICKNESS, RoughEq};

    let mut file = fs::File::open(file_path).unwrap();

    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let mut clip_area = None;
    let mut subject_area = None;
    let mut expected_result_primitive_areas = VecLike::new();

    let path_substrs = contents.split("<path").filter(
        |string| string.contains("d="),
    );

    for path_substr in path_substrs {
        let path_commands_idx = path_substr.find(" d=").unwrap();
        let path_commands = path_substr[path_commands_idx + 4..]
            .splitn(2, '"')
            .next()
            .unwrap();

        let id_idx = path_substr.find(" id=").unwrap();
        let id = path_substr[id_idx + 5..].splitn(2, '"').next().unwrap();

        println!("Found path {} with id {}", path_commands, id);

        let area = PrimitiveArea::new(Path::from_svg(path_commands).unwrap()).unwrap();

        if id == "subject" {
            subject_area = Some(Area::new(Some(area).into_iter().collect()));
        } else if id == "clip" {
            clip_area = Some(Area::new(Some(area).into_iter().collect()));
        } else if id.starts_with("result") {
            expected_result_primitive_areas.push(area);
        }
    }

    let expected_result_area = Area::new(expected_result_primitive_areas);

    let subject_area = subject_area.expect("should have subject");
    let clip_area = clip_area.expect("should have clip");

    let split_result = subject_area.split(&clip_area);

    println!("{}", split_result.debug_svg());

    let result_area = if file_path.ends_with("intersection.svg") {
        split_result.intersection()
    } else if file_path.ends_with("union.svg") {
        split_result.union()
    } else if file_path.ends_with("difference.svg") {
        split_result.a_minus_b()
    } else if file_path.ends_with("not.svg") {
        split_result.b_minus_a()
    } else {
        panic!("unsupported file ending");
    };

    assert_eq!(
        expected_result_area.primitives.len(),
        result_area.primitives.len()
    );

    assert!((&result_area).rough_eq_by(&expected_result_area, THICKNESS));
}

#[test]
fn clip_1_difference() {
    svg_test("./src/areas/tests/1_difference.svg");
}

#[test]
fn clip_1_intersection() {
    svg_test("./src/areas/tests/1_intersection.svg");
}

#[test]
fn clip_1_union() {
    svg_test("./src/areas/tests/1_union.svg");
}

#[test]
fn clip_2_difference() {
    svg_test("./src/areas/tests/2_difference.svg");
}

#[test]
fn clip_2_intersection() {
    svg_test("./src/areas/tests/2_intersection.svg");
}

#[test]
fn clip_3_difference() {
    svg_test("./src/areas/tests/3_difference.svg");
}

#[test]
fn area_intersecting_at_curved_road() {
    use V2;
    //     _
    //    | |
    // ,--|-|--.
    // '--|-|.  \
    //    |_| `--'

    assert_eq!(
        &*Area::new_simple(
            Path::new(vec![
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
        ).unwrap()
            .split(&Area::new_simple(
                Path::new(vec![
                    Segment::line(P2::new(1.0, 1.0), P2::new(1.0, -1.0))
                        .unwrap(),
                    Segment::line(P2::new(1.0, -1.0), P2::new(2.0, -1.0))
                        .unwrap(),
                    Segment::line(P2::new(2.0, -1.0), P2::new(2.0, 1.0))
                        .unwrap(),
                    Segment::line(P2::new(2.0, 1.0), P2::new(1.0, 1.0))
                        .unwrap(),
                ]).unwrap(),
            ).unwrap())
            .intersection()
            .primitives
            [0]
            .boundary
            .segments,
        &[
            Segment::line(P2::new(2.0, 0.5), P2::new(1.0, 0.5)).unwrap(),
            Segment::line(P2::new(1.0, 0.5), P2::new(1.0, 0.0)).unwrap(),
            Segment::line(P2::new(1.0, 0.0), P2::new(2.0, 0.0)).unwrap(),
            Segment::line(P2::new(2.0, 0.0), P2::new(2.0, 0.5)).unwrap(),
        ]
    );
}

const TINY_BIT: N = THICKNESS / 3.0;

#[test]
fn area_intersecting_before_curved_road() {
    use V2;
    //     _
    //    | |
    // ,--(-(--.
    // '--(-(.  \
    //    |_| `--'

    assert_eq!(
        &*Area::new_simple(
            Path::new(vec![
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
        ).unwrap()
            .split(&Area::new_simple(
                Path::new(vec![
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
            ).unwrap())
            .intersection()
            .primitives
            [0]
            .boundary
            .segments,
        &[
            Segment::line(P2::new(2.0 - TINY_BIT, 0.5), P2::new(1.0 - TINY_BIT, 0.5)).unwrap(),
            Segment::line(P2::new(1.0 - TINY_BIT, 0.5), P2::new(1.0 - TINY_BIT, 0.0)).unwrap(),
            Segment::line(P2::new(1.0 - TINY_BIT, 0.0), P2::new(2.0 - TINY_BIT, 0.0)).unwrap(),
            Segment::line(P2::new(2.0 - TINY_BIT, 0.0), P2::new(2.0 - TINY_BIT, 0.5)).unwrap(),
        ]
    );
}
