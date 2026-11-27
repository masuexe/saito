//! Platform microphone permission registration for EXE5DS Crisis Shout (S-136).
//!
//! - **Android**: checks `Manifest.permission.RECORD_AUDIO` and triggers the runtime permission
//!   dialog through `SaitoActivity`.
//! - **macOS**: queries `AVCaptureDevice.authorizationStatusForMediaType(AVMediaTypeAudio)` and
//!   triggers `requestAccessForMediaType:completionHandler:` so the system privacy prompt appears
//!   with `NSMicrophoneUsageDescription`.
//! - **iOS**: queries/requests `AVAudioSession.recordPermission` and switches the session to
//!   `PlayAndRecord` before opening an input stream.
//! - **Windows / Linux / Wasm**: no pre-open permission prompt API; `MicMonitor` uses `cpal`
//!   directly (with [`saito_audio::MicMonitor::probe_access`] registering access when enabled).

#[cfg(target_os = "android")]
mod platform {
    use saito_audio::mic::{MicPermission, PermissionHooks};
    use std::sync::atomic::{AtomicBool, Ordering};

    static DENIED_THIS_ATTEMPT: AtomicBool = AtomicBool::new(false);

    pub fn init() {
        saito_audio::mic::set_permission_hooks(PermissionHooks {
            status,
            request,
            prepare_input: || Ok(()),
        });
    }

    pub fn reset_attempt() {
        DENIED_THIS_ATTEMPT.store(false, Ordering::Release);
    }

    pub fn on_android_result(granted: bool) {
        DENIED_THIS_ATTEMPT.store(!granted, Ordering::Release);
        saito_audio::mic::notify_permission_result(granted);
    }

    fn status() -> MicPermission {
        if crate::activity::has_microphone_permission() {
            DENIED_THIS_ATTEMPT.store(false, Ordering::Release);
            MicPermission::Granted
        } else if DENIED_THIS_ATTEMPT.load(Ordering::Acquire) {
            MicPermission::Denied
        } else {
            MicPermission::Undetermined
        }
    }

    fn request() {
        crate::activity::request_microphone_permission();
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use objc2::runtime::{AnyClass, Bool};
    use objc2_foundation::NSString;
    use saito_audio::mic::{MicPermission, PermissionHooks};

    #[link(name = "AVFoundation", kind = "framework")]
    unsafe extern "C" {}

    pub fn init() {
        saito_audio::mic::set_permission_hooks(PermissionHooks {
            status,
            request,
            prepare_input: || Ok(()),
        });
    }

    pub fn reset_attempt() {}

    fn status() -> MicPermission {
        let Some(cls) = AnyClass::get(c"AVCaptureDevice") else {
            return MicPermission::Granted;
        };
        // `AVMediaTypeAudio` is the constant FourCC string `"soun"`.
        let media_type = NSString::from_str("soun");
        // SAFETY: `authorizationStatusForMediaType:` is a macOS 10.14+ class method on AVCaptureDevice.
        let raw: isize = unsafe { objc2::msg_send![cls, authorizationStatusForMediaType: &*media_type] };
        match raw {
            0 => MicPermission::Undetermined,
            3 => MicPermission::Granted,
            _ => MicPermission::Denied,
        }
    }

    fn request() {
        let Some(cls) = AnyClass::get(c"AVCaptureDevice") else {
            saito_audio::mic::notify_permission_result(true);
            return;
        };
        let media_type = NSString::from_str("soun");
        let block = block2::RcBlock::new(|granted: Bool| {
            saito_audio::mic::notify_permission_result(granted.as_bool());
        });
        // SAFETY: `requestAccessForMediaType:completionHandler:` copies the block and invokes it on an arbitrary queue.
        unsafe {
            let _: () = objc2::msg_send![cls, requestAccessForMediaType: &*media_type, completionHandler: &*block];
        }
    }
}

#[cfg(target_os = "ios")]
mod platform {
    use objc2::runtime::Bool;
    use objc2_avf_audio::{
        AVAudioSession, AVAudioSessionCategoryOptions, AVAudioSessionCategoryPlayAndRecord,
        AVAudioSessionRecordPermission,
    };
    use saito_audio::mic::{MicPermission, PermissionHooks};

    pub fn init() {
        saito_audio::mic::set_permission_hooks(PermissionHooks {
            status,
            request,
            prepare_input,
        });
    }

    pub fn reset_attempt() {}

    fn status() -> MicPermission {
        #[allow(deprecated)]
        // SAFETY: `sharedInstance` and `recordPermission` are safe on iOS 15+.
        let perm = unsafe { AVAudioSession::sharedInstance().recordPermission() };
        #[allow(deprecated)]
        if perm == AVAudioSessionRecordPermission::Granted {
            MicPermission::Granted
        } else if perm == AVAudioSessionRecordPermission::Denied {
            MicPermission::Denied
        } else {
            MicPermission::Undetermined
        }
    }

    fn request() {
        let block = block2::RcBlock::new(|granted: Bool| {
            saito_audio::mic::notify_permission_result(granted.as_bool());
        });
        #[allow(deprecated)]
        // SAFETY: `requestRecordPermission:` copies the block and invokes it once the user answers.
        unsafe {
            AVAudioSession::sharedInstance().requestRecordPermission(&block);
        }
    }

    fn prepare_input() -> Result<(), String> {
        // SAFETY: configuring the shared AVAudioSession for PlayAndRecord before opening RemoteIO input.
        unsafe {
            let session = AVAudioSession::sharedInstance();
            let Some(category) = AVAudioSessionCategoryPlayAndRecord else {
                return Err("AVAudioSessionCategoryPlayAndRecord unavailable".to_string());
            };
            #[allow(deprecated)]
            let options = AVAudioSessionCategoryOptions::DefaultToSpeaker
                | AVAudioSessionCategoryOptions::AllowBluetooth
                | AVAudioSessionCategoryOptions::MixWithOthers;
            session
                .setCategory_withOptions_error(category, options)
                .map_err(|e| format!("failed to configure iOS audio session: {e}"))?;
            session
                .setActive_error(true)
                .map_err(|e| format!("failed to activate iOS audio session: {e}"))?;
        }
        Ok(())
    }
}

#[cfg(not(any(target_os = "android", target_os = "macos", target_os = "ios")))]
mod platform {
    pub fn init() {}
    pub fn reset_attempt() {}
}

pub(crate) fn init() {
    platform::init();
}

pub(crate) fn reset_attempt() {
    platform::reset_attempt();
}

#[cfg(target_os = "android")]
pub(crate) fn on_android_result(granted: bool) {
    platform::on_android_result(granted);
}
