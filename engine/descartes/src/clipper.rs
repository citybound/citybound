/// A line-arc shape clipping algorithm based on
///
/// "An Extension of Polygon Clipping To Resolve Degenerate Cases"
/// by Dae Hyun Kima & Myoung-Jun Kim

use super::shapes::SimpleShape;
use super::{Shape, Segment, PointOnShapeLocation, Path, FiniteCurve};
use super::PointOnShapeLocation::*;
use super::intersect::Intersect;
use super::{RoughlyComparable, THICKNESS};
use ordered_float::OrderedFloat;
use itertools::Itertools;
use std::collections::BinaryHeap;

const DEBUG_PRINT: bool = false;

#[derive(Copy, Clone)]
pub enum Mode {
    Intersection,
    Union,
    Difference,
    Not,
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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct VertexRef(usize);

struct VertexData {
    forward_segment: Segment,
    role: Role,
    neighbor: Option<VertexRef>,
    partner: Option<VertexRef>,
}

impl VertexData {
    fn bare_vertex(forward_segment: Segment) -> VertexData {
        VertexData {
            forward_segment,
            role: Role::None,
            neighbor: None,
            partner: None,
        }
    }

    fn bare_intersection(forward_segment: Segment) -> VertexData {
        VertexData {
            forward_segment,
            role: Role::None,
            neighbor: None,
            partner: None,
        }
    }
}

struct Vertex {
    next: VertexRef,
    prev: VertexRef,
    data: VertexData,
}

impl ::std::ops::Deref for Vertex {
    type Target = VertexData;

    fn deref(&self) -> &VertexData {
        &self.data
    }
}

impl ::std::ops::DerefMut for Vertex {
    fn deref_mut(&mut self) -> &mut VertexData {
        &mut self.data
    }
}

struct VertexArena {
    vertices: Vec<Vertex>,
}

impl VertexArena {
    fn new() -> VertexArena {
        VertexArena { vertices: Vec::new() }
    }

    fn get(&self, vertex_ref: VertexRef) -> &Vertex {
        &self.vertices[vertex_ref.0]
    }

    fn get_mut(&mut self, vertex_ref: VertexRef) -> &mut Vertex {
        &mut self.vertices[vertex_ref.0]
    }

    fn add_chain<I: ExactSizeIterator<Item = VertexData>>(&mut self, datas: I) -> VertexRef {
        let start_index = self.vertices.len();
        let n_data = datas.len();

        self.vertices.extend(datas.enumerate().map(|(i, data)| {
            Vertex {
                prev: VertexRef(((i + n_data - 1) % n_data) + start_index),
                next: VertexRef(((i + 1) % n_data) + start_index),
                data,
            }
        }));

        VertexRef(start_index)
    }

    fn chain_refs(&self, start: VertexRef) -> Vec<VertexRef> {
        let mut chain = vec![];

        let mut current = start;

        while {
            chain.push(current);
            current = self.get(current).next;
            current != start
        }
        {}

        chain
    }

