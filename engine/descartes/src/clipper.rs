/// A line-arc shape clipping algorithm based on
///
/// "An Extension of Polygon Clipping To Resolve Degenerate Cases"
/// by Dae Hyun Kima & Myoung-Jun Kim

use super::shapes::SimpleShape;
use super::{N, Shape, Segment, PointOnShapeLocation, Path, FiniteCurve};
use super::PointOnShapeLocation::*;
use super::intersect::Intersect;
use super::{RoughlyComparable, THICKNESS};
use ordered_float::OrderedFloat;
use itertools::Itertools;
use std::collections::BinaryHeap;

const DEBUG_PRINT: bool = true;

#[derive(Copy, Clone)]
pub enum Mode {
    Intersection,
    Union,
    Difference,
    Not,
    Split,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Role {
    None,
    Entry,
    Exit,
    EntryExit,
    ExitEntry,
}

#[derive(Copy, Clone, Debug)]
enum Direction {
    ForwardStay,
    ForwardSwitch,
    BackwardStay,
    BackwardSwitch,
}

#[derive(PartialEq)]
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

pub fn clip<S: SimpleShape>(mode: Mode, subject_shape: &S, clip_shape: &S) -> Vec<S> {
    let shapes = [subject_shape, clip_shape];

    // get raw intersections and put them into relative order along the subject
    // and clip shape using insertion sort on a doubly-linked list

    let raw_intersections = (subject_shape.outline(), clip_shape.outline()).intersect();

    if raw_intersections.is_empty() {
        // TODO: handle full containment
        // TODO: handle full containment with single intersection that touches
        return Vec::new();
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
            let mut prev = first.clone();

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
            start + end / 2.0
        } else {
            let distance = (length - start) + end;

            if distance / 2.0 < (length - start) {
                start + distance / 2.0
            } else {
                end + (start + distance / 2.0 - length)
            }
        }
    }

    for &focus in &SUBJECT_AND_CLIP {
        for i in 0..intersections.len() {
            let role = {
                let intersection = &intersections[i];
                let prev_intersection = &intersections[intersection.prev[focus]];
                let prev_midpoint = shapes[focus].outline().along(midpoint_between(
                    shapes[focus].outline().length(),
                    prev_intersection.along[focus],
                    intersection.along[focus],
                ));
                let prev_location = shapes[other_focus(focus)].location_of(prev_midpoint);

                let next_intersection = &intersections[intersection.next[focus]];
                let next_midpoint = shapes[focus].outline().along(midpoint_between(
                    shapes[focus].outline().length(),
                    intersection.along[focus],
                    next_intersection.along[focus],
                ));
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
        ) -> (Direction, Role) {
            use self::Direction::*;
            use self::Role::*;

            match mode {
                Mode::Union => {
                    match (current_direction, current_role) {
                        (ForwardStay, Entry) => (ForwardSwitch, None),
                        (ForwardStay, EntryExit) => (ForwardStay, Exit),
                        (ForwardStay, Exit) |
                        (ForwardSwitch, Exit) => (ForwardStay, None),
                        (ForwardStay, ExitEntry) => (ForwardSwitch, Entry),
                        (ForwardSwitch, Entry) => unreachable!(),
                        (direction, None) => (direction, None),
                        _ => unimplemented!(),
                    }
                }
                Mode::Intersection => {
                    match (current_direction, current_role) {
                        (ForwardStay, Entry) |
                        (ForwardSwitch, Entry) => (ForwardStay, None),
                        (ForwardStay, EntryExit) => (ForwardStay, Exit),
                        (ForwardStay, Exit) => (ForwardSwitch, None),
                        (ForwardStay, ExitEntry) => (ForwardStay, Entry),
                        (ForwardSwitch, Exit) => unreachable!(),
                        (direction, None) => (direction, None),
                        _ => unimplemented!(),
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
                        (ForwardSwitch, Entry) => unreachable!(),
                        (direction, None) => (direction, None),
                        _ => unimplemented!(),
                    }
                }
                _ => unimplemented!(),
            }
        }

        loop {
            let (new_role, next_intersection_i) = {
                let current_intersection = &intersections[current_intersection_i];

                let (new_direction, new_role) =
                    traverse_step(current_intersection.role[focus], direction, mode);

                if DEBUG_PRINT {
                    println!(
                        "<!-- {:?} {:?} -> {:?} {:?} -->",
                        current_intersection.role[focus],
                        direction,
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
                        segments.extend(
                            shapes[next_focus]
                                .outline()
                                .subsection(
                                    current_intersection.along[next_focus],
                                    next_intersection.along[next_focus],
                                )
                                .expect("Intersections should always have valid subsections between them")
                                .segments(),
                        )
                    }
                    Direction::BackwardStay => {
                        segments.extend(
                            shapes[next_focus]
                                .outline()
                                .subsection(
                                    next_intersection.along[next_focus],
                                    current_intersection.along[next_focus],
                                )
                                .expect("Intersections should always have valid subsections between them")
                                .reverse()
                                .segments(),
                        )
                    }
                    _ => {}
                }

                direction = new_direction;
                focus = next_focus;

                (new_role, next_intersection_i)
            };

            intersections[current_intersection_i].role[focus] = new_role;

            current_intersection_i = next_intersection_i;

            if current_intersection_i == start_intersection_i {
                break;
            }
        }

        if DEBUG_PRINT {
            println!(
                r#"
                    <g stroke="rgba(0, 0, 0, 0.2)" stroke-width="0.05" marker-end="url(#result_marker)">
                        <marker id="result_marker" viewBox="0 0 6 6" refX="6" refY="3" markerUnits="strokeWidth" orient="auto">
                            <path d="M 0 0 L 6 3 L 0 6 z" fill="rgba(0, 0, 0, 0.1)"/>
                        </marker>
                        {}
                    </g>
            "#,
                segments
                    .iter()
                    .map(|segment| format!(r#"<path d="{}"/> "#, segment.to_svg()))
                    .collect::<Vec<_>>()
                    .join(" "),
                
            );
        }

        // TODO: maybe this can be caught earlier
        if !segments.is_empty() {
            result_shapes.push(
                SimpleShape::new(S::P::new(segments).expect(
                    "Resulting clip path should be valid",
                )).expect("Resulting clip shape should be valid"),
            );
        }
    }

    result_shapes
}


#[test]
fn test() {
    use super::P2;
    use super::path::VecPath;

    let subject = SimpleShape::new(VecPath::new(vec![
        Segment::line(P2::new(0.0, 0.0), P2::new(1.0, 0.0))
            .unwrap(),
        Segment::line(P2::new(1.0, 0.0), P2::new(1.0, 1.0))
            .unwrap(),
        Segment::line(P2::new(1.0, 1.0), P2::new(0.0, 1.0))
            .unwrap(),
        Segment::line(P2::new(0.0, 1.0), P2::new(0.0, 0.0))
            .unwrap(),
    ]));

    let clip = SimpleShape::new(VecPath::new(vec![
        Segment::line(P2::new(0.5, 0.5), P2::new(1.5, 0.5))
            .unwrap(),
        Segment::line(P2::new(1.5, 0.5), P2::new(1.5, 1.5))
            .unwrap(),
        Segment::line(P2::new(1.5, 1.5), P2::new(0.5, 1.5))
            .unwrap(),
        Segment::line(P2::new(0.5, 1.5), P2::new(0.5, 0.5))
            .unwrap(),
    ]));

    self::clip(Mode::Union, &subject, &clip);
    self::clip(Mode::Intersection, &subject, &clip);
    self::clip(Mode::Difference, &subject, &clip);
}