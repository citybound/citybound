pub const TICKS_PER_SIM_SECOND: usize = 3;
pub const TICKS_PER_SIM_MINUTE: usize = 60 * TICKS_PER_SIM_SECOND;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ticks(pub usize);

impl From<Duration> for Ticks {
    fn from(d_secs: Duration) -> Ticks {
        Ticks(d_secs.0 * TICKS_PER_SIM_SECOND)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(pub usize);

impl Duration {
    pub fn from_seconds(seconds: usize) -> Self {
        Duration(seconds)
    }

    pub fn from_minutes(minutes: usize) -> Self {
        Self::from_seconds(60 * minutes)
    }

    pub fn from_hours(hours: usize) -> Self {
        Self::from_minutes(60 * hours)
    }

    pub fn as_seconds(&self) -> f32 {
        self.0 as f32
    }

    pub fn as_minutes(&self) -> f32 {
        self.0 as f32 / 60.0
    }

    pub fn as_hours(&self) -> f32 {
        self.as_minutes() / 60.0
    }

    pub fn as_days(&self) -> f32 {
        self.as_hours() / 24.0
    }
}

impl ::std::ops::Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Duration(self.0 + rhs.0)
    }
}

impl ::std::ops::AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Instant(usize);

impl Instant {
    pub fn new(ticks: usize) -> Self {
        Instant(ticks)
    }

    pub fn ticks(&self) -> usize {
        self.0
    }

    pub fn iticks(&self) -> isize {
        self.0 as isize
    }
}

impl<D: Into<Ticks>> ::std::ops::Add<D> for Instant {
    type Output = Self;

    fn add(self, rhs: D) -> Self {
        Instant(self.0 + rhs.into().0)
    }
}

impl<D: Into<Ticks>> ::std::ops::AddAssign<D> for Instant {
    fn add_assign(&mut self, rhs: D) {
        self.0 += rhs.into().0
    }
}

impl<D: Into<Ticks>> ::std::ops::Sub<D> for Instant {
    type Output = Self;

    fn sub(self, rhs: D) -> Self {
        Instant(self.0 - rhs.into().0)
    }
}

impl<D: Into<Ticks>> ::std::ops::SubAssign<D> for Instant {
    fn sub_assign(&mut self, rhs: D) {
        self.0 -= rhs.into().0
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct TimeOfDay {
    minutes_of_day: u16,
}

const BEGINNING_TIME_OF_DAY: usize = 7;
const MINUTES_PER_DAY: usize = 60 * 24;

impl TimeOfDay {
    pub fn new(h: usize, m: usize) -> Self {
        TimeOfDay {
            minutes_of_day: m as u16 + (h * 60) as u16,
        }
    }

    pub fn hours_minutes(&self) -> (usize, usize) {
        (
            (self.minutes_of_day / 60) as usize,
            (self.minutes_of_day % 60) as usize,
        )
    }

    pub fn earlier_by(&self, delta: Duration) -> Self {
        TimeOfDay {
            minutes_of_day: ((((self.minutes_of_day as isize - delta.as_minutes() as isize)
                % MINUTES_PER_DAY as isize)
                + MINUTES_PER_DAY as isize) as usize % MINUTES_PER_DAY)
                as u16,
        }
    }

    pub fn later_by(&self, delta: Duration) -> Self {
        TimeOfDay {
            minutes_of_day: ((self.minutes_of_day as usize + delta.as_minutes() as usize)
                % MINUTES_PER_DAY) as u16,
        }
    }
}

impl From<Instant> for TimeOfDay {
    fn from(instant: Instant) -> TimeOfDay {
        TimeOfDay {
            minutes_of_day: ((BEGINNING_TIME_OF_DAY * 60 + (instant.ticks() / TICKS_PER_SIM_MINUTE))
                % MINUTES_PER_DAY) as u16,
        }
    }
}

impl<D: Into<Duration>> ::std::ops::Add<D> for TimeOfDay {
    type Output = Self;

    fn add(self, rhs: D) -> Self {
        TimeOfDay {
            minutes_of_day: self.minutes_of_day + (rhs.into().0 / 60) as u16,
        }
    }
}

impl<D: Into<Duration>> ::std::ops::AddAssign<D> for TimeOfDay {
    #[cfg_attr(feature = "cargo-clippy", allow(suspicious_op_assign_impl))]
    fn add_assign(&mut self, rhs: D) {
        self.minutes_of_day += (rhs.into().0 / 60) as u16
    }
}

impl<D: Into<Duration>> ::std::ops::Sub<D> for TimeOfDay {
    type Output = Self;

    fn sub(self, rhs: D) -> Self {
        TimeOfDay {
            minutes_of_day: self.minutes_of_day - (rhs.into().0 / 60) as u16,
        }
    }
}

impl<D: Into<Duration>> ::std::ops::SubAssign<D> for TimeOfDay {
    #[cfg_attr(feature = "cargo-clippy", allow(suspicious_op_assign_impl))]
    fn sub_assign(&mut self, rhs: D) {
        self.minutes_of_day -= (rhs.into().0 / 60) as u16
    }
}

#[derive(Copy, Clone)]
pub struct TimeOfDayRange {
    pub start: TimeOfDay,
    pub end: TimeOfDay,
}

impl TimeOfDayRange {
    pub fn new(start_h: usize, start_m: usize, end_h: usize, end_m: usize) -> TimeOfDayRange {
        TimeOfDayRange {
            start: TimeOfDay::new(start_h, start_m),
            end: TimeOfDay::new(end_h, end_m),
        }
    }

    pub fn contains<T: Into<TimeOfDay>>(&self, time: T) -> bool {
        let time = time.into();
        if self.start <= self.end {
            self.start <= time && time <= self.end
        } else {
            self.start <= time || time <= self.end
        }
    }

    pub fn earlier_by(&self, delta: Duration) -> Self {
        TimeOfDayRange {
            start: self.start.earlier_by(delta),
            end: self.end.earlier_by(delta),
        }
    }

    pub fn later_by(&self, delta: Duration) -> Self {
        TimeOfDayRange {
            start: self.start.later_by(delta),
            end: self.end.later_by(delta),
        }
    }

    pub fn end_after_on_same_day(&self, time: TimeOfDay) -> bool {
        if self.end > self.start {
            time < self.end
        } else {
            time > self.start || time < self.end
        }
    }
}
