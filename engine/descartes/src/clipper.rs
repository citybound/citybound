/// A line-arc shape clipping algorithm based on
///
/// "An Extension of Polygon Clipping To Resolve Degenerate Cases"
/// by Dae Hyun Kima & Myoung-Jun Kim

use super::shapes::SimpleShape;
use super::{N, Shape, Segment, Path, FiniteCurve};
use super::path::PathError;
use super::shapes::ShapeError;
use super::PointOnShapeLocation::*;
use super::intersect::Intersect;

const DEBUG_PRINT: bool = false;

#[derive(Copy, Clone, Debug)]
pub enum Mode {
    Intersection,
    Union,
    Difference,
    Not,
    Split,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Role {
    None,
    Entry,
    Exit,
    EntryExit,
    ExitEntry,
}

#[derive(Copy, Clone, Debug)]
pub enum Direction {
    ForwardStay,
    ForwardSwitch,
    BackwardStay,
    BackwardSwitch,
}

#[derive(PartialEq, Debug)]
struct Intersection {
    along: [N; 2],
    role: [Role; 2],
    next: [usize; 2],
    prev: [usize; 2],
    partner: [Option<usize>; 2],
}

const SUBJECT: usize = 0;
const CLIP: usize = 1;

const SUBJECT_AND_CLIP: [usize; 2] = [SUBJECT, CLIP];

fn other_focus(focus: usize) -> usize {
    if focus == SUBJECT { CLIP } else { SUBJECT }
}

#[derive(Debug)]
pub enum ClipError {
    UnimplementedTransition(Mode, Direction, Role),
    ImpossibleTransition(Mode, Direction, Role),
    InvalidSegmentBetweenIntersections,
    InvalidResultPath(PathError),
    InvalidResultShape(ShapeError),
    UnimplementedComplexResult,
    InfiniteLoop,
}

pub fn clip<S: SimpleShape>(
    mode: Mode,
    subject_shape: &S,
    clip_shape: &S,
) -> Result<Vec<S>, ClipError> {
    let shapes = [subject_shape, clip_shape];

    if DEBUG_PRINT {
        println!(
            r#"
            <svg width="320" height="320" viewbox="-0.5 -0.5 2.5 2.5" xmlns="http://www.w3.org/2000/svg">
                <g fill="none" stroke="rgba(0, 0, 255, 0.3)" stroke-width="0.02" marker-end="url(#subj_marker)">
                    <marker id="subj_marker" viewBox="0 0 6 6" refX="6" refY="3" markerUnits="strokeWidth" orient="auto">
                        <path d="M 0 0 L 6 3 L 0 6 z" fill="rgba(0, 0, 255, 0.3)"/>
                    </marker>
                    <path d="{}"/>
                </g>
                <g fill="none" stroke="rgba(255, 0, 0, 0.3)" stroke-width="0.02" marker-end="url(#clip_marker)">
                    <marker id="clip_marker" viewBox="0 0 6 6" refX="6" refY="3" markerUnits="strokeWidth" orient="auto">
                        <path d="M 0 0 L 6 3 L 0 6 z" fill="rgba(255, 0, 0, 0.3)"/>
                    </marker>
                    <path d="{}"/>
                </g>
            
        "#,
        shapes[SUBJECT].outline().to_svg(),
        shapes[CLIP].outline().to_svg()
        );
    }

    // get raw intersections and put them into relative order along the subject
    // and clip shape using insertion sort on a doubly-linked list

    let raw_intersections = (subject_shape.outline(), clip_shape.outline()).intersect();

    if raw_intersections.len() < 2 {
        // TODO: handle full containment
        // TODO: handle full containment with single intersection that touches

        let all_subject_in_clip = subject_shape
            .outline()
            .segments()
            .iter()
            .map(|segment| segment.start())
            .all(|point| clip_shape.contains(point));

        let all_clip_in_subject = clip_shape
            .outline()
            .segments()
            .iter()
            .map(|segment| segment.start())
            .all(|point| subject_shape.contains(point));

        return match mode {
            Mode::Union => {
                if all_subject_in_clip {
                    Ok(vec![clip_shape.clone()])
                } else if all_clip_in_subject {
                    Ok(vec![subject_shape.clone()])
                } else {
                    Err(ClipError::UnimplementedComplexResult)
                }
            }
            Mode::Intersection => {
                if all_subject_in_clip {
                    Ok(vec![subject_shape.clone()])
                } else if all_clip_in_subject {
                    Ok(vec![clip_shape.clone()])
                } else {
                    Ok(vec![])
                }
            }
            Mode::Difference => {
                if all_subject_in_clip {
                    Ok(vec![])
                } else if all_clip_in_subject {
                    Err(ClipError::UnimplementedComplexResult)
                } else {
                    Ok(vec![subject_shape.clone()])
                }
            }
            _ => Err(ClipError::UnimplementedComplexResult),
        };
    }

    let mut intersections = Vec::<Intersection>::with_capacity(raw_intersections.len());

    {
        const START_SENTINEL: usize = ::std::usize::MAX - 1;
        const END_SENTINEL: usize = ::std::usize::MAX;

        intersections.push(Intersection {
            along: [raw_intersections[0].along_a, raw_intersections[0].along_b],
            role: [Role::None, Role::None],
            next: [END_SENTINEL, END_SENTINEL],
            prev: [START_SENTINEL, START_SENTINEL],
            partner: [None, None],
        });

        let mut first = [0, 0];
        let mut last = [0, 0];

        for raw_intersection in &raw_intersections[1..] {
            let along = [raw_intersection.along_a, raw_intersection.along_b];
            let mut next = first.clone();
            let mut prev = [START_SENTINEL, START_SENTINEL];

            let self_i = intersections.len();

            for &focus in &SUBJECT_AND_CLIP {
                while next[focus] != END_SENTINEL &&
                    intersections[next[focus]].along[focus] < along[focus]
                {
                    prev[focus] = next[focus];
                    next[focus] = intersections[next[focus]].next[focus];
                }

                if prev[focus] == START_SENTINEL {
                    first[focus] = self_i;
                } else {
                    intersections[prev[focus]].next[focus] = self_i;
                }

                if next[focus] == END_SENTINEL {
                    last[focus] = self_i;
                } else {
                    intersections[next[focus]].prev[focus] = self_i;
                }
            }

            intersections.push(Intersection {
                along,
                role: [Role::None, Role::None],
                next,
                prev,
                partner: [None, None],
            });
        }

        // Close the loop
        for &focus in &SUBJECT_AND_CLIP {
            intersections[first[focus]].prev[focus] = last[focus];
            intersections[last[focus]].next[focus] = first[focus];
        }
    }

    // Determine roles based on prev / next midpoint
    // TODO: roles of the subject chain can more easily deduced by the roles of the clip chain

    fn midpoint_between(length: N, start: N, end: N) -> N {
        if start < end {
            (start + end) / 2.0
        } else {
            let distance = (length - start) + end;

            if distance / 2.0 < (length - start) {
                start + distance / 2.0
            } else {
                end - distance / 2.0
            }
        }
    }

    for &focus in &SUBJECT_AND_CLIP {
        for i in 0..intersections.len() {
            let role = {
                let intersection = &intersections[i];
                let prev_intersection = &intersections[intersection.prev[focus]];
                let prev_midpoint_along = midpoint_between(
                    shapes[focus].outline().length(),
                    prev_intersection.along[focus],
                    intersection.along[focus],
                );
                let prev_midpoint = shapes[focus].outline().along(prev_midpoint_along);
                let prev_location = shapes[other_focus(focus)].location_of(prev_midpoint);

                let next_intersection = &intersections[intersection.next[focus]];
                let next_midpoint_along = midpoint_between(
                    shapes[focus].outline().length(),
                    intersection.along[focus],
                    next_intersection.along[focus],
                );
                let next_midpoint = shapes[focus].outline().along(next_midpoint_along);
                let next_location = shapes[other_focus(focus)].location_of(next_midpoint);

                match (prev_location, next_location) {
                    (OnEdge, Outside) |
                    (Inside, OnEdge) |
                    (Inside, Outside) => Role::Exit,
                    (OnEdge, Inside) |
                    (Outside, OnEdge) |
                    (Outside, Inside) => Role::Entry,
                    (Inside, Inside) => Role::ExitEntry,
                    (Outside, Outside) => Role::EntryExit,
                    _ => Role::None,
                }
            };

            intersections[i].role[focus] = role;
        }
    }

    if DEBUG_PRINT {
        println!(
            r#"
                <g font-size="0.1" fill="rgba(0, 0, 255, 0.3)">
                    {}
                </g>
                <g font-size="0.1" fill="rgba(255, 0, 0, 0.3)">
                    {}
                </g>
        "#,
            intersections
                .iter()
                .map(|intersection| {
                    format!(
                        r#"<text x="{}" y={}>{:?}</text> "#,
                        shapes[SUBJECT].outline().along(intersection.along[SUBJECT]).x,
                        shapes[SUBJECT].outline().along(intersection.along[SUBJECT]).y,
                        intersection.role[SUBJECT]
                    )
                })
                .collect::<Vec<_>>()
                .join(" "),
            intersections
                .iter()
                .map(|intersection| {
                    format!(
                        r#"<text x="{}" y={}>{:?}</text> "#,
                        shapes[CLIP].outline().along(intersection.along[CLIP]).x,
                        shapes[CLIP].outline().along(intersection.along[CLIP]).y + 0.1,
                        intersection.role[CLIP]
                    )
                })
                .collect::<Vec<_>>()
                .join(" "),
        );
    }


    // TODO: set couples

    // TODO: deal with cross-change situations



    // Find start vertex

    let mut result_shapes = Vec::new();

    while let Some((start_intersection_i, start_focus)) =
        intersections
            .iter()
            .enumerate()
            .filter_map(|(potential_start_i, potential_start)| {

                let maybe_start_focus = SUBJECT_AND_CLIP.iter().find(|&&focus| {
                    potential_start.role[focus] != Role::None &&
                        if let Some(partner_idx) = potential_start.partner[focus] {
                            if intersections[partner_idx].role[focus] == Role::None {
                                // Once a flag of a couple has been deleted, both of the
                                // vertices can no longer be used as a starting vertex.
                                false
                            } else {
                                // If the couple with each flag still set have (en, en),
                                // the second vertex can be selected as a starting vertex;
                                // if the couple have (ex, ex) the first vertex is selected.
                                (potential_start.role[focus] == Role::Entry &&
                                     potential_start.prev[focus] == partner_idx) ||
                                    (potential_start.role[focus] == Role::Exit &&
                                         potential_start.next[focus] == partner_idx)
                            }
                        } else {
                            true
                        }
                });

                maybe_start_focus.map(|found_focus| (potential_start_i, *found_focus))
            })
            .next()
    {
        // Walk the chain & collect output vertices
        let mut current_intersection_i = start_intersection_i;
        let mut focus = start_focus;
        let mut direction = Direction::ForwardStay;
        let mut segments = Vec::<Segment>::new();

        fn traverse_step(
            current_role: Role,
            current_direction: Direction,
            mode: Mode,
        ) -> Result<(Direction, Role), ClipError> {
            use self::Direction::*;
            use self::Role::*;

            Ok(match mode {
                Mode::Union => {
                    match (current_direction, current_role) {
                        (ForwardStay, Entry) => (ForwardSwitch, None),
                        (ForwardStay, EntryExit) => (ForwardStay, Exit),
                        (ForwardStay, Exit) |
                        (ForwardSwitch, Exit) => (ForwardStay, None),
                        (ForwardStay, ExitEntry) => (ForwardSwitch, Entry),
                        (ForwardSwitch, Entry) => {
                            return Err(ClipError::ImpossibleTransition(
                                mode,
                                current_direction,
                                current_role,
                            ))
                        }
                        (direction, None) => (direction, None),
                        _ => {
                            return Err(ClipError::UnimplementedTransition(
                                mode,
                                current_direction,
                                current_role,
                            ))
                        }
                    }
                }
                Mode::Intersection => {
                    match (current_direction, current_role) {
                        (ForwardStay, Entry) |
                        (ForwardSwitch, Entry) => (ForwardStay, None),
                        (ForwardStay, EntryExit) => (ForwardStay, Exit),
                        (ForwardStay, Exit) => (ForwardSwitch, None),
                        (ForwardStay, ExitEntry) => (ForwardStay, Entry),
                        (ForwardSwitch, Exit) => {
                            return Err(ClipError::ImpossibleTransition(
                                mode,
                                current_direction,
                                current_role,
                            ))
                        }
                        (direction, None) => (direction, None),
                        _ => {
                            return Err(ClipError::UnimplementedTransition(
                                mode,
                                current_direction,
                                current_role,
                            ))
                        }
                    }
                }
                Mode::Difference => {
                    match (current_direction, current_role) {
                        (ForwardStay, Entry) => (BackwardSwitch, None),
                        (ForwardStay, Exit) |
                        (ForwardSwitch, Exit) => (ForwardStay, None),
                        (ForwardStay, EntryExit) => (BackwardSwitch, Exit),
                        (BackwardSwitch, Exit) => (BackwardStay, None),
                        (BackwardSwitch, ExitEntry) => (BackwardStay, Entry),
                        (BackwardStay, Entry) => (ForwardSwitch, None),
                        (ForwardSwitch, Entry) => {
                            return Err(ClipError::ImpossibleTransition(
                                mode,
                                current_direction,
                                current_role,
                            ))
                        }
                        (direction, None) => (direction, None),
                        _ => {
                            return Err(ClipError::UnimplementedTransition(
                                mode,
                                current_direction,
                                current_role,
                            ))
                        }
                    }
                }
                _ => unimplemented!(),
            })
        }

        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 500;
        loop {
            let (new_role, next_focus, next_intersection_i) = {
                let current_intersection = &intersections[current_intersection_i];

                let (new_direction, new_role) =
                    traverse_step(current_intersection.role[focus], direction, mode)?;

                if DEBUG_PRINT {
                    println!(
                        "<!-- {:?} {:?} {:?} -> {:?} {:?} -->",
                        current_intersection.role[focus],
                        direction,
                        focus,
                        new_direction,
                        new_role
                    );
                }

                let (next_intersection_i, next_focus) = match new_direction {
                    Direction::ForwardStay => (current_intersection.next[focus], focus),
                    Direction::BackwardStay => (current_intersection.next[focus], focus),
                    Direction::ForwardSwitch |
                    Direction::BackwardSwitch => (current_intersection_i, other_focus(focus)),
                };

                let next_intersection = &intersections[next_intersection_i];

                match new_direction {
                    Direction::ForwardStay => {
                        let hopefully_subsection =
                            shapes[next_focus].outline().subsection(
                                current_intersection.along[next_focus],
                                next_intersection.along[next_focus],
                            );

                        match hopefully_subsection {
                            Some(subsection) => segments.extend(subsection.segments()),
                            None => return Err(ClipError::InvalidSegmentBetweenIntersections),
                        }
                    }
                    Direction::BackwardStay => {
                        let hopefully_subsection =
                            shapes[next_focus].outline().subsection(
                                next_intersection.along[next_focus],
                                current_intersection.along[next_focus],
                            );

                        match hopefully_subsection {
                            Some(subsection) => segments.extend(subsection.reverse().segments()),
                            None => return Err(ClipError::InvalidSegmentBetweenIntersections),
                        }
                    }
                    _ => {}
                }

                direction = new_direction;

                (new_role, next_focus, next_intersection_i)
            };

            intersections[current_intersection_i].role[focus] = new_role;

            focus = next_focus;

            current_intersection_i = next_intersection_i;

            if current_intersection_i == start_intersection_i && focus == start_focus {
                break;
            }

            if iterations > MAX_ITERATIONS {
                return Err(ClipError::InfiniteLoop);
            }

            iterations += 1;
        }

        // TODO: maybe this can be caught earlier
        if !segments.is_empty() {
            if DEBUG_PRINT {
                println!(r#"<!-- SEGMENTS {:?} -->"#, segments);
            }

            let path = match S::P::new_welded(segments) {
                Ok(path) => path,
                Err(err) => {
                    return Err(ClipError::InvalidResultPath(err));
                }
            };

            if DEBUG_PRINT {
                println!(
                r#"
                    <g fill="none" stroke="rgba(0, 0, 0, 0.2)" stroke-width="0.05" marker-end="url(#result_marker)">
                        <marker id="result_marker" viewBox="0 0 6 6" refX="6" refY="3" markerUnits="strokeWidth" orient="auto">
                            <path d="M 0 0 L 6 3 L 0 6 z" fill="rgba(0, 0, 0, 0.1)"/>
                        </marker>
                        <path d="{}"/>
                    </g>
            "#,
                path.to_svg(),
                
            );
            }

            result_shapes.push(match SimpleShape::new(path) {
                Ok(shape) => shape,
                Err(err) => {
                    return Err(ClipError::InvalidResultShape(err));
                }
            });
        }
    }

    Ok(result_shapes)
}


#[test]
fn test() {
    use super::P2;
    use super::path::VecPath;

    struct TestShape {
        outline: VecPath,
    }

    impl SimpleShape for TestShape {
        type P = VecPath;

        fn outline(&self) -> &VecPath {
            &self.outline
        }

        fn new_unchecked(outline: VecPath) -> Self {
            TestShape { outline }
        }
    }

    let subject = TestShape::new(
        VecPath::new(vec![
            Segment::line(P2::new(0.0, 0.0), P2::new(1.0, 0.0))
                .unwrap(),
            Segment::line(P2::new(1.0, 0.0), P2::new(1.0, 1.0))
                .unwrap(),
            Segment::line(P2::new(1.0, 1.0), P2::new(0.0, 1.0))
                .unwrap(),
            Segment::line(P2::new(0.0, 1.0), P2::new(0.0, 0.0))
                .unwrap(),
        ]).unwrap(),
    ).unwrap();

    let clip = TestShape::new(
        VecPath::new(vec![
            Segment::line(P2::new(0.5, 0.5), P2::new(1.5, 0.5))
                .unwrap(),
            Segment::line(P2::new(1.5, 0.5), P2::new(1.5, 1.5))
                .unwrap(),
            Segment::line(P2::new(1.5, 1.5), P2::new(0.5, 1.5))
                .unwrap(),
            Segment::line(P2::new(0.5, 1.5), P2::new(0.5, 0.5))
                .unwrap(),
        ]).unwrap(),
    ).unwrap();

    self::clip(Mode::Union, &subject, &clip);
    self::clip(Mode::Intersection, &subject, &clip);
    self::clip(Mode::Difference, &subject, &clip);
}