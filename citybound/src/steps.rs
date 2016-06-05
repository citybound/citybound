use State;

pub fn tick (past: &State, future: &mut State) {
    future.core.header.ticks = past.core.header.ticks + 1;
    future.core.header.time = past.core.header.time + 0.25;
}