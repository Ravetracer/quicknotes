//! Renders PlantUML source to SVG using a local PlantUML installation.

use gtk::{gio, glib};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Finds a usable PlantUML command line. Order: setting, `$QUICKNOTES_PLANTUML`,
/// `plantuml` on `$PATH` (plus common user bin dirs), then a `plantuml.jar`.
pub fn command(configured: Option<&str>) -> Option<Vec<String>> {
    // An existing file is a program (paths may contain spaces), anything
    // else a command line like `java -jar /opt/plantuml.jar`.
    let split = |s: &str| {
        let s = s.trim();
        if Path::new(s).is_file() {
            vec![s.to_owned()]
        } else {
            s.split_whitespace().map(str::to_owned).collect::<Vec<_>>()
        }
    };
    if let Some(c) = configured.filter(|c| !c.trim().is_empty()) {
        return Some(split(c));
    }
    if let Ok(c) = std::env::var("QUICKNOTES_PLANTUML") {
        if !c.trim().is_empty() {
            return Some(split(&c));
        }
    }
    let home = dirs::home_dir().unwrap_or_default();
    // Prefer the jar: `plantuml` launchers are often thin wrappers that don't
    // forward all arguments (pipe mode is required here).
    let jar_dirs = [
        home.join("bin"),
        home.join(".local/bin"),
        home.join(".local/share/plantuml"),
        home.join(".local/lib"),
        "/usr/share/plantuml".into(),
        "/usr/share/java".into(),
        "/opt/plantuml".into(),
    ];
    if let Some(jar) = jar_dirs.iter().find_map(|d| find_jar(d)) {
        return Some(vec!["java".into(), "-jar".into(), jar.to_string_lossy().into_owned()]);
    }
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    dirs.extend([home.join("bin"), home.join(".local/bin"), "/usr/local/bin".into(), "/usr/bin".into()]);
    dirs.iter()
        .map(|d| d.join("plantuml"))
        .find(|p| is_executable(p))
        .map(|bin| vec![bin.to_string_lossy().into_owned()])
}

/// Newest `plantuml*.jar` in `dir` (by file name, which carries the version).
fn find_jar(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("plantuml") && n.ends_with(".jar"))
        })
        .max()
}

fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata().map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

/// Runs PlantUML and returns the SVG markup (PlantUML renders syntax errors as SVG too).
pub async fn render(cmd: Vec<String>, source: String) -> Result<String, String> {
    let mut argv: Vec<&OsStr> = cmd.iter().map(OsStr::new).collect();
    argv.extend(["-tsvg", "-pipe", "-charset", "UTF-8"].map(OsStr::new));
    let proc = gio::Subprocess::newv(
        &argv,
        gio::SubprocessFlags::STDIN_PIPE | gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    )
    .map_err(|e| format!("Cannot start PlantUML ({}): {e}", cmd.join(" ")))?;
    let input = glib::Bytes::from_owned(source.into_bytes());
    let (stdout, stderr) = proc
        .communicate_future(Some(&input))
        .await
        .map_err(|e| format!("PlantUML failed: {e}"))?;
    let out = stdout.map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    match out.find("<svg") {
        Some(i) => Ok(out[i..].to_owned()),
        None => {
            let err = stderr.map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
            Err(if err.trim().is_empty() { "PlantUML produced no output".into() } else { err })
        }
    }
}
