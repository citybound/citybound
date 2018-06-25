use {N};
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
}
