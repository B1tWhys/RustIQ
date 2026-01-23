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

/// Amplitude in Decibels (dB).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Decibels(pub f32);

impl std::fmt::Display for Decibels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1} dB", self.0)
    }
}

impl Decibels {
    /// Convert decibels to linear amplitude.
    /// For voltage/amplitude: linear = 10^(dB/20)
    pub fn to_linear(self) -> f32 {
        10.0_f32.powf(self.0 / 20.0)
    }

    /// Convert linear amplitude to decibels.
    /// For voltage/amplitude: dB = 20 * log10(linear)
    pub fn from_linear(linear: f32) -> Self {
        Self(20.0 * linear.log10())
    }

    pub const fn as_db(self) -> f32 {
        self.0
    }
}

impl From<f32> for Decibels {
    fn from(db: f32) -> Self {
        Self(db)
    }
}

impl From<Decibels> for f32 {
    fn from(db: Decibels) -> Self {
        db.0
    }
}
