/// Frequency in Hertz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hertz(pub u64);

impl std::fmt::Display for Hertz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} Hz", self.0)
    }
}

impl Hertz {
    pub const fn khz(khz: u64) -> Self {
        Self(khz * 1_000)
    }

    pub const fn mhz(mhz: u64) -> Self {
        Self(mhz * 1_000_000)
    }

    pub const fn ghz(ghz: u64) -> Self {
        Self(ghz * 1_000_000_000)
    }

    pub const fn as_hz(self) -> u64 {
        self.0
    }
}

impl From<u64> for Hertz {
    fn from(hz: u64) -> Self {
        Self(hz)
    }
}

impl From<Hertz> for u64 {
    fn from(hz: Hertz) -> Self {
        hz.0
    }
}
