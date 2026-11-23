//! Android's updates: the newest release's APK (the release's one `.apk`, every release's signed with the one release
//! key: packaging/android), asked of GitHub's releases ([`crate::Releases`]) against the installed app's version
//! (the APK's `versionName`, falling back to the compiled-in version if JNI fails), downloaded into the app's cache
//! folder (reusing an already-verified cached APK across unknown-apps permission grants), checked by the SHA-256
//! GitHub says, and handed to the system's installer by `android/saito/updater/SaitoUpdater.kt` (via
//! `PackageInstaller` session or `FileProvider` `ACTION_VIEW` fallback).

use crate::android_release::check_android_releases;
use crate::{Releases, Told};
use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::jint;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::Sender;

pub use crate::android_release::Found;

/// Ceiling on an APK download when its exact byte length wasn't known beforehand.
const MAX_APK: u64 = 512 << 20;

/// Verify that `path` exists and its streaming SHA-256 digest matches lowercase hex `expected`.
fn verify_sha256(path: &Path, expected: &str) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else { return false };
    let mut sum = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => sum.update(&buf[..n]),
            Err(_) => return false,
        }
    }
    format!("{:x}", sum.finalize()) == expected
}

/// `PackageInstaller.STATUS_FAILURE_ABORTED`: the player turned the update down.
const ABORTED: jint = 3;

/// `SaitoUpdater.HANDED_OFF`: control handed off to Android's external `ACTION_VIEW` package installer UI.
const HANDED_OFF: jint = -100;

/// Where the installer's answer goes, while the update is being installed.
static ANSWER: Mutex<Option<Sender<Told>>> = Mutex::new(None);

pub fn startup() {}

/// Ask GitHub for the newest release, on a thread.
pub fn check(releases: &Releases, fallback_version: Option<&str>, tell: Sender<Told>) -> bool {
    let releases = releases.clone();
    let fallback = fallback_version.map(str::to_owned);
    let started = on_thread("saito-updater: checking", async move {
        let told = match ask(&releases, fallback.as_deref()).await {
            Ok(Some(found)) => Told::Found(found),
            Ok(None) => Told::UpToDate,
            Err(why) => Told::CheckFailed(why),
        };
        let _ = tell.send(told);
    });
    if let Err(why) = &started {
        log::warn!("checking for an update: {why}");
    }
    started.is_ok()
}

/// Check `releases` against the installed app's version; cleans up stale cached APKs once up to date.
///
/// Queries the configured mirror (`<mirror>/latest/android.json`) first in a single request, falling back
/// to GitHub (`web_latest` -> tag API / `expanded_assets`, then `api.github.com/.../releases/latest`).
async fn ask(releases: &Releases, fallback_version: Option<&str>) -> Result<Option<Found>, String> {
    let installed = installed(fallback_version)?;
    let clients = crate::http::Clients::new()?;
    let found = check_android_releases(&clients, releases, &installed).await?;
    if found.is_none()
        && let Ok(path) = apk()
    {
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("apk.part"));
    }
    Ok(found)
}

/// Where the update's APK is downloaded: the app's cache folder.
fn apk() -> Result<PathBuf, String> {
    let dir = saito_files::folders::temporary().ok_or("the app has no cache folder")?;
    Ok(dir.join("saito-update.apk"))
}

/// Download `found`'s APK on a thread, telling how far (or reusing an already-verified cached APK).
pub fn download(_releases: &Releases, found: &Found, tell: Sender<Told>) {
    let found = found.clone();
    let failed = tell.clone();
    let started = on_thread("saito-updater: downloading", async move {
        let done = match apk() {
            Ok(path) => fetch(&found, &path, &tell).await,
            Err(why) => Err(why),
        };
        let _ = tell.send(Told::Downloaded(done));
    });
    if let Err(why) = started {
        let _ = failed.send(Told::Downloaded(Err(why)));
    }
}

