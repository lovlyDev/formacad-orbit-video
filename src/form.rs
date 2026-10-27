//! Form values → recording parameters. Pure code, tested on the host target;
//! FormaCAD checks the same ranges again in `viewport.record-orbit`.
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Params {
    pub resolution: String,
    pub width: u32,
    pub height: u32,
    pub fps: String,
    pub duration_sec: f64,
    pub turns: f64,
    pub direction: String,
    pub up: String,
    pub tilt: f64,
    pub elevation: f64,
    pub start_angle: f64,
    pub zoom: f64,
    pub background: String,
    pub format: String,
    pub bitrate_kbps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Up {
    X,
    Y,
    Z,
    MinusX,
    MinusY,
    MinusZ,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Background {
    Theme,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Mp4,
    Webm,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Recording {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration_ms: u32,
    pub turns: f32,
    pub clockwise: bool,
    pub up: Up,
    pub tilt_deg: f32,
    pub elevation_deg: f32,
    pub start_deg: f32,
    pub zoom: f32,
    pub background: Background,
    pub format: Format,
    pub bitrate_kbps: u32,
}

/// What is wrong with the form. The message comes from `locales/*.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    Resolution,
    OddSize,
    Fps,
    Duration,
    Turns,
    Choice,
    TooManyFrames,
}

impl Problem {
    pub const ALL: [Problem; 7] = [
        Problem::Resolution,
        Problem::OddSize,
        Problem::Fps,
        Problem::Duration,
        Problem::Turns,
        Problem::Choice,
        Problem::TooManyFrames,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Resolution => "RESOLUTION_RANGE",
            Self::OddSize => "RESOLUTION_ODD",
            Self::Fps => "FPS_RANGE",
            Self::Duration => "DURATION_RANGE",
            Self::Turns => "TURNS_RANGE",
            Self::Choice => "CHOICE_UNKNOWN",
            Self::TooManyFrames => "TOO_MANY_FRAMES",
        }
    }

    /// Key in `locales/<lang>.json`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Resolution => "errorResolution",
            Self::OddSize => "errorOddSize",
            Self::Fps => "errorFps",
            Self::Duration => "errorDuration",
            Self::Turns => "errorTurns",
            Self::Choice => "errorChoice",
            Self::TooManyFrames => "errorFrames",
        }
    }
}

/// Same limit as FormaCAD: 36 000 frames (10 min at 60 fps).
pub const MAX_FRAMES: u64 = 36_000;

fn preset(name: &str) -> Option<(u32, u32)> {
    Some(match name {
        "720p" => (1280, 720),
        "1080p" => (1920, 1080),
        "1440p" => (2560, 1440),
        "2160p" => (3840, 2160),
        "square" => (1080, 1080),
        "vertical" => (1080, 1920),
        _ => return None,
    })
}

fn choice<T: Copy>(value: &str, options: &[(&str, T)]) -> Result<T, Problem> {
    options
        .iter()
        .find(|(name, _)| *name == value)
        .map(|(_, v)| *v)
        .ok_or(Problem::Choice)
}

pub fn recording(p: &Params) -> Result<Recording, Problem> {
    let (width, height) = match p.resolution.as_str() {
        "custom" => (p.width, p.height),
        name => preset(name).ok_or(Problem::Choice)?,
    };
    if !(160..=3840).contains(&width) || !(120..=2160).contains(&height) {
        return Err(Problem::Resolution);
    }
    if width % 2 != 0 || height % 2 != 0 {
        return Err(Problem::OddSize);
    }
    let fps: u32 = p.fps.parse().map_err(|_| Problem::Choice)?;
    if !(1..=120).contains(&fps) {
        return Err(Problem::Fps);
    }
    if !(0.5..=600.0).contains(&p.duration_sec) {
        return Err(Problem::Duration);
    }
    if !(0.05..=100.0).contains(&p.turns) {
        return Err(Problem::Turns);
    }
    let duration_ms = (p.duration_sec * 1000.0).round() as u32;
    if (u64::from(duration_ms) * u64::from(fps)).div_ceil(1000) > MAX_FRAMES {
        return Err(Problem::TooManyFrames);
    }
    Ok(Recording {
        width,
        height,
        fps,
        duration_ms,
        turns: p.turns as f32,
        clockwise: choice(&p.direction, &[("ccw", false), ("cw", true)])?,
        up: choice(
            &p.up,
            &[
                ("z", Up::Z),
                ("y", Up::Y),
                ("x", Up::X),
                ("-z", Up::MinusZ),
                ("-y", Up::MinusY),
                ("-x", Up::MinusX),
            ],
        )?,
        tilt_deg: p.tilt.clamp(-90.0, 90.0) as f32,
        elevation_deg: p.elevation.clamp(-89.0, 89.0) as f32,
        start_deg: p.start_angle.clamp(-360.0, 360.0) as f32,
        zoom: p.zoom.clamp(0.25, 4.0) as f32,
        background: choice(
            &p.background,
            &[
                ("theme", Background::Theme),
                ("light", Background::Light),
                ("dark", Background::Dark),
            ],
        )?,
        format: choice(&p.format, &[("mp4", Format::Mp4), ("webm", Format::Webm)])?,
        bitrate_kbps: match p.bitrate_kbps {
            0 => 0,
            value => value.clamp(100, 200_000),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> Params {
        serde_json::from_value(serde_json::json!({
            "resolution": "1080p", "width": 1920, "height": 1080, "fps": "30",
            "durationSec": 8.0, "turns": 1.0, "direction": "ccw", "up": "z",
            "tilt": 0.0, "elevation": 20.0, "startAngle": 0.0, "zoom": 1.0,
            "background": "theme", "format": "mp4", "bitrateKbps": 0
        }))
        .unwrap()
    }

    #[test]
    fn presets_and_defaults_become_a_recording() {
        let r = recording(&params()).unwrap();
        assert_eq!((r.width, r.height, r.fps, r.duration_ms), (1920, 1080, 30, 8000));
        assert_eq!((r.up, r.format, r.clockwise), (Up::Z, Format::Mp4, false));
        let mut p = params();
        p.resolution = "vertical".into();
        p.direction = "cw".into();
        p.up = "-y".into();
        let r = recording(&p).unwrap();
        assert_eq!((r.width, r.height, r.clockwise, r.up), (1080, 1920, true, Up::MinusY));
    }

    #[test]
    fn custom_size_and_limits_are_checked() {
        let mut p = params();
        p.resolution = "custom".into();
        p.width = 1001;
        assert_eq!(recording(&p), Err(Problem::OddSize));
        p.width = 5000;
        assert_eq!(recording(&p), Err(Problem::Resolution));
        let mut p = params();
        p.fps = "120".into();
        p.duration_sec = 600.0;
        assert_eq!(recording(&p), Err(Problem::TooManyFrames));
        let mut p = params();
        p.turns = 0.0;
        assert_eq!(recording(&p), Err(Problem::Turns));
        let mut p = params();
        p.format = "gif".into();
        assert_eq!(recording(&p), Err(Problem::Choice));
    }

    #[test]
    fn locales_match_and_cover_every_problem() {
        let ru: serde_json::Value = serde_json::from_str(include_str!("../locales/ru.json")).unwrap();
        let en: serde_json::Value = serde_json::from_str(include_str!("../locales/en.json")).unwrap();
        for problem in Problem::ALL {
            assert!(ru[problem.key()].is_string() && en[problem.key()].is_string(), "{}", problem.key());
        }
        let ru_keys: Vec<_> = ru.as_object().unwrap().keys().collect();
        let en_keys: Vec<_> = en.as_object().unwrap().keys().collect();
        assert_eq!(ru_keys, en_keys);
    }

    #[test]
    fn manifest_keys_exist_in_locales() {
        let manifest = include_str!("../manifest.json");
        let ru: serde_json::Value = serde_json::from_str(include_str!("../locales/ru.json")).unwrap();
        let mut rest = manifest;
        while let Some(start) = rest.find("\"%") {
            let tail = &rest[start + 2..];
            let end = tail.find("%\"").unwrap();
            let key = &tail[..end];
            assert!(ru[key].is_string(), "missing locale key {key}");
            rest = &tail[end..];
        }
    }
}
