use super::{N, P2, V2, Norm, Curve, FiniteCurve, RoughlyComparable};
use super::primitives::Segment;
use super::intersect::Intersect;
use ordered_float::OrderedFloat;

type ScannerFn<'a> = fn(&mut StartOffsetState, &'a Segment) -> Option<(&'a Segment, N)>;
type ScanIter<'a> = ::std::iter::Scan<::std::slice::Iter<'a, Segment>, StartOffsetState, ScannerFn<'a>>;

pub trait Path : Sized + Clone {
    fn segments(&self) -> &[Segment];
    fn new(vec: Vec<Segment>) -> Self;

    fn scan_segments<'a>(start_offset: &mut StartOffsetState, segment: &'a Segment) -> Option<(&'a Segment, N)> {
        let pair = (segment, start_offset.0);
        start_offset.0 += segment.length;
        Some(pair)
    }
    
    fn segments_with_start_offsets(&self) -> ScanIter {
        self.segments().into_iter().scan(StartOffsetState(0.0), Self::scan_segments)
    }

    fn find_on_segment(&self, distance: N) -> Option<(&Segment, N)> {
        let mut distance_covered = 0.0;
        for segment in self.segments().iter() {
            let new_distance_covered = distance_covered + segment.length();
            if new_distance_covered > distance {
                return Some((segment, distance - distance_covered));
            }
            distance_covered = new_distance_covered;
        }
        None
    }

    // TODO: move this to shape
    fn contains(&self, point: P2) -> bool {
        let ray = Segment::line(point, P2::new(point.x + 10000000000.0, point.y));
        (self, &Self::new(vec![ray].into())).intersect().len() % 2 == 1
    }
}

pub struct StartOffsetState(N);

impl<T: Path> FiniteCurve for T {
    fn length(&self) -> N {
        self.segments().into_iter().map(|segment| segment.length()).fold(0.0, ::std::ops::Add::add)
    }

    fn along(&self, distance: N) -> P2 {
        match self.find_on_segment(distance) {
            Some((segment, distance_on_segment)) => segment.along(distance_on_segment),
            None => if distance < 0.0 {self.segments()[0].start} else {self.segments().last().unwrap().end}
        }
    }

    fn direction_along(&self, distance: N) -> V2 {
        match self.find_on_segment(distance) {
            Some((segment, distance_on_segment)) => segment.direction_along(distance_on_segment),
            None => self.segments()[0].direction_along(0.0)
        }
    }

    fn start(&self) -> P2 {
        self.segments()[0].start()
    }

    fn start_direction(&self) -> V2 {
        self.segments()[0].start_direction()
    }

    fn end(&self) -> P2 {
        self.segments().last().unwrap().end()
    }

    fn end_direction(&self) -> V2 {
        self.segments().last().unwrap().end_direction()
    }

    fn reverse(&self) -> Self {
        Self::new(self.segments().iter().rev().map(Segment::reverse).collect())
    }

    fn subsection(&self, start: N, end: N) -> Option<T> {
        let segments = self.segments_with_start_offsets().filter_map(|pair: (&Segment, N)| {
            let (segment, start_offset) = pair;
            let end_offset = start_offset + segment.length;
            if start_offset > end || end_offset < start {
                None
            } else {
                segment.subsection(start - start_offset, end - start_offset)
            }
        }).collect::<Vec<_>>();
        if segments.is_empty() {
            None
        } else {
            Some(T::new(segments))
        }
    }

    fn shift_orthogonally(&self, shift_to_right: N) -> Option<Self> {
        let segments = self.segments().iter().filter_map(
            |segment| segment.shift_orthogonally(shift_to_right)
        ).collect::<Vec<_>>();
        let mut glued_segments = Vec::new();
        let mut window_segments_iter = segments.iter().peekable();
        while let Some(segment) = window_segments_iter.next() {
            glued_segments.push(*segment);
            match window_segments_iter.peek() {
                Some(next_segment) => if !segment.end().is_roughly_within(next_segment.start(), 0.1) {
                    glued_segments.push(Segment::line(segment.end(), next_segment.start()));
                },
                None => break
            }
        }
        if glued_segments.is_empty() {
            None
        } else {
            let was_closed = self.end().is_roughly_within(self.start(), 0.1);
            let new_end = glued_segments.last().unwrap().end();
            let new_start = glued_segments[0].start();
            if was_closed && !new_end.is_roughly_within(new_start, 0.1) {
                glued_segments.push(Segment::line(new_end, new_start));
            }
            Some(Self::new(glued_segments))
        }
    }
}

impl<T: Path> Curve for T {
    // TODO: this can be really buggy/unexpected
    fn project(&self, point: P2) -> Option<N> {
        self.segments_with_start_offsets().filter_map(|pair: (&Segment, N)| {
            let (segment, start_offset) = pair;
            segment.project(point).map(|offset| offset + start_offset)
        }).min_by_key(|offset| OrderedFloat((self.along(*offset) - point).norm()))
    }

    fn includes(&self, point: P2) -> bool {
        self.segments().into_iter().any(|segment| segment.includes(point))
    }

    fn distance_to(&self, _point: P2) -> N {
        panic!("Don't trust this shit!");
        //self.segments().iter().map(|segment| OrderedFloat(segment.distance_to(point))).min().map(|ord_f| *ord_f).unwrap()
    }
}

impl<'a, T: Path> RoughlyComparable for &'a T {
    fn is_roughly_within(&self, other: &T, tolerance: N) -> bool {
        self.segments().len() == other.segments().len()
        && self.segments().iter().zip(other.segments().iter()).all(
            |(segment_1, segment_2)| segment_1.is_roughly_within(segment_2, tolerance)
        )
    }
}

use ncollide_transformation::convex_hull2_idx;

pub fn convex_hull<P: Path>(points: &[P2]) -> P {
    let mut hull_indices = convex_hull2_idx(points);
    let first_index = hull_indices[0];
    hull_indices.push(first_index);
    P::new(hull_indices.windows(2).filter_map(|idx_window| {
        let (point_1, point_2) = (points[idx_window[0]], points[idx_window[1]]);
        if point_1.is_roughly_within(point_2, ::primitives::MIN_START_TO_END) {
            None
        } else {
            Some(Segment::line(point_1, point_2))
        }
    }).collect())
}