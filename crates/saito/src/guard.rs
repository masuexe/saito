//! Launch guard that refuses to start saito when `nettai` software is detected on the device.
//!
//! Platform coverage:
//! - Desktop (Linux, Windows, macOS): checks known `nettai` installation folders, app data folders,
//!   desktop entries/shortcuts/registry keys, `$PATH` executables, and active `nettai` processes while
//!   strictly excluding saito's own executable and PID.
//! - Android: queries `SaitoActivity.isNettaiInstalled()` via JNI (`net.n1gp.nettai`).
//! - iOS & Web: OS sandboxes forbid enumerating other installed applications; returns no findings
//!   without attempting any sandbox bypass.

use std::path::{Path, PathBuf};

/// Verify whether `nettai` is absent from this device.
/// Returns `true` to continue startup, or `false` if Android should stay on the native refusal dialog.
/// On desktop platforms, prints a localized refusal message, shows an error dialog if a display is
/// available, and exits with status 1.
#[cfg(target_os = "android")]
pub fn enforce() -> bool {
    let detected = crate::activity::java(|env, activity| {
        env.call_method(activity, jni::jni_str!("isNettaiInstalled"), jni::jni_sig!("()Z"), &[])?.z()
    });
    if matches!(detected, Ok(true)) {
        let (title, message) = refusal_message(crate::lang::system(), &["Android package: net.n1gp.nettai".to_string()]);
        eprintln!("{title}\n{message}");
        return false;
    }
    true
}

/// Verify whether `nettai` is absent from this device.
/// Returns `true` to continue startup; exits the process with code 1 if `nettai` is detected.
#[cfg(not(target_os = "android"))]
pub fn enforce() -> bool {
    let findings = detect();
    if findings.is_empty() {
        return true;
    }
    let (title, message) = refusal_message(crate::lang::system(), &findings);
    eprintln!("{title}\n{message}");
    saito_dialogs::alert(title, &message);
    std::process::exit(1);
}

#[cfg(any(target_os = "ios", target_arch = "wasm32"))]
fn detect() -> Vec<String> {
    // iOS and browser sandboxes forbid enumerating other installed apps; do not bypass sandboxes.
    Vec::new()
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
fn detect() -> Vec<String> {
    let own_exe = std::env::current_exe().ok().and_then(|p| std::fs::canonicalize(p).ok());
    let own_pid = std::process::id();
    let mut findings = detect_paths(&platform_candidates(), own_exe.as_deref());
    detect_platform_extras(&mut findings, own_exe.as_deref(), own_pid);
    findings
}

/// Check candidate file-system paths for existing `nettai` artifacts while excluding `own_exe`.
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos", test))]
fn detect_paths(candidates: &[PathBuf], own_exe: Option<&Path>) -> Vec<String> {
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for path in candidates {
        if path.as_os_str().is_empty() || !path.exists() {
            continue;
        }
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
        if let Some(own) = own_exe
            && canonical == own
        {
            continue;
        }
        if seen.contains(&canonical) {
            continue;
        }
        seen.push(canonical);
        out.push(path.display().to_string());
    }
    out
}

#[cfg(target_os = "linux")]
fn platform_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let xdg_data = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()).map(PathBuf::from);
    let home = std::env::var_os("HOME").filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(ref xdg) = xdg_data {
        paths.push(xdg.join("nettai"));
        paths.push(xdg.join("applications/nettai.desktop"));
        paths.push(xdg.join("applications/net.n1gp.nettai.desktop"));
    }
    if let Some(ref h) = home {
        paths.push(h.join(".local/share/nettai"));
        paths.push(h.join(".local/share/applications/nettai.desktop"));
        paths.push(h.join(".local/share/applications/net.n1gp.nettai.desktop"));
        paths.push(h.join(".local/bin/nettai"));
    }
    for sys_app in ["/usr/share/applications", "/usr/local/share/applications"] {
        let dir = Path::new(sys_app);
        paths.push(dir.join("nettai.desktop"));
        paths.push(dir.join("net.n1gp.nettai.desktop"));
    }
    paths.push(PathBuf::from("/opt/nettai"));
    paths.push(PathBuf::from("/usr/bin/nettai"));
    paths.push(PathBuf::from("/usr/local/bin/nettai"));
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            paths.push(dir.join("nettai"));
        }
    }
    paths
}