/// `found`'s APK written atomically to `path`, its SHA-256 matching the release; returns immediately if cached and valid.
/// If downloading from `found.url` (e.g. mirror) fails, automatically retries `found.fallback_url` (GitHub) while
/// enforcing the same SHA-256 digest.
async fn fetch(found: &Found, path: &Path, tell: &Sender<Told>) -> Result<(), String> {
    if verify_sha256(path, &found.sha256) {
        let _ = tell.send(Told::Progress(1.0));
        return Ok(());
    }
    let clients = crate::http::Clients::new()?;
    match fetch_from_url(&clients, &found.url, found.size, &found.sha256, path, tell).await {
        Ok(()) => Ok(()),
        Err(primary_err) => {
            if let Some(fallback) = found.fallback_url.as_deref().filter(|u| *u != found.url) {
                log::warn!("APK download from primary URL failed ({primary_err}); falling back to GitHub");
                fetch_from_url(&clients, fallback, found.size, &found.sha256, path, tell)
                    .await
                    .map_err(|fb_err| format!("{fb_err} (primary: {primary_err})"))
            } else {
                Err(primary_err)
            }
        }
    }
}

async fn fetch_from_url(
    clients: &crate::http::Clients,
    url: &str,
    expected_size: u64,
    expected_sha256: &str,
    path: &Path,
    tell: &Sender<Told>,
) -> Result<(), String> {
    let part = path.with_extension("apk.part");
    let written = |e: std::io::Error| format!("{}: {e}", part.display());
    let limit = if expected_size > 0 { expected_size } else { MAX_APK };
    let res = async {
        let mut file = std::fs::File::create(&part).map_err(written)?;
        let mut sum = Sha256::new();
        let mut said = 0;
        clients
            .get_streaming(url, None, limit, |piece, got, content_len| {
                sum.update(piece);
                file.write_all(piece).map_err(written)?;
                let total = if expected_size > 0 { expected_size } else { content_len.unwrap_or(0) };
                if total > 0 {
                    // (Told by the percent: no more often.)
                    let percent = (got.saturating_mul(100) / total).min(100);
                    if percent != said {
                        said = percent;
                        let _ = tell.send(Told::Progress(percent as f32 / 100.0));
                    }
                }
                Ok(())
            })
            .await?;
        file.flush().map_err(written)?;
        drop(file);
        let sum = format!("{:x}", sum.finalize());
        if sum != expected_sha256 {
            return Err(format!("the APK downloaded has the SHA-256 {sum}, where the release says {expected_sha256}"));
        }
        std::fs::rename(&part, path).map_err(|e| format!("{} -> {}: {e}", part.display(), path.display()))?;
        Ok(())
    }
    .await;
    if res.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    res
}

/// Hand the APK to the system's installer, on a thread: keeps the verified APK cached in case Android kills or prompts for
/// permission so the next launch can resume without re-downloading.
pub fn install(_releases: &Releases, found: Found, tell: Sender<Told>) -> Result<(), String> {
    let path = apk()?;
    log::info!("installing saito {}", found.version);
    *ANSWER.lock().unwrap_or_else(|e| e.into_inner()) = Some(tell.clone());
    let spawned = std::thread::Builder::new().name("saito-updater: installing".into()).spawn(move || {
        if let Err(why) = commit(&path) {
            let _ = tell.send(Told::NotInstalled { why, declined: false });
        }
    });
    spawned.map(drop).map_err(|e| format!("a thread for the update: {e}"))
}

/// `work` run to its end on a thread of its own, with a tokio runtime of its own (reqwest's).
fn on_thread(name: &str, work: impl Future<Output = ()> + Send + 'static) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
    let runtime = runtime.map_err(|e| format!("a runtime for the update: {e}"))?;
    let spawned = std::thread::Builder::new().name(name.into()).spawn(move || runtime.block_on(work));
    spawned.map(drop).map_err(|e| format!("a thread for the update: {e}"))
}

/// `SaitoUpdater.httpProxy`: the system's configured HTTP(S) proxy URL, if any.
pub(crate) fn http_proxy() -> Option<String> {
    let raw = java(|env, activity| {
        let class = load_class(env, activity)?;
        let proxy = env
            .call_static_method(
                &class,
                jni_str!("httpProxy"),
                jni_sig!("(Landroid/app/Activity;)Ljava/lang/String;"),
                &[JValue::Object(activity)],
            )?
            .l()?;
        if proxy.is_null() {
            return Ok(None);
        }
        let proxy = env.cast_local::<JString>(proxy)?;
        Ok(Some(proxy.try_to_string(env)?))
    })
    .ok()
    .flatten()?;
    crate::http::normalize_http_proxy(&raw)
}