    fn splice_in(
        &mut self,
        start_ref: VertexRef,
        end_ref: VertexRef,
        sub_chain_start_ref: VertexRef,
        replace_start: bool,
        replace_end: bool,
    ) {
        let start_prev_ref = if replace_start {
            self.get(start_ref).prev
        } else {
            start_ref
        };

        let end_next_ref = if replace_end {
            self.get(end_ref).next
        } else {
            end_ref
        };

        self.get_mut(start_prev_ref).next = sub_chain_start_ref;
        self.get_mut(end_next_ref).prev = self.get(sub_chain_start_ref).prev;

        let last_sub_ref = self.get(sub_chain_start_ref).prev;
        self.get_mut(sub_chain_start_ref).prev = start_prev_ref;
        self.get_mut(last_sub_ref).next = end_next_ref;
    }
}

pub fn clip<P: Path>(
    mode: Mode,
    subject_shape: &SimpleShape<P>,
    clip_shape: &SimpleShape<P>,
) -> Vec<SimpleShape<P>> {
    let mut vertices = VertexArena::new();

    let mut start_subject_ref =
        vertices.add_chain(subject_shape.outline.segments().iter().map(|segment| {
            VertexData::bare_vertex(*segment)
        }));

    let mut start_clip_ref =
        vertices.add_chain(clip_shape.outline.segments().iter().map(|segment| {
            VertexData::bare_vertex(*segment)
        }));

    if DEBUG_PRINT {
        println!(
            r#"
            <svg width="320" height="320" viewbox="-0.5 -0.5 2.5 2.5" xmlns="http://www.w3.org/2000/svg">
                <g stroke="rgba(0, 0, 255, 0.3)" stroke-width="0.02" marker-end="url(#subj_marker)">
                    <marker id="subj_marker" viewBox="0 0 6 6" refX="6" refY="3" markerUnits="strokeWidth" orient="auto">
                        <path d="M 0 0 L 6 3 L 0 6 z" fill="rgba(0, 0, 255, 0.3)"/>
                    </marker>
                    {}
                </g>
                <g stroke="rgba(255, 0, 0, 0.3)" stroke-width="0.02" marker-end="url(#clip_marker)">
                    <marker id="clip_marker" viewBox="0 0 6 6" refX="6" refY="3" markerUnits="strokeWidth" orient="auto">
                        <path d="M 0 0 L 6 3 L 0 6 z" fill="rgba(255, 0, 0, 0.3)"/>
                    </marker>
                    {}
                </g>
            
        "#,
            vertices.chain_refs(start_subject_ref)
                .iter()
                .map(|subject_ref| format!(r#"<path d="{}"/> "#, vertices.get(*subject_ref).forward_segment.to_svg()))
                .collect::<Vec<_>>()
                .join(" "),
            vertices.chain_refs(start_clip_ref)
                .iter()
                .map(|clip_ref| format!(r#"<path d="{}"/> "#, vertices.get(*clip_ref).forward_segment.to_svg()))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    // Find intersections

    let mut subject_intersections = BinaryHeap::new();
    let mut clip_intersections = BinaryHeap::new();

    let mut subject_ref = start_subject_ref;

    'subject_segments: loop {
        let mut clip_ref = start_clip_ref;

        'clip_segments: loop {
            let intersections = (
                &vertices.get(subject_ref).forward_segment,
                &vertices.get(clip_ref).forward_segment,
            ).intersect();

            for intersection in &intersections {
                subject_intersections.push((subject_ref, OrderedFloat(intersection.along_a)));
                clip_intersections.push((clip_ref, OrderedFloat(intersection.along_b)));
            }

            clip_ref = vertices.get(clip_ref).next;
            if clip_ref == start_clip_ref {
                break 'clip_segments;
            }
        }

        subject_ref = vertices.get(subject_ref).next;
        if subject_ref == start_subject_ref {
            break 'subject_segments;
        }
    }

    // Insert intersections into chains at the appropriate point

    fn insert_intersections(
        vertices: &mut VertexArena,
        intersections: BinaryHeap<(VertexRef, OrderedFloat<f32>)>,
        original_start_ref: &mut VertexRef,
    ) -> Vec<VertexRef> {
        let mut intersection_refs = Vec::new();

        for (vertex_ref, intersection_distances) in
            &intersections.into_sorted_vec().into_iter().group_by(
                |&(vertex_ref, _)| vertex_ref,
            )
        {
            let segment = vertices.get(vertex_ref).forward_segment;
            let segmentation_distances = Some(0.0)
                .into_iter()
                .chain(intersection_distances.map(|(_, dist)| *dist))
                .chain(Some(segment.length()));

            // we only replace the start (if there is a close intersection),
            // because if it is close to the end, it is also close to
            // the start of the next segment

            use itertools::Position;

            // TODO: all of this is pretty ugly
            let mut first_actual_intersection_index = 0;

            let segmented_chain = vertices.add_chain(
                segmentation_distances
                    .tuple_windows()
                    .with_position()
                    .filter_map(|pair_with_position| match pair_with_position {
                        Position::First((a, b)) => {
                            segment.subsection(a, b).map(|subsegment| {
                                first_actual_intersection_index = 1;
                                VertexData::bare_vertex(subsegment)
                            })
                        }
                        Position::Middle((a, b)) |
                        Position::Last((a, b)) => {
                            segment.subsection(a, b).map(|subsegment| {
                                VertexData::bare_intersection(subsegment)
                            })
                        }
                        Position::Only(_) => unreachable!(),
                    })
                    .collect::<Vec<_>>()
                    .into_iter(),
            );

            intersection_refs.extend(
                vertices.chain_refs(segmented_chain)
                    [first_actual_intersection_index..]
                    .iter(),
            );

            let next = vertices.get(vertex_ref).next;
            vertices.splice_in(vertex_ref, next, segmented_chain, true, false);

            if vertex_ref == *original_start_ref {
                *original_start_ref = segmented_chain
            }
        }

        intersection_refs
    }

    let subject_intersection_refs =
        insert_intersections(&mut vertices, subject_intersections, &mut start_subject_ref);
    let clip_intersection_refs =
        insert_intersections(&mut vertices, clip_intersections, &mut start_clip_ref);

    let subject_chain_refs = vertices.chain_refs(start_subject_ref);
    let clip_chain_refs = vertices.chain_refs(start_clip_ref);

    if DEBUG_PRINT {
        println!(
            r#"
                <g stroke="rgba(0, 0, 255, 0.3)" stroke-width="0.01" marker-end="url(#subj_marker2)">
                    <marker id="subj_marker2" viewBox="0 0 6 6" refX="6" refY="3" markerUnits="strokeWidth" orient="auto">
                        <path d="M 0 0 L 6 3 L 0 6 z" fill="rgba(0, 0, 255, 1.0)"/>
                    </marker>
                    {}
                </g>
                <g stroke="rgba(255, 0, 0, 0.3)" stroke-width="0.01" marker-end="url(#clip_marker2)">
                    <marker id="clip_marker2" viewBox="0 0 6 6" refX="6" refY="3" markerUnits="strokeWidth" orient="auto">
                        <path d="M 0 0 L 6 3 L 0 6 z" fill="rgba(255, 0, 0, 1.0)"/>
                    </marker>
                    {}
                </g>
        "#,
            subject_chain_refs
                .iter()
                .map(|subject_ref| format!(r#"<path d="{}"/> "#, vertices.get(*subject_ref).forward_segment.to_svg()))
                .collect::<Vec<_>>()
                .join(" "),
            clip_chain_refs
                .iter()
                .map(|clip_ref| format!(r#"<path d="{}"/> "#, vertices.get(*clip_ref).forward_segment.to_svg()))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    // Assign neighbors

    for (subject_ref, clip_ref) in
        subject_chain_refs.iter().cartesian_product(
            clip_chain_refs.iter(),
        )
    {
        if vertices
            .get(*subject_ref)
            .forward_segment
            .start()
            .is_roughly(vertices.get(*clip_ref).forward_segment.start())
        {
            vertices.get_mut(*subject_ref).neighbor = Some(*clip_ref);
            vertices.get_mut(*clip_ref).neighbor = Some(*subject_ref);
        }
    }

    // Determine roles based on prev / next vertex
    // TODO: roles of the subject chain can more easily deduced by the roles of the clip chain

    for subject_ref in &subject_intersection_refs {
        let neighbor_ref = vertices.get(*subject_ref).neighbor.expect(
            "All intersections must have neighbors",
        );

        let (prev_subject_location, prev_clip_location, next_subject_location, next_clip_location) = {
            let vertex = vertices.get(*subject_ref);
            let prev_vertex = vertices.get(vertex.prev);
            let neighbor_vertex = vertices.get(neighbor_ref);
            let neighbor_prev_vertex = vertices.get(neighbor_vertex.prev);

            let prev_on_prev = (&prev_vertex.forward_segment).is_roughly_within(
                &neighbor_prev_vertex
                    .forward_segment,
                THICKNESS,
            );
            let prev_on_next = (&prev_vertex.forward_segment).is_roughly_within(
                &neighbor_vertex.forward_segment,
                THICKNESS,
            );

            let next_on_next = (&vertex.forward_segment).is_roughly_within(
                &neighbor_vertex.forward_segment,
                THICKNESS,
            );
            let next_on_prev = (&vertex.forward_segment).is_roughly_within(
                &neighbor_prev_vertex.forward_segment,
                THICKNESS,
            );

            (
                if prev_on_prev || prev_on_next {PointOnShapeLocation::OnEdge} else {
                    clip_shape.location_of(prev_vertex.forward_segment.midpoint())
                },
                if prev_on_prev || next_on_prev {PointOnShapeLocation::OnEdge} else {
                    subject_shape.location_of(neighbor_prev_vertex.forward_segment.midpoint())
                },
                if next_on_next || next_on_prev {PointOnShapeLocation::OnEdge} else {
                    clip_shape.location_of(vertex.forward_segment.midpoint())
                },
                if next_on_next || prev_on_next{PointOnShapeLocation::OnEdge} else {
                    subject_shape.location_of(neighbor_vertex.forward_segment.midpoint())
                }
            )
        };

        fn role_for(prev: PointOnShapeLocation, next: PointOnShapeLocation) -> Role {
            match (prev, next) {
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
        }

        vertices.get_mut(*subject_ref).role =
            role_for(prev_subject_location, next_subject_location);
        vertices.get_mut(neighbor_ref).role = role_for(prev_clip_location, next_clip_location);
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
            subject_intersection_refs
                .iter()
                .map(|subject_ref| {
                    let vertex = vertices.get(*subject_ref);
                    format!(
                        r#"<text x="{}" y={}>{:?}</text> "#,
                        vertex.forward_segment.start().x,
                        vertex.forward_segment.start().y,
                        vertex.role
                    )
                })
                .collect::<Vec<_>>()
                .join(" "),
            clip_intersection_refs
                .iter()
                .map(|clip_ref| {
                    let vertex = vertices.get(*clip_ref);
                    format!(
                        r#"<text x="{}" y={}>{:?}</text> "#,
                        vertex.forward_segment.start().x,
                        vertex.forward_segment.start().y + 0.1,
                        vertex.role
                    )
                })
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    // TODO: set couples

    // TODO: deal with cross-change situations



    // Find start vertex

    let mut result_shapes = Vec::new();

    while let Some(start_ref) = subject_intersection_refs
        .iter()
        //.chain(clip_intersection_refs.iter())
        .find(|potential_start_ref| {
            let potential_start = vertices.get(**potential_start_ref);

            potential_start.role != Role::None &&
                if let Some(couple_ref) = potential_start.partner {
                    if vertices.get(couple_ref).role == Role::None {
                        // Once a flag of a couple has been deleted, both of the
                        // vertices can no longer be used as a starting vertex.
                        false
                    } else {
                        // If the couple with each flag still set have (en, en),
                        // the second vertex can be selected as a starting vertex;
                        // if the couple have (ex, ex) the first vertex is selected.
                        (potential_start.role == Role::Entry &&
                             potential_start.prev == couple_ref) ||
                            (potential_start.role == Role::Exit &&
                                 potential_start.next == couple_ref)
                    }
                } else {
                    true
                }
        })
    {
        // Walk the chain & collect output vertices

        let mut current_ref = *start_ref;
        let mut direction = Direction::ForwardStay;
        let mut segments = Vec::new();

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
            let (new_direction, new_role) =
                traverse_step(vertices.get(current_ref).role, direction, mode);

            if DEBUG_PRINT {
                println!(
                    "<!-- {:?} {:?} -> {:?} {:?} -->",
                    vertices.get(current_ref).role,
                    direction,
                    new_direction,
                    new_role
                );
            }

            vertices.get_mut(current_ref).role = new_role;
            let old_vertex = vertices.get(current_ref);

            match new_direction {
                Direction::ForwardStay => {
                    segments.push(old_vertex.forward_segment);
                    current_ref = old_vertex.next;
                }
                Direction::BackwardStay => {
                    let prev = vertices.get(old_vertex.prev);
                    segments.push(prev.forward_segment.reverse());
                    current_ref = old_vertex.prev;
                }
                Direction::ForwardSwitch |
                Direction::BackwardSwitch => {
                    current_ref = old_vertex.neighbor.expect(
                        "Should only switch on intersections",
                    );
                }
            }

            direction = new_direction;

            if current_ref == *start_ref {
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

        result_shapes.push(SimpleShape::new(P::new(segments)));
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