#[cfg(target_os = "linux")]
fn detect_platform_extras(findings: &mut Vec<String>, own_exe: Option<&Path>, own_pid: u32) {
    let Ok(entries) = std::fs::read_dir("/proc") else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid_str) = name.to_str() else { continue };
        let Ok(pid) = pid_str.parse::<u32>() else { continue };
        if pid == own_pid {
            continue;
        }
        let comm_path = entry.path().join("comm");
        let Ok(comm) = std::fs::read_to_string(comm_path) else { continue };
        if comm.trim() != "nettai" {
            continue;
        }
        if let Some(own) = own_exe
            && std::fs::canonicalize(entry.path().join("exe")).is_ok_and(|exe| exe == own)
        {
            continue;
        }
        findings.push(format!("process: nettai (pid {pid})"));
    }
}

#[cfg(target_os = "windows")]
fn platform_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty()).map(PathBuf::from) {
        paths.push(local.join("nettai"));
    }
    if let Some(roaming) = std::env::var_os("APPDATA").filter(|v| !v.is_empty()).map(PathBuf::from) {
        paths.push(roaming.join("nettai"));
        paths.push(roaming.join(r"Microsoft\Windows\Start Menu\Programs\nettai.lnk"));
    }
    if let Some(pf) = std::env::var_os("ProgramFiles").filter(|v| !v.is_empty()).map(PathBuf::from) {
        paths.push(pf.join("nettai"));
    }
    if let Some(pf86) = std::env::var_os("ProgramFiles(x86)").filter(|v| !v.is_empty()).map(PathBuf::from) {
        paths.push(pf86.join("nettai"));
    }
    if let Some(profile) = std::env::var_os("USERPROFILE").filter(|v| !v.is_empty()).map(PathBuf::from) {
        paths.push(profile.join(r"Desktop\nettai.lnk"));
    }
    paths
}

#[cfg(target_os = "windows")]
fn detect_platform_extras(findings: &mut Vec<String>, _own_exe: Option<&Path>, own_pid: u32) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    if let Ok(status) = std::process::Command::new("reg")
        .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\nettai"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        && status.status.success()
    {
        findings.push(r"registry: HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\nettai".to_string());
    }

    if let Ok(output) = std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq nettai.exe", "/FO", "CSV", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        && output.status.success()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let mut cols = line.split(',').map(|c| c.trim().trim_matches('"'));
            let Some(image) = cols.next() else { continue };
            let Some(pid_str) = cols.next() else { continue };
            if image.eq_ignore_ascii_case("nettai.exe")
                && let Ok(pid) = pid_str.parse::<u32>()
                && pid != own_pid
            {
                findings.push(format!("process: nettai.exe (pid {pid})"));
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn platform_candidates() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from("/Applications/nettai.app")];
    if let Some(home) = std::env::var_os("HOME").filter(|v| !v.is_empty()).map(PathBuf::from) {
        paths.push(home.join("Applications/nettai.app"));
        paths.push(home.join("Library/Application Support/nettai"));
    }
    paths
}

