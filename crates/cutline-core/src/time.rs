use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TimeError {
    #[error("Invalid time format '{0}'. Expected HH:MM:SS.mmm, MM:SS.mmm, seconds (e.g. '12.5s'), or 'frame:N'")]
    InvalidFormat(String),
    #[error("FPS denominator cannot be zero")]
    ZeroDenominator,
    #[error("Frame index cannot be negative: {0}")]
    NegativeFrame(i64),
}

/// Rational Frame Rate (Numerator / Denominator)
/// e.g., 30/1 for 30fps, 30000/1001 for 29.97fps, 24/1 for film.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct Fps {
    pub num: u32,
    pub den: u32,
}

impl Fps {
    pub const FPS_30: Self = Self { num: 30, den: 1 };
    pub const FPS_60: Self = Self { num: 60, den: 1 };
    pub const FPS_24: Self = Self { num: 24, den: 1 };
    pub const FPS_25: Self = Self { num: 25, den: 1 };
    pub const FPS_29_97: Self = Self {
        num: 30000,
        den: 1001,
    };
    pub const FPS_59_94: Self = Self {
        num: 60000,
        den: 1001,
    };

    pub fn new(num: u32, den: u32) -> Result<Self, TimeError> {
        if den == 0 {
            return Err(TimeError::ZeroDenominator);
        }
        Ok(Self { num, den })
    }

    #[inline]
    pub fn as_f64(&self) -> f64 {
        self.num as f64 / self.den as f64
    }
}

impl Default for Fps {
    fn default() -> Self {
        Self::FPS_30
    }
}

impl fmt::Display for Fps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{:.2}", self.as_f64())
        }
    }
}

/// Frame-accurate timestamp represented as an integer frame index at a rational FPS.
/// Guarantees determinism, zero floating-point accumulation drift, and exact cut points.
/// Covered by REQ-CORE-002.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct FrameTime {
    pub frame: i64,
    pub fps: Fps,
}

impl FrameTime {
    pub fn from_frame(frame: i64, fps: Fps) -> Result<Self, TimeError> {
        if frame < 0 {
            return Err(TimeError::NegativeFrame(frame));
        }
        Ok(Self { frame, fps })
    }

    pub fn from_seconds(secs: f64, fps: Fps) -> Result<Self, TimeError> {
        if secs < 0.0 {
            return Err(TimeError::InvalidFormat(format!(
                "Negative duration: {secs}"
            )));
        }
        let frame = (secs * fps.num as f64 / fps.den as f64).round() as i64;
        Ok(Self { frame, fps })
    }

    pub fn from_millis(ms: i64, fps: Fps) -> Result<Self, TimeError> {
        if ms < 0 {
            return Err(TimeError::InvalidFormat(format!(
                "Negative milliseconds: {ms}"
            )));
        }
        let frame = (ms as f64 * fps.num as f64 / (fps.den as f64 * 1000.0)).round() as i64;
        Ok(Self { frame, fps })
    }

    pub fn to_seconds(&self) -> f64 {
        (self.frame as f64 * self.fps.den as f64) / self.fps.num as f64
    }

    pub fn to_millis(&self) -> i64 {
        ((self.frame as f64 * self.fps.den as f64 * 1000.0) / self.fps.num as f64).round() as i64
    }

