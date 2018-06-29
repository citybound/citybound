use {N, P2};
use line_path::LinePath;
use rough_eq::{RoughEq, THICKNESS};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "compact_containers", derive(Compact))]
pub struct ClosedLinePath(LinePath);

impl ClosedLinePath {
    pub fn new(path: LinePath) -> Option<Self> {
        if path.end().rough_eq_by(path.start(), THICKNESS) {
            Some(ClosedLinePath(path))
        } else {
            None
        }
    }

    pub fn try_clone_from(path: &LinePath) -> Option<Self> {
        if path.end().rough_eq_by(path.start(), THICKNESS) {
            Some(ClosedLinePath(path.clone()))
        } else {
            None
        }
    }

    pub fn path(&self) -> &LinePath {
        &self.0
    }

    pub fn subsection(&self, start: N, end: N) -> Option<LinePath> {
        if start > end + THICKNESS {
            let maybe_first_half = self.path().subsection(start, self.path().length());
            let maybe_second_half = self.path().subsection(0.0, end);

            match (maybe_first_half, maybe_second_half) {
                (Some(first_half), Some(second_half)) => {
                    first_half.concat_weld(&second_half, THICKNESS * 2.0).ok()
                }
                (Some(first_half), None) => Some(first_half),
                (None, Some(second_half)) => Some(second_half),
                _ => None,
            }
        } else {
            self.path().subsection(start, end)
        }
    }

    pub fn midpoint_between(&self, start: N, end: N) -> P2 {
        if start > end + THICKNESS {
            let length = self.path().length();
            let start_end_distance = (length - start) + end;

            if start + start_end_distance / 2.0 <= length {
                self.path().along(start + start_end_distance / 2.0)
            } else {
                self.path().along(end - start_end_distance / 2.0)
            }
        } else {
            self.path().along((start + end) / 2.0)
        }
    }
}

impl<'a> RoughEq for &'a ClosedLinePath {
    fn rough_eq_by(&self, other: Self, tolerance: N) -> bool {
        // TODO: is this really equality?
        self.path().points.len() == other.path().points.len()
            && self.path().segments().all(|self_segment| {
                other.path().segments().any(|other_segment| {
                    self_segment
                        .start()
                        .rough_eq_by(other_segment.start(), tolerance)
                        && self_segment
                            .end()
                            .rough_eq_by(other_segment.end(), tolerance)
                })
            })
    }
}
