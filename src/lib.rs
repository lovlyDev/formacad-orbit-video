//! В«Р’РёРґРµРѕ РІСЂР°С‰РµРЅРёСЏВ» вЂ” FormaCAD plugin that records a turntable video of the
//! model in the 3D view. The plugin checks the form and asks the host through
//! `viewport.record-orbit` (plugin-api 0.2, permission `viewport.capture`);
//! FormaCAD renders the frames, encodes the video and writes it to the file
//! the user picks. The plugin never sees the geometry, the pixels or the path.
pub mod form;

#[cfg(target_arch = "wasm32")]
mod plugin {
    wit_bindgen::generate!({
        path: "wit",
        world: "lovlydev:orbit-video/orbit-video",
        generate_all,
    });

    use super::form::{self, Background, Format, Up};
    use exports::formacad::plugin::command::Guest;
    use formacad::plugin::{
        assets, log,
        types::{Level, PluginError},
        viewport,
    };
    use serde_json::{json, Value};

    struct Plugin;

    /// Plugin strings from the package's own `locales/<lang>.json`.
    fn strings(context_json: &str) -> Value {
        let locale = serde_json::from_str::<Value>(context_json)
            .ok()
            .and_then(|c| c["locale"].as_str().map(str::to_owned));
        let file = if locale.as_deref() == Some("en") { "locales/en.json" } else { "locales/ru.json" };
        assets::read(file)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or(Value::Null)
    }

    fn fill(template: &str, values: &[(&str, String)]) -> String {
        values.iter().fold(template.to_owned(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
    }

    fn orbit(r: &form::Recording) -> viewport::Orbit {
        viewport::Orbit {
            width: r.width,
            height: r.height,
            fps: r.fps,
            duration_ms: r.duration_ms,
            turns: r.turns,
            clockwise: r.clockwise,
            up: match r.up {
                Up::X => viewport::UpAxis::X,
                Up::Y => viewport::UpAxis::Y,
                Up::Z => viewport::UpAxis::Z,
                Up::MinusX => viewport::UpAxis::MinusX,
                Up::MinusY => viewport::UpAxis::MinusY,
                Up::MinusZ => viewport::UpAxis::MinusZ,
            },
            tilt_deg: r.tilt_deg,
            elevation_deg: r.elevation_deg,
            start_deg: r.start_deg,
            zoom: r.zoom,
            background: match r.background {
                Background::Theme => viewport::Background::Theme,
                Background::Light => viewport::Background::Light,
                Background::Dark => viewport::Background::Dark,
            },
            format: match r.format {
                Format::Mp4 => viewport::VideoFormat::Mp4,
                Format::Webm => viewport::VideoFormat::Webm,
            },
            bitrate_kbps: r.bitrate_kbps,
        }
    }

    impl Guest for Plugin {
        fn run(command_id: String, params_json: String, context_json: String) -> Result<String, PluginError> {
            let text = strings(&context_json);
            let say = |key: &str| text[key].as_str().unwrap_or(key).to_owned();
            let error = |code: &str, message: String| PluginError { code: code.into(), message };
            if command_id != "record" {
                return Err(error("UNKNOWN_COMMAND", command_id));
            }
            let params: form::Params =
                serde_json::from_str(&params_json).map_err(|e| error("PARAMS_INVALID", e.to_string()))?;
            let recording = form::recording(&params).map_err(|p| error(p.code(), say(p.key())))?;
            viewport::record_orbit(orbit(&recording)).map_err(|detail| {
                log::write(Level::Warning, &detail);
                error("HOST_REFUSED", fill(&say("errorHost"), &[("detail", detail)]))
            })?;
            let message = fill(
                &say("queued"),
                &[
                    ("width", recording.width.to_string()),
                    ("height", recording.height.to_string()),
                    ("fps", recording.fps.to_string()),
                    ("seconds", (f64::from(recording.duration_ms) / 1000.0).to_string()),
                ],
            );
            Ok(json!({ "message": message, "commands": [] }).to_string())
        }
    }

    export!(Plugin);
}