/// `SaitoUpdater.version`: the installed app's version, as Android says it (falling back to compiled-in version).
fn installed(fallback: Option<&str>) -> Result<String, String> {
    let res = java(|env, activity| {
        let class = load_class(env, activity)?;
        let version = env
            .call_static_method(&class, jni_str!("version"), jni_sig!("(Landroid/app/Activity;)Ljava/lang/String;"), &[JValue::Object(activity)])?
            .l()?;
        let version = env.cast_local::<JString>(version)?;
        version.try_to_string(env)
    });
    match res {
        Ok(version) if !version.trim().is_empty() => Ok(version),
        other => {
            if let Some(fb) = fallback.filter(|s| !s.trim().is_empty()) {
                if let Err(why) = &other {
                    log::warn!("reading Android versionName via JNI failed ({why}); using built-in version {fb}");
                }
                Ok(fb.to_owned())
            } else {
                other.and_then(|_| Err("Android versionName is empty".into()))
            }
        }
    }
}

/// `SaitoUpdater.install`: the session made, the APK written into it, and committed; or why not.
fn commit(apk: &Path) -> Result<(), String> {
    let refused = java(|env, activity| {
        let class = helper(env, activity)?;
        let path = env.new_string(apk.to_string_lossy())?;
        let refused = env
            .call_static_method(
                &class,
                jni_str!("install"),
                jni_sig!("(Landroid/app/Activity;Ljava/lang/String;)Ljava/lang/String;"),
                &[JValue::Object(activity), JValue::Object(&path)],
            )?
            .l()?;
        if refused.is_null() {
            return Ok(None);
        }
        let refused = env.cast_local::<JString>(refused)?;
        Ok(Some(refused.try_to_string(env)?))
    })?;
    refused.map_or(Ok(()), Err)
}

/// `call` run with Java's environment (this thread attached to the VM) and the activity: ndk-context's, which the
/// app sets as it starts (its `android_main`). What Java threw, or what went wrong, is the error, said.
fn java<T>(call: impl FnOnce(&mut Env, &JObject) -> jni::errors::Result<T>) -> Result<T, String> {
    let context = ndk_context::android_context();
    // SAFETY: the process's VM, which android-activity hands ndk-context as the app starts.
    let vm = unsafe { jni::JavaVM::from_raw(context.vm().cast()) };
    vm.attach_current_thread(|env| {
        // SAFETY: the activity, which GameActivity holds as a global reference for the app's life.
        let activity = unsafe { JObject::from_raw(env, context.context().cast()) };
        call(env, &activity)
    })
    .map_err(|e: jni::errors::Error| e.to_string())
}

/// Load `saito.updater.SaitoUpdater` through the activity's class loader without touching native method registration.
fn load_class<'local>(env: &mut Env<'local>, activity: &JObject) -> jni::errors::Result<JClass<'local>> {
    let loader = env.call_method(activity, jni_str!("getClassLoader"), jni_sig!("()Ljava/lang/ClassLoader;"), &[])?.l()?;
    let name = env.new_string("saito.updater.SaitoUpdater")?;
    let class = env.call_method(&loader, jni_str!("loadClass"), jni_sig!("(Ljava/lang/String;)Ljava/lang/Class;"), &[JValue::Object(&name)])?.l()?;
    env.cast_local::<JClass>(class)
}

/// `SaitoUpdater` with its `ended` native callback registered (only needed for `install`).
fn helper<'local>(env: &mut Env<'local>, activity: &JObject) -> jni::errors::Result<JClass<'local>> {
    let class = load_class(env, activity)?;
    // SAFETY: the function's parameters are what its signature says: (JNIEnv*, jclass, jint, jstring).
    unsafe {
        env.register_native_methods(
            &class,
            &[NativeMethod::from_raw_parts(jni_str!("ended"), jni_str!("(ILjava/lang/String;)V"), ended as *mut std::ffi::c_void)],
        )?;
    }
    Ok(class)
}

/// `SaitoUpdater.ended`: the installer's answer, where the update wasn't installed (`PackageInstaller`'s status and
/// its message).
extern "system" fn ended<'local>(mut env: EnvUnowned<'local>, _class: JClass<'local>, status: jint, message: JString<'local>) {
    let message = env.with_env(|env| message.try_to_string(env)).resolve::<jni::errors::LogErrorAndDefault>();
    if let Some(tell) = ANSWER.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let why = format!("{message} (the installer's status {status})");
        let _ = tell.send(Told::NotInstalled { why, declined: status == ABORTED || status == HANDED_OFF });
    }
}
