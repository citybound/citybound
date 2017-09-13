#[derive(Compact, Clone)]
pub struct AsyncCounter {
    pub count: usize,
    pub target: Option<usize>,
}

impl AsyncCounter {
    pub fn new() -> AsyncCounter {
        AsyncCounter { count: 0, target: None }
    }

    pub fn with_target(target: usize) -> AsyncCounter {
        AsyncCounter { count: 0, target: Some(target) }
    }

    pub fn increment(&mut self) {
        self.count += 1;
    }

    pub fn set_target(&mut self, target: usize) {
        self.target = Some(target)
    }

    pub fn is_done(&self) -> bool {
        self.target == Some(self.count)
    }
}
