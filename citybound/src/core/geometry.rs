use compass::{Path, Segment};
use kay::{CVec, Compact};

derive_compact! {
    pub struct CPath {
        segments: CVec<Segment>
    }
}

impl Path for CPath {
    fn segments(&self) -> &[Segment] {
        &self.segments
    }

    fn new(vec: Vec<Segment>) -> Self {
        CPath{
            segments: vec.into()
        }
    }
}