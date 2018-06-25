use {N, P2, V2};

// Thickness radius
pub const THICKNESS: N = 0.001;
const ROUGH_TOLERANCE: N = 0.000_000_1;

pub trait RoughEq: Sized {
    fn rough_eq(&self, other: Self) -> bool {
        self.rough_eq_by(other, ROUGH_TOLERANCE)
    }
    fn rough_eq_by(&self, other: Self, tolerance: N) -> bool;
}

impl RoughEq for N {
    fn rough_eq_by(&self, other: N, tolerance: N) -> bool {
        (self - other).abs() <= tolerance
    }
}

impl RoughEq for P2 {
    fn rough_eq_by(&self, other: P2, tolerance: N) -> bool {
        (*self - other).norm() <= tolerance
    }
}

impl RoughEq for V2 {
    fn rough_eq_by(&self, other: V2, tolerance: N) -> bool {
        (*self - other).norm() <= tolerance
    }
}