#[cfg(target_os = "macos")]
fn detect_platform_extras(findings: &mut Vec<String>, own_exe: Option<&Path>, own_pid: u32) {
    if let Ok(output) = std::process::Command::new("mdfind")
        .arg("kMDItemCFBundleIdentifier == 'net.n1gp.nettai'")
        .output()
        && output.status.success()
    {
        for line in String::from_utf8_lossy(&output.stdout).lines().map(str::trim).filter(|l| !l.is_empty()) {
            let bundle = std::fs::canonicalize(line).unwrap_or_else(|_| PathBuf::from(line));
            if let Some(own) = own_exe
                && own.starts_with(&bundle)
            {
                continue;
            }
            findings.push(format!("bundle: {line}"));
        }
    }

    if let Ok(output) = std::process::Command::new("pgrep").args(["-x", "nettai"]).output()
        && output.status.success()
    {
        for line in String::from_utf8_lossy(&output.stdout).lines().map(str::trim) {
            if let Ok(pid) = line.parse::<u32>()
                && pid != own_pid
            {
                findings.push(format!("process: nettai (pid {pid})"));
            }
        }
    }
}

/// Format a localized launch-refusal title and message listing what `nettai` artifacts were found.
fn refusal_message(lang: &str, findings: &[String]) -> (&'static str, String) {
    let list = findings.iter().map(|f| format!("• {f}")).collect::<Vec<_>>().join("\n");
    match lang {
        "ja" => (
            "saito を起動できません",
            format!("この端末で nettai ソフトウェアが検出されたため、saito の起動を拒否しました：\n{list}\n\nnettai を完全にアンインストール・削除してからもう一度お試しください。"),
        ),
        "ko" => (
            "saito를 실행할 수 없습니다",
            format!("이 기기에서 nettai 소프트웨어가 감지되어 saito 실행이 거부되었습니다:\n{list}\n\nnettai를 완전히 제거한 후 다시 시도하세요."),
        ),
        _ => (
            "saito 拒绝启动",
            format!("检测到本设备存在 nettai 软件，saito 已拒绝启动：\n{list}\n\n请先彻底卸载或移除 nettai 后再试。"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Clean directories produce no findings.
    #[test]
    fn clean_environment_produces_no_findings() {
        let dir = tempfile::tempdir().unwrap();
        let candidates = [dir.path().join("nettai"), dir.path().join("nettai.desktop")];
        assert!(detect_paths(&candidates, None).is_empty());
    }

    /// Existing `nettai` directories and files are reported and deduplicated.
    #[test]
    fn existing_nettai_artifacts_are_detected() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("nettai");
        let bin = dir.path().join("nettai.exe");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(&bin, b"stub").unwrap();
        let found = detect_paths(&[data.clone(), data.clone(), bin.clone()], None);
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|s| s.ends_with("nettai")));
        assert!(found.iter().any(|s| s.ends_with("nettai.exe")));
    }

    /// `saito`'s own executable path is excluded even if matched as a candidate.
    #[test]
    fn own_executable_is_excluded() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("saito");
        std::fs::write(&bin, b"self").unwrap();
        let canon = std::fs::canonicalize(&bin).unwrap();
        let other = dir.path().join("nettai");
        std::fs::create_dir_all(&other).unwrap();
        let found = detect_paths(&[bin, other], Some(&canon));
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("nettai"));
    }

    /// Refusal messages localize cleanly in zh-Hans, ja, and ko and list all detected items.
    #[test]
    fn refusal_messages_are_localized() {
        let items = vec!["/opt/nettai".to_string(), "process: nettai (pid 42)".to_string()];
        let (title_zh, body_zh) = refusal_message("zh-Hans", &items);
        let (title_ja, body_ja) = refusal_message("ja", &items);
        let (title_ko, body_ko) = refusal_message("ko", &items);
        let (title_fb, body_fb) = refusal_message("en", &items);
        assert_eq!(title_zh, "saito 拒绝启动");
        assert_eq!((title_fb, &body_fb), (title_zh, &body_zh));
        assert!(body_zh.contains("nettai") && body_zh.contains("/opt/nettai") && body_zh.contains("pid 42"));
        assert!(title_ja.contains("起動できません") && body_ja.contains("nettai") && body_ja.contains("/opt/nettai"));
        assert!(title_ko.contains("실행할 수 없습니다") && body_ko.contains("nettai") && body_ko.contains("/opt/nettai"));
    }
}
