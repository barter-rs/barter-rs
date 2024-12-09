use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use smol_str::{format_smolstr, SmolStr};

pub trait TimeInterval: Copy {
    fn name(&self) -> SmolStr;
    fn interval(&self) -> TimeDelta;
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct Annual365;

impl TimeInterval for Annual365 {
    fn name(&self) -> SmolStr {
        SmolStr::new("Annual(365)")
    }

    fn interval(&self) -> TimeDelta {
        TimeDelta::days(365)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct Annual252;

impl TimeInterval for Annual252 {
    fn name(&self) -> SmolStr {
        SmolStr::new("Annual(252)")
    }

    fn interval(&self) -> TimeDelta {
        TimeDelta::days(252)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct Daily;

impl TimeInterval for Daily {
    fn name(&self) -> SmolStr {
        SmolStr::new("Daily")
    }

    fn interval(&self) -> TimeDelta {
        TimeDelta::days(1)
    }
}

impl TimeInterval for TimeDelta {
    fn name(&self) -> SmolStr {
        format_smolstr!("Duration {} (minutes)", self.num_minutes())
    }

    fn interval(&self) -> TimeDelta {
        *self
    }
}
