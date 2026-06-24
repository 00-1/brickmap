//! Android **immersive-sticky fullscreen** — hide the status + navigation bars so the system
//! chrome stops overlaying the drill (owner-reported on-device). Same class of fix as the TWA
//! `T228` / the Capacitor cutout work on the web side.
//!
//! **UI-thread fix (owner: bars still showed):** the decor-view / `WindowInsetsController` calls
//! must run on the Android **UI thread**, but `android_main` (and so our resume handler) runs on a
//! *separate* thread — calling them there throws `CalledFromWrongThreadException`, which our guard
//! swallowed → a silent no-op. We now marshal the JNI work onto the UI thread via
//! [`AndroidApp::run_on_java_main_thread`], and additionally set the window flags through the
//! thread-safe [`AndroidApp::set_window_flags`] (FULLSCREEN + draw-under-cutout).
//!
//! cargo-apk packages a bare `NativeActivity` (no Java/Kotlin of ours), so on the UI thread we reach
//! the framework over **JNI**, in four independently-guarded steps:
//! 1. `Window.addFlags(FLAG_FULLSCREEN)` — drops the status bar (works broadly, incl. ≤ API 29).
//! 2. `decorView.setSystemUiVisibility(IMMERSIVE_STICKY | …)` — the legacy nav-bar hide.
//! 3. `Window.getInsetsController().hide(systemBars())` + transient-by-swipe — the API 30+ path.
//! 4. `LayoutParams.layoutInDisplayCutoutMode = SHORT_EDGES` — draw into the notch (API 28+).
//!
//! Every JNI call is wrapped so a missing method / thrown Java exception **can only no-op** (logged),
//! never panic or abort the process — built blind (no NDK locally; the owner device-judges), so a
//! partial result degrades gracefully rather than crashing.

use jni::objects::{JObject, JValue};
use jni::JavaVM;
use winit::platform::android::activity::{AndroidApp, WindowManagerFlags};

// android.view.View system-UI flags (the legacy immersive set).
const SYSTEM_UI_FLAG_FULLSCREEN: i32 = 0x0000_0004;
const SYSTEM_UI_FLAG_HIDE_NAVIGATION: i32 = 0x0000_0002;
const SYSTEM_UI_FLAG_LAYOUT_STABLE: i32 = 0x0000_0100;
const SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION: i32 = 0x0000_0200;
const SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN: i32 = 0x0000_0400;
const SYSTEM_UI_FLAG_IMMERSIVE_STICKY: i32 = 0x0000_1000;
const IMMERSIVE_STICKY_FLAGS: i32 = SYSTEM_UI_FLAG_FULLSCREEN
    | SYSTEM_UI_FLAG_HIDE_NAVIGATION
    | SYSTEM_UI_FLAG_LAYOUT_STABLE
    | SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
    | SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
    | SYSTEM_UI_FLAG_IMMERSIVE_STICKY;

// android.view.WindowManager.LayoutParams.FLAG_FULLSCREEN.
const FLAG_FULLSCREEN: i32 = 0x0000_0400;
// WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE (API 30+).
const BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE: i32 = 2;
// WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES (API 28+).
const CUTOUT_MODE_SHORT_EDGES: i32 = 1;

/// Apply immersive-sticky fullscreen to the current activity. Call on every resume/focus: the flags
/// are cleared by the system when the window loses focus, so re-applying keeps the bars hidden.
/// Best-effort and panic-free.
pub fn enable(app: &AndroidApp) {
    // Thread-safe window flags (the framework marshals these): drop the status bar + draw to the
    // screen edges so content fills behind the (now-hidden) bars.
    app.set_window_flags(
        WindowManagerFlags::FULLSCREEN | WindowManagerFlags::LAYOUT_NO_LIMITS,
        WindowManagerFlags::empty(),
    );
    // The decor-view / insets-controller calls MUST run on the Java UI thread — marshal them there
    // (the earlier silent no-op was calling them from the wrong thread).
    app.run_on_java_main_thread(Box::new(|| {
        if let Err(e) = apply() {
            log::warn!("immersive fullscreen not applied: {e}");
        }
    }));
}

/// The JNI body, returning an error rather than panicking so [`enable`] can swallow it.
fn apply() -> Result<(), jni::errors::Error> {
    let ctx = ndk_context::android_context();
    // SAFETY: ndk-context hands us the live VM + Activity pointers for this process.
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }?;
    let mut env = vm.attach_current_thread()?;
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    let window = env
        .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?
        .l()?;

    // 1) FLAG_FULLSCREEN — the broadly-supported status-bar drop.
    guarded(&mut env, "addFlags(FLAG_FULLSCREEN)", |env| {
        env.call_method(&window, "addFlags", "(I)V", &[JValue::Int(FLAG_FULLSCREEN)])?;
        Ok(())
    });

    // 2) Legacy decor-view immersive-sticky flags (effective on most pre-30 devices).
    guarded(&mut env, "setSystemUiVisibility", |env| {
        let decor = env
            .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?
            .l()?;
        env.call_method(
            &decor,
            "setSystemUiVisibility",
            "(I)V",
            &[JValue::Int(IMMERSIVE_STICKY_FLAGS)],
        )?;
        Ok(())
    });

    // 3) The API 30+ WindowInsetsController path (no-ops below 30, where the method is absent — the
    //    guard swallows the NoSuchMethod/exception).
    guarded(&mut env, "WindowInsetsController.hide", |env| {
        let controller = env
            .call_method(
                &window,
                "getInsetsController",
                "()Landroid/view/WindowInsetsController;",
                &[],
            )?
            .l()?;
        // WindowInsets.Type.systemBars()
        let bars = env
            .call_static_method("android/view/WindowInsets$Type", "systemBars", "()I", &[])?
            .i()?;
        env.call_method(
            &controller,
            "setSystemBarsBehavior",
            "(I)V",
            &[JValue::Int(BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE)],
        )?;
        env.call_method(&controller, "hide", "(I)V", &[JValue::Int(bars)])?;
        Ok(())
    });

    // 4) Draw into the display cutout / notch (API 28+): set the window LayoutParams' cutout mode.
    guarded(&mut env, "layoutInDisplayCutoutMode", |env| {
        let lp = env
            .call_method(
                &window,
                "getAttributes",
                "()Landroid/view/WindowManager$LayoutParams;",
                &[],
            )?
            .l()?;
        env.set_field(
            &lp,
            "layoutInDisplayCutoutMode",
            "I",
            JValue::Int(CUTOUT_MODE_SHORT_EDGES),
        )?;
        env.call_method(
            &window,
            "setAttributes",
            "(Landroid/view/WindowManager$LayoutParams;)V",
            &[JValue::Object(&lp)],
        )?;
        Ok(())
    });

    Ok(())
}

/// Run a JNI step, swallowing+clearing any thrown Java exception or error so one failed step (e.g. a
/// method absent on this API level, or a wrong-thread call) can't abort the process or block the
/// others.
fn guarded<F>(env: &mut jni::JNIEnv, what: &str, f: F)
where
    F: FnOnce(&mut jni::JNIEnv) -> Result<(), jni::errors::Error>,
{
    if let Err(e) = f(env) {
        log::debug!("immersive step `{what}` skipped: {e}");
    }
    // A pending Java exception would poison the next JNI call — clear it defensively.
    if matches!(env.exception_check(), Ok(true)) {
        let _ = env.exception_clear();
    }
}
