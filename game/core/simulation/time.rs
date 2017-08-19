pub const TICKS_PER_SIM_SECOND: usize = 1;
pub const TICKS_PER_SIM_MINUTE: usize = 60 * TICKS_PER_SIM_SECOND;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DurationTicks(usize);

impl DurationTicks {
    pub fn new(ticks: usize) -> Self {
        DurationTicks(ticks)
    }
}

impl From<DurationSeconds> for DurationTicks {
    fn from(d_secs: DurationSeconds) -> DurationTicks {
        DurationTicks(d_secs.0 * TICKS_PER_SIM_SECOND)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DurationSeconds(usize);

impl DurationSeconds {
    pub fn new(seconds: usize) -> Self {
        DurationSeconds(seconds)
    }

    pub fn seconds(&self) -> usize {
        self.0
    }
}

impl ::std::ops::Add for DurationSeconds {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        DurationSeconds(self.0 + rhs.0)
    }
}

impl ::std::ops::AddAssign for DurationSeconds {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(usize);

impl Timestamp {
    pub fn new(ticks: usize) -> Self {
        Timestamp(ticks)
    }

    pub fn ticks(&self) -> usize {
        self.0
    }

    pub fn iticks(&self) -> isize {
        self.0 as isize
    }
}

impl<D: Into<DurationTicks>> ::std::ops::Add<D> for Timestamp {
    type Output = Self;

    fn add(self, rhs: D) -> Self {
        Timestamp(self.0 + rhs.into().0)
    }
}

impl<D: Into<DurationTicks>> ::std::ops::AddAssign<D> for Timestamp {
    fn add_assign(&mut self, rhs: D) {
        self.0 += rhs.into().0
    }
}

impl<D: Into<DurationTicks>> ::std::ops::Sub<D> for Timestamp {
    type Output = Self;

    fn sub(self, rhs: D) -> Self {
        Timestamp(self.0 - rhs.into().0)
    }
}

impl<D: Into<DurationTicks>> ::std::ops::SubAssign<D> for Timestamp {
    fn sub_assign(&mut self, rhs: D) {
        self.0 -= rhs.into().0
    }
}


#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct TimeOfDay {
    minutes_since_midnight: u16,
}

impl TimeOfDay {
    pub fn new(h: usize, m: usize) -> Self {
        TimeOfDay { minutes_since_midnight: m as u16 + (h * 60) as u16 }
    }

    pub fn from_tick(current_tick: Timestamp) -> Self {
        TimeOfDay {
            minutes_since_midnight: 7 * 60 + (current_tick.ticks() / TICKS_PER_SIM_MINUTE) as u16,
        }
    }

    pub fn hours_minutes(&self) -> (usize, usize) {
        (
            (self.minutes_since_midnight / 60) as usize,
            (self.minutes_since_midnight % 60) as usize,
        )
    }
}

impl<D: Into<DurationSeconds>> ::std::ops::Add<D> for TimeOfDay {
    type Output = Self;

    fn add(self, rhs: D) -> Self {
        TimeOfDay {
            minutes_since_midnight: self.minutes_since_midnight + (rhs.into().0 / 60) as u16,
        }
    }
}

impl<D: Into<DurationSeconds>> ::std::ops::AddAssign<D> for TimeOfDay {
    fn add_assign(&mut self, rhs: D) {
        self.minutes_since_midnight += (rhs.into().0 / 60) as u16
    }
}

impl<D: Into<DurationSeconds>> ::std::ops::Sub<D> for TimeOfDay {
    type Output = Self;

    fn sub(self, rhs: D) -> Self {
        TimeOfDay {
            minutes_since_midnight: self.minutes_since_midnight - (rhs.into().0 / 60) as u16,
        }
    }
}

impl<D: Into<DurationSeconds>> ::std::ops::SubAssign<D> for TimeOfDay {
    fn sub_assign(&mut self, rhs: D) {
        self.minutes_since_midnight -= (rhs.into().0 / 60) as u16
    }
}
