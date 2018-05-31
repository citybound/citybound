use super::{N, P2, V2, RoughEq, THICKNESS, VecLike};
use super::curves::{Curve, FiniteCurve, Segment};
use super::intersect::{Intersect, Intersection, IntersectionResult};
use ordered_float::OrderedFloat;

type ScannerFn<'a> = fn(&mut StartOffsetState, &'a Segment) -> Option<(&'a Segment, N)>;
type ScanIter<'a> = ::std::iter::Scan<
    ::std::slice::Iter<'a, Segment>,
    StartOffsetState,
    ScannerFn<'a>,
>;

#[derive(Debug)]
pub enum PathError {
    EmptyPath,
    NotContinuous,
}

#[cfg_attr(feature = "compact_containers", derive(Compact))]
#[derive(Clone)]
pub struct Path {
    pub segments: VecLike<Segment>,
}

impl Path {
    pub fn new_unchecked(segments: VecLike<Segment>) -> Self {
        Path { segments }
    }

    pub fn new(segments: VecLike<Segment>) -> Result<Self, PathError> {
        if segments.is_empty() {
            Result::Err(PathError::EmptyPath)
        } else {
            let continuous = segments.windows(2).all(|seg_pair| {
                seg_pair[0].end().rough_eq_by(
                    seg_pair[1].start(),
                    THICKNESS,
                )
            });

            if !continuous {
                Result::Err(PathError::NotContinuous)
            } else {
                Result::Ok(Self::new_unchecked(segments))
            }
        }

    }

    pub fn new_welded(mut segments: VecLike<Segment>, tolerance: N) -> Result<Self, PathError> {
        if segments.is_empty() {
            Result::Err(PathError::EmptyPath)
        } else {
            let probably_closed = segments.last().unwrap().end().rough_eq_by(
                segments
                    .first()
                    .unwrap()
                    .start(),
                tolerance,
            );

            let original_length = segments.len();

            if probably_closed {
                let first_again = segments[0];
                segments.push(first_again);
            }

            let mut welded_segments: VecLike<Segment> = segments
                .windows(2)
                .filter_map(|seg_pair| if seg_pair[0].is_linear() {
                    Segment::line(seg_pair[0].start(), seg_pair[1].start())
                } else {
                    Segment::arc_with_direction(
                        seg_pair[0].start(),
                        seg_pair[0].start_direction(),
                        seg_pair[1].start(),
                    )
                })
                .collect();

            if !probably_closed {
                welded_segments.push(segments.last().cloned().unwrap())
            }

            if welded_segments.len() < original_length {
                // some welding resulted in an invalid segment, weld again
                Self::new_welded(welded_segments, tolerance)
            } else {
                Self::new(welded_segments)
            }
        }
    }

    pub fn scan_segments<'a>(
        start_offset: &mut StartOffsetState,
        segment: &'a Segment,
    ) -> Option<(&'a Segment, N)> {
        let pair = (segment, start_offset.0);
        start_offset.0 += segment.length;
        Some(pair)
    }

    pub fn segments_with_start_offsets(&self) -> ScanIter {
        self.segments.iter().scan(
            StartOffsetState(0.0),
            Self::scan_segments,
        )
    }

    pub fn find_on_segment(&self, distance: N) -> Option<(&Segment, N)> {
        let mut distance_covered = 0.0;
        for segment in self.segments.iter() {
            let new_distance_covered = distance_covered + segment.length();
            if new_distance_covered > distance {
                return Some((segment, distance - distance_covered));
            }
            distance_covered = new_distance_covered;
        }
        None
    }

    pub fn self_intersections(&self) -> Vec<Intersection> {
        self.segments_with_start_offsets()
            .enumerate()
            .flat_map(|(i, (segment_a, offset_a))| {
                self.segments_with_start_offsets()
                    .skip(i + 1)
                    .flat_map(|(segment_b, offset_b)| match (segment_a, segment_b)
                        .intersect() {
                        IntersectionResult::Intersecting(intersections) => {
                            intersections
                                .into_iter()
                                .filter_map(|intersection| if intersection.along_a.rough_eq_by(
                                    0.0,
                                    THICKNESS,
                                ) ||
                                    intersection.along_a.rough_eq_by(
                                        segment_a.length(),
                                        THICKNESS,
                                    ) ||
                                    intersection.along_b.rough_eq_by(0.0, THICKNESS) ||
                                    intersection.along_b.rough_eq_by(
                                        segment_b.length(),
                                        THICKNESS,
                                    )
                                {
                                    None
                                } else {
                                    Some(Intersection {
                                        position: intersection.position,
                                        along_a: offset_a + intersection.along_a,
                                        along_b: offset_b + intersection.along_b,
                                    })
                                })
                                .collect::<Vec<_>>()
                        }
                        _ => vec![],
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    pub fn is_closed(&self) -> bool {
        self.segments.last().unwrap().end().rough_eq_by(
            self.segments
                .first()
                .unwrap()
                .start(),
            THICKNESS,
        )
    }

    pub fn is_ordered_along(&self, start: N, mid: N, end: N) -> bool {
        if self.is_closed() {
            (start <= mid && mid < end) || (end < start && mid >= start) ||
                (end < start && mid < end)
        } else {
            start < mid && mid < end
        }
    }

    pub fn concat(&self, other: &Self) -> Result<Self, PathError> {
        // TODO: somehow change this to move self and other into here
        // but then segments would have to return [Segment], possible?
        if self.end().rough_eq_by(other.start(), THICKNESS) {
            Ok(Self::new_unchecked(
                self.segments
                    .iter()
                    .chain(other.segments.iter())
                    .cloned()
                    .collect(),
            ))
        } else {
            Err(PathError::NotContinuous)
        }
    }

    pub fn concat_weld(&self, other: &Self, tolerance: N) -> Result<Self, PathError> {
        Path::new_welded(
            self.segments
                .iter()
                .chain(other.segments.iter())
                .cloned()
                .collect(),
            tolerance,
        )
    }

    pub fn dash(&self, dash_length: N, gap_length: N) -> Vec<Path> {
        let mut on_dash = true;
        let mut position = 0.0;
        let mut dashes = Vec::new();

        while position < self.length() {
            let old_position = position;
            if on_dash {
                position += dash_length;
                if let Some(dash) = self.subsection(old_position, position) {
                    dashes.push(dash)
                }
            } else {
                position += gap_length;
            }

            on_dash = !on_dash;
        }

        dashes
    }

    pub fn to_svg(&self) -> String {
        self.segments
            .iter()
            .map(Segment::to_svg)
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn from_svg(string: &str) -> Result<Self, PathError> {
        let mut tokens = string.split_whitespace();
        let mut position = P2::new(0.0, 0.0);
        let mut first_position = None;
        let mut segments = VecLike::new();

        while let Some(command) = tokens.next() {
            if command == "M" || command == "L" {
                let x: f32 = tokens
                    .next()
                    .expect("Expected 1st token after M/L")
                    .parse()
                    .expect("Can't parse 1st token after M/L");
                let y: f32 = tokens
                    .next()
                    .expect("Expected 2nd token after M/L")
                    .parse()
                    .expect("Can't parse 2nd token after M/L");

                let next_position = P2::new(x, y);

                if command == "L" {
                    segments.push(Segment::line(position, next_position).expect(
                        "Invalid Segment",
                    ));
                }

                position = next_position;
                if first_position.is_none() {
                    first_position = Some(next_position);
                }
            } else if command == "Z" {
                if let Some(closing_segment) =
                    Segment::line(
                        position,
                        first_position.expect("Should have first_position"),
                    )
                {
                    segments.push(closing_segment);
                }
            }
        }

        Self::new(segments)
    }
}

pub struct StartOffsetState(N);

impl FiniteCurve for Path {
    fn length(&self) -> N {
        self.segments.iter().map(|segment| segment.length()).fold(
            0.0,
            ::std::ops::Add::add,
        )
    }

    fn along(&self, distance: N) -> P2 {
        match self.find_on_segment(distance) {
            Some((segment, distance_on_segment)) => segment.along(distance_on_segment),
            None => {
                if distance < 0.0 {
                    self.segments[0].start
                } else {
                    self.segments.last().unwrap().end
                }
            }
        }
    }

    fn direction_along(&self, distance: N) -> V2 {
        match self.find_on_segment(distance) {
            Some((segment, distance_on_segment)) => segment.direction_along(distance_on_segment),
            None => {
                if distance < 0.0 {
                    self.segments[0].start_direction()
                } else {
                    self.segments.last().unwrap().end_direction()
                }
            }
        }
    }

    fn start(&self) -> P2 {
        self.segments[0].start()
    }

    fn start_direction(&self) -> V2 {
        self.segments[0].start_direction()
    }

    fn end(&self) -> P2 {
        self.segments.last().unwrap().end()
    }

    fn end_direction(&self) -> V2 {
        self.segments.last().unwrap().end_direction()
    }

    fn reverse(&self) -> Self {
        Self::new_unchecked(self.segments.iter().rev().map(Segment::reverse).collect())
    }

    fn subsection(&self, start: N, end: N) -> Option<Path> {
        if start > end + THICKNESS && self.is_closed() {
            let maybe_first_half = self.subsection(start, self.length());
            let maybe_second_half = self.subsection(0.0, end);

            match (maybe_first_half, maybe_second_half) {
                (Some(first_half), Some(second_half)) => {
                    Some(first_half.concat(&second_half).expect(
                        "Closed path, should always be continous",
                    ))
                }
                (Some(first_half), None) => Some(first_half),
                (None, Some(second_half)) => Some(second_half),
                _ => None,
            }
        } else {
            let segments = self.segments_with_start_offsets()
                .filter_map(|pair: (&Segment, N)| {
                    let (segment, start_offset) = pair;
                    let end_offset = start_offset + segment.length;
                    if start_offset > end || end_offset < start {
                        None
                    } else {
                        segment.subsection(start - start_offset, end - start_offset)
                    }
                })
                .collect();
            Path::new(segments).ok()
        }
    }

    fn shift_orthogonally(&self, shift_to_right: N) -> Option<Path> {
        let segments = self.segments
            .iter()
            .filter_map(|segment| segment.shift_orthogonally(shift_to_right))
            .collect::<Vec<_>>();
        let mut glued_segments = VecLike::new();
        let mut window_segments_iter = segments.iter().peekable();
        while let Some(segment) = window_segments_iter.next() {
            glued_segments.push(*segment);
            match window_segments_iter.peek() {
                Some(next_segment) => {
                    if !segment.end().rough_eq_by(next_segment.start(), THICKNESS) {
                        glued_segments.push(Segment::line(segment.end(), next_segment.start())?);
                    }
                }
                None => break,
            }
        }
        if glued_segments.is_empty() {
            None
        } else {
            let was_closed = self.end().rough_eq_by(self.start(), THICKNESS);
            let new_end = glued_segments.last().unwrap().end();
            let new_start = glued_segments[0].start();
            if was_closed && !new_end.rough_eq_by(new_start, THICKNESS) {
                glued_segments.push(Segment::line(new_end, new_start)?);
            }
            Some(Self::new(glued_segments).unwrap())
        }
    }
}

impl Curve for Path {
    // TODO: this can be really buggy/unexpected
    fn project_with_tolerance(&self, point: P2, tolerance: N) -> Option<N> {
        self.segments_with_start_offsets()
            .filter_map(|pair: (&Segment, N)| {
                let (segment, start_offset) = pair;
                segment.project_with_tolerance(point, tolerance).map(
                    |offset| {
                        offset + start_offset
                    },
                )
            })
            .min_by_key(|offset| OrderedFloat((self.along(*offset) - point).norm()))
    }

    fn includes(&self, point: P2) -> bool {
        self.segments.iter().any(|segment| segment.includes(point))
    }

    fn distance_to(&self, point: P2) -> N {
        if let Some(offset) = self.project(point) {
            (point - self.along(offset)).norm()
        } else {
            *::std::cmp::min(
                OrderedFloat((point - self.start()).norm()),
                OrderedFloat((point - self.end()).norm()),
            )
        }
    }
}

impl<'a> RoughEq for &'a Path {
    fn rough_eq_by(&self, other: &Path, tolerance: N) -> bool {
        self.segments.len() == other.segments.len() &&
            if self.is_closed() && other.is_closed() {
                // TODO: this is strictly too loose
                // and maybe this should be moved to shape instead,
                // since the paths are *not* exactly equal
                self.segments.iter().all(|segment_1| {
                    other.segments.iter().any(|segment_2| {
                        segment_1.rough_eq_by(segment_2, tolerance)
                    })
                })
            } else {
                self.segments.iter().zip(other.segments.iter()).all(
                    |(segment_1, segment_2)| segment_1.rough_eq_by(segment_2, tolerance),
                )
            }

    }
}

impl ::std::fmt::Debug for Path {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        write!(f, "Path({:?})", &*self.segments)
    }
}
