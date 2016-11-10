use super::{N, P2, V2, Curve, FiniteCurve};
use super::primitives::Segment;

type ScannerFn<'a> = fn(&mut StartOffsetState, &'a Segment) -> Option<(&'a Segment, N)>;
type ScanIter<'a> = ::std::iter::Scan<::std::slice::Iter<'a, Segment>, StartOffsetState, ScannerFn<'a>>;

pub trait Path {
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
}

pub struct StartOffsetState(N);

impl<T: Path> FiniteCurve for T {
    fn length(&self) -> N {
        self.segments().into_iter().map(|segment| segment.length()).fold(0.0, ::std::ops::Add::add)
    }

    fn along(&self, distance: N) -> P2 {
        match self.find_on_segment(distance) {
            Some((segment, distance_on_segment)) => segment.along(distance_on_segment),
            None => self.segments()[0].start
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

    fn subsection(&self, start: N, end: N) -> T {
        T::new(self.segments_with_start_offsets().filter_map(|pair: (&Segment, N)| {
            let (segment, start_offset) = pair;
            let end_offset = start_offset + segment.length;
            if start_offset > end || end_offset < start {
                None
            } else {
                Some(segment.subsection(start - start_offset, end - start_offset))
            }
        }).collect())
    }

    fn shift_orthogonally(&self, shift_to_right: N) -> Self {
        Self::new(self.segments().iter().map(|segment| segment.shift_orthogonally(shift_to_right)).collect())
    }
}

impl<T: Path> Curve for T {
    fn project(&self, point: P2) -> Option<N> {
        self.segments_with_start_offsets().filter_map(|pair: (&Segment, N)| {
            let (segment, start_offset) = pair;
            let offset_on_segment = segment.project(point);
            match offset_on_segment {
                Some(offset) => Some(start_offset + offset),
                None => None
            }
        }).next()
    }

    fn includes(&self, point: P2) -> bool {
        self.segments().into_iter().any(|segment| segment.includes(point))
    }

    fn distance_to(&self, point: P2) -> N {
        self.segments().into_iter().fold(None, |min, segment| {
            let distance = segment.distance_to(point);
            if min.is_some() && distance < min.unwrap() {Some(distance)} else {min}
        }).unwrap()
    }
}

use ncollide_transformation::convex_hull2_idx;

pub fn convex_hull<P: Path>(points: &[P2]) -> P {
    let mut hull_indices = convex_hull2_idx(points);
    let first_index = hull_indices[0];
    hull_indices.push(first_index);
    P::new(hull_indices.windows(2).map(|idx_window| Segment::line(points[idx_window[0]], points[idx_window[1]])).collect())
}