    /// Formats as HH:MM:SS.mmm
    pub fn to_timestamp_string(&self) -> String {
        let total_ms = self.to_millis();
        let ms = total_ms % 1000;
        let total_secs = total_ms / 1000;
        let secs = total_secs % 60;
        let total_mins = total_secs / 60;
        let mins = total_mins % 60;
        let hours = total_mins / 60;
        format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, ms)
    }

    /// Parses flexible human and machine inputs:
    /// - "frame:120" or "f120"
    /// - "12.4", "12.4s", "12s"
    /// - "00:01:23.450" (HH:MM:SS.mmm)
    /// - "01:23.450" (MM:SS.mmm)
    pub fn parse(input: &str, fps: Fps) -> Result<Self, TimeError> {
        let s = input.trim();
        if let Some(frame_str) = s.strip_prefix("frame:").or_else(|| s.strip_prefix("f")) {
            let frame = frame_str
                .parse::<i64>()
                .map_err(|_| TimeError::InvalidFormat(input.to_string()))?;
            return Self::from_frame(frame, fps);
        }

        if let Some(sec_str) = s.strip_suffix('s') {
            let secs = sec_str
                .parse::<f64>()
                .map_err(|_| TimeError::InvalidFormat(input.to_string()))?;
            return Self::from_seconds(secs, fps);
        }

        if let Ok(secs) = s.parse::<f64>() {
            return Self::from_seconds(secs, fps);
        }

        if s.contains(':') {
            let parts: Vec<&str> = s.split(':').collect();
            match parts.len() {
                2 => {
                    // MM:SS.mmm
                    let mins = parts[0]
                        .parse::<f64>()
                        .map_err(|_| TimeError::InvalidFormat(input.to_string()))?;
                    let secs = parts[1]
                        .parse::<f64>()
                        .map_err(|_| TimeError::InvalidFormat(input.to_string()))?;
                    return Self::from_seconds(mins * 60.0 + secs, fps);
                }
                3 => {
                    // HH:MM:SS.mmm
                    let hours = parts[0]
                        .parse::<f64>()
                        .map_err(|_| TimeError::InvalidFormat(input.to_string()))?;
                    let mins = parts[1]
                        .parse::<f64>()
                        .map_err(|_| TimeError::InvalidFormat(input.to_string()))?;
                    let secs = parts[2]
                        .parse::<f64>()
                        .map_err(|_| TimeError::InvalidFormat(input.to_string()))?;
                    return Self::from_seconds(hours * 3600.0 + mins * 60.0 + secs, fps);
                }
                _ => return Err(TimeError::InvalidFormat(input.to_string())),
            }
        }

        Err(TimeError::InvalidFormat(input.to_string()))
    }
}

impl Add for FrameTime {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        debug_assert_eq!(self.fps, rhs.fps, "Cannot add FrameTime with different FPS");
        Self {
            frame: self.frame + rhs.frame,
            fps: self.fps,
        }
    }
}

impl Sub for FrameTime {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        debug_assert_eq!(
            self.fps, rhs.fps,
            "Cannot subtract FrameTime with different FPS"
        );
        Self {
            frame: (self.frame - rhs.frame).max(0),
            fps: self.fps,
        }
    }
}

impl fmt::Display for FrameTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (frame:{})", self.to_timestamp_string(), self.frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rational_time_conversion_and_fps_snap() {
        // REQ-CORE-002: Exact frame calculation with no float drift
        let fps = Fps::FPS_30;
        let t1 = FrameTime::from_seconds(12.4, fps).unwrap();
        // 12.4 * 30 = 372
        assert_eq!(t1.frame, 372);
        assert_eq!(t1.to_seconds(), 12.4);

        let t2 = FrameTime::parse("00:00:12.400", fps).unwrap();
        assert_eq!(t2.frame, 372);

        let t3 = FrameTime::parse("frame:372", fps).unwrap();
        assert_eq!(t3.frame, 372);
    }

    #[test]
    fn test_time_parsing_formats() {
        let fps = Fps::FPS_30;
        assert_eq!(FrameTime::parse("10s", fps).unwrap().frame, 300);
        assert_eq!(FrameTime::parse("01:30.000", fps).unwrap().frame, 2700);
        assert_eq!(FrameTime::parse("01:00:00.000", fps).unwrap().frame, 108000);
        assert!(FrameTime::parse("invalid-time", fps).is_err());
    }

    #[test]
    fn test_timestamp_formatting() {
        let fps = Fps::FPS_30;
        let t = FrameTime::from_frame(372, fps).unwrap();
        assert_eq!(t.to_timestamp_string(), "00:00:12.400");
    }
}
