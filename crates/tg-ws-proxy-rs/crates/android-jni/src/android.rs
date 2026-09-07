//! JNI surface for the Android app.
//!
//! Intentionally tiny: parse the same CLI string the binary accepts, run
//! [`tg_ws_proxy_rs::server`] on a background Tokio runtime, and push log lines plus
//! the `tg://` link back into Kotlin.  Start/stop is cooperative — completing
//! the watch channel breaks the accept loop the same way a Ctrl+C would.

use std::cell::Cell;
use std::io::{self, Write};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use jni::objects::{Global, JClass, JObject, JString, JValue, JValueOwned};
use jni::signature::MethodSignature;
use jni::strings::JNIString;
use jni::sys::{JNI_FALSE, JNI_TRUE, JNI_VERSION_1_6, jboolean, jstring};
use jni::{EnvUnowned, Outcome, jni_sig};
use tokio::sync::watch;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{EnvFilter, Registry, fmt::MakeWriter, reload};

use tg_ws_proxy_rs::config::Config;
use tg_ws_proxy_rs::server;

static JVM: OnceLock<jni::JavaVM> = OnceLock::new();
/// Cached in [`JNI_OnLoad`] / the first `native*` call, while the thread still
/// has the app class loader.  `FindClass` after `AttachCurrentThread` on a
/// Tokio worker sees only the system loader, which cannot see `NativeProxy`
/// and leaves a pending `ClassNotFoundException` that kills the process.
static NATIVE_CLASS: OnceLock<Global<JClass<'static>>> = OnceLock::new();

/// Reload handle for the `EnvFilter` installed by [`install_logging`].
///
/// A global subscriber can only be set once per process, so every Start after
/// the first has to swap the filter through this handle instead: with
/// `--quiet` in the app's default arguments the second `try_init` used to fail
/// silently, which left the Log panel permanently empty and made dropping
/// `--quiet` from the arguments field do nothing until the process was killed.
///
/// The `S` parameter is the subscriber the layer is registered on; the filter
/// is the first `.with()` on a bare `Registry`, so it is simply `Registry`.
static LOG_FILTER: OnceLock<reload::Handle<EnvFilter, Registry>> = OnceLock::new();

/// Signatures of the static Kotlin callbacks (`onNativeLog`, `onNativeError`,
/// `onNativeListening`, `onNativeStopped`), parsed at compile time.
static LOG_SIG: MethodSignature = jni_sig!((arg: JString) -> void);
static VOID_SIG: MethodSignature = jni_sig!(() -> void);

struct ProxyState {
    shutdown: Option<watch::Sender<bool>>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// Set by [`stop_proxy`], cleared when the worker is reaped.  The two
    /// readers need different answers while the runtime winds down: Kotlin
    /// must see `nativeIsRunning() == false` the moment Stop is pressed, or
    /// `ACTION_START` would repaint "Running" over a proxy with no listener,
    /// while [`start_proxy`] must keep refusing until the old runtime is gone.
    /// Only [`start_proxy`] ever clears it, so between a Stop and the next
    /// Start `nativeIsRunning()` answers false whatever the worker is doing.
    stopping: bool,
}

static STATE: Mutex<ProxyState> = Mutex::new(ProxyState {
    shutdown: None,
    thread: None,
    stopping: false,
});

const NATIVE_CLASS_NAME: &str = "io/github/valnesfjord/tgwsproxyrs/NativeProxy";

#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(
    vm: *mut jni::sys::JavaVM,
    _reserved: *mut std::ffi::c_void,
) -> jni::sys::jint {
    // SAFETY: JNI_OnLoad is given a valid JavaVM pointer by the runtime.
    let vm = unsafe { jni::JavaVM::from_raw(vm) };
    let _ = vm.with_top_local_frame(|env| {
        cache_native_class_by_name(env);
        jni::errors::Result::Ok(())
    });
    let _ = JVM.set(vm);
    JNI_VERSION_1_6
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_valnesfjord_tgwsproxyrs_NativeProxy_nativeStart<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    class: JClass<'caller>,
    args: JString<'caller>,
) -> jstring {
    unowned_env
        .with_env(|env| -> jni::errors::Result<jstring> {
            cache_native_class(env, &class);
            let args: String = match args.try_to_string(env) {
                Ok(s) => s,
                Err(e) => {
                    // A failed read leaves a pending exception; NewStringUTF
                    // below is not callable while one is pending, so clear it.
                    env.exception_clear();
                    return throw_string(env, &format!("invalid arguments: {e}"));
                }
            };

            match start_proxy(&args) {
                Ok(()) => Ok(std::ptr::null_mut()),
                Err(e) => throw_string(env, &e),
            }
        })
        .into_outcome()
        .into_value()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_valnesfjord_tgwsproxyrs_NativeProxy_nativeStop<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    class: JClass<'caller>,
) {
    unowned_env
        .with_env(|env| -> jni::errors::Result<()> {
            cache_native_class(env, &class);
            stop_proxy();
            Ok(())
        })
        .into_outcome()
        .into_value();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_valnesfjord_tgwsproxyrs_NativeProxy_nativeIsRunning<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    class: JClass<'caller>,
) -> jboolean {
    unowned_env
        .with_env(|env| -> jni::errors::Result<jboolean> {
            cache_native_class(env, &class);
            Ok(match STATE.lock() {
                Ok(state) => {
                    if !state.stopping && state.thread.as_ref().is_some_and(|t| !t.is_finished()) {
                        JNI_TRUE
                    } else {
                        JNI_FALSE
                    }
                }
                Err(_) => JNI_FALSE,
            })
        })
        .into_outcome()
        .into_value()
}

/// Extract the value from a `with_env` outcome, falling back to the default on
/// any JNI error or panic — the Kotlin side is only ever told success or an
/// error `jstring`, never made to observe a Rust panic.
trait OutcomeValue {
    type Value;
    fn into_value(self) -> Self::Value;
}

impl<T: Default> OutcomeValue for Outcome<T, jni::errors::Error> {
    type Value = T;

    fn into_value(self) -> T {
        match self {
            Outcome::Ok(value) => value,
            Outcome::Err(_) | Outcome::Panic(_) => T::default(),
        }
    }
}

fn cache_native_class(env: &mut jni::Env, class: &JClass) {
    if NATIVE_CLASS.get().is_some() {
        return;
    }
    if let Ok(global) = env.new_global_ref(class) {
        let _ = NATIVE_CLASS.set(global);
    }
}

fn cache_native_class_by_name(env: &mut jni::Env) {
    if NATIVE_CLASS.get().is_some() {
        return;
    }
    match env.find_class(JNIString::new(NATIVE_CLASS_NAME)) {
        Ok(class) => cache_native_class(env, &class),
        Err(_) => {
            env.exception_clear();
        }
    }
}

fn throw_string(env: &mut jni::Env, msg: &str) -> jni::errors::Result<jstring> {
    match JString::new(env, msg) {
        Ok(s) => Ok(s.into_raw()),
        Err(_) => Ok(std::ptr::null_mut()),
    }
}

fn start_proxy(args: &str) -> Result<(), String> {
    let mut state = STATE
        .lock()
        .map_err(|_| "proxy state lock poisoned".to_string())?;
    // Reap a worker that has already exited — a `--check` run, a startup
    // failure, or a Stop whose runtime has finished winding down — so the guard
    // below only ever sees a thread that is still doing something.
    if state.thread.as_ref().is_some_and(|t| t.is_finished()) {
        state.thread = None;
        state.stopping = false;
    }
    if state.thread.is_some() {
        // Refusing is the only thing that keeps exactly one worker alive at a
        // time, which is what makes `emit_listening`/`emit_stopped`/`emit_error`
        // unambiguous about which run they describe.  The wind-down window is
        // normally sub-millisecond, so this reads as "retry", not as a dead end.
        return Err(if state.stopping {
            "proxy is still stopping, try again in a moment".into()
        } else {
            "proxy is already running".into()
        });
    }

    let config = Config::try_from_cli_line(args)?;
    install_logging(&config);
    server::install_crypto_provider();

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let thread = std::thread::Builder::new()
        .name("tg-ws-proxy".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                // The default is one worker per core, i.e. 8+ threads with
                // stacks on a modern phone for an accept loop.  Two is enough
                // and is the right floor: nothing on a worker blocks (DNS goes
                // to the blocking pool via `lookup_host`), but the per-packet
                // AES-CTR and SHA-256 work is real CPU time, so a second
                // worker stays parked to drive the shared IO driver while the
                // first is mid-burst.
                .worker_threads(2)
                .thread_name("tg-ws-worker")
                .on_thread_start(attach_runtime_thread)
                .on_thread_stop(detach_runtime_thread)
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    emit_error(&format!("failed to start runtime: {e}"));
                    return;
                }
            };

            let result = rt.block_on(async move {
                let shutdown = async move {
                    let mut rx = shutdown_rx;
                    loop {
                        if *rx.borrow() {
                            break;
                        }
                        if rx.changed().await.is_err() {
                            break;
                        }
                    }
                };
                server::run_with_listen(config, shutdown, |info| {
                    emit_listening(&info.tg_link);
                })
                .await
            });

            // Wind down first, report second.  Reporting here would be true
            // about the port — the listener was dropped when `block_on`
            // returned — but not about the shim: `start_proxy` refuses with
            // "proxy is still stopping" until this thread is finished, and
            // Kotlin turns a reported stop straight into a "Start" button, so
            // reporting first meant Stop, "Stopped", Start, red error, nothing
            // started.  A reported stop has to mean startable.
            //
            // `shutdown_timeout` rather than a plain `drop(rt)`, which would
            // block indefinitely on an outstanding blocking task (a stuck
            // `getaddrinfo`).  So the cost of the new order is bounded: if
            // teardown really takes the full two seconds, the app keeps saying
            // "Running" for those two seconds, which is what is actually
            // happening.  `stop_proxy` still returns to the main thread at
            // once; only this report waits.
            rt.shutdown_timeout(Duration::from_secs(2));

            // Last statement in the thread body on purpose: whatever Kotlin
            // does on the callback, `thread.is_finished()` is a return away.
            match result {
                Ok(()) => emit_stopped(),
                Err(e) => emit_error(&format!("proxy failed: {e}")),
            }
        })
        .map_err(|e| format!("failed to spawn proxy thread: {e}"))?;

    state.shutdown = Some(shutdown_tx);
    state.thread = Some(thread);
    state.stopping = false;
    Ok(())
}

fn stop_proxy() {
    let mut state = match STATE.lock() {
        Ok(s) => s,
        Err(_) => return,
    };
    if let Some(tx) = state.shutdown.take() {
        let _ = tx.send(true);
    }
    // Deliberately no `join()`: this runs on the Android main thread, and the
    // worker's tail is a bounded runtime shutdown that may still be waiting on
    // a blocking task the OS has stalled (`getaddrinfo`) — an ANR is a much
    // worse outcome than a proxy that takes a moment to finish winding down.
    // The join handle stays in `STATE` so `start_proxy` can still tell "winding
    // down" from "idle".
    state.stopping = true;
}

fn install_logging(config: &Config) {
    let log_level = if config.quiet {
        "off"
    } else if config.verbose {
        // `jni` narrates every thread attach and detach at debug level. That is
        // this file's own plumbing rather than anything the proxy did, and it
        // arrives through tracing-log's bridge from inside the very forwarding
        // path that produced it, so it is noise in the Log panel and a loop
        // waiting for the guard on [`IN_JNI_CALL`] to slip.
        "debug,jni=off"
    } else {
        "info"
    };

    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| log_level.into());

    if let Some(handle) = LOG_FILTER.get() {
        // Reloading also rebuilds tracing's interest cache, which recomputes
        // the global max level — without that a first Start at `off` would pin
        // it to OFF and no later `info!` would ever reach a callsite.
        let _ = handle.reload(env_filter);
        return;
    }

    let (filter, handle) = reload::Layer::new(env_filter);
    // `Registry` + `fmt::layer()` is the same stack the `fmt()` builder used,
    // so the line format is unchanged; it is spelled out only because the
    // builder gives no way to wrap its filter in a reloadable layer.
    let installed = Registry::default()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_target(false)
                .with_writer(AndroidMakeWriter),
        )
        .try_init()
        .is_ok();
    if installed {
        let _ = LOG_FILTER.set(handle);
    }
}

struct AndroidMakeWriter;

impl<'a> MakeWriter<'a> for AndroidMakeWriter {
    type Writer = AndroidWriter;

    fn make_writer(&'a self) -> Self::Writer {
        AndroidWriter { buf: Vec::new() }
    }
}

struct AndroidWriter {
    buf: Vec<u8>,
}

impl Write for AndroidWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(data);
        drain_lines(&mut self.buf);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.buf.is_empty() {
            let line = String::from_utf8_lossy(&self.buf).into_owned();
            self.buf.clear();
            forward_line(&line);
        }
        Ok(())
    }
}

impl Drop for AndroidWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

fn drain_lines(buf: &mut Vec<u8>) {
    while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        let mut line: Vec<u8> = buf.drain(..=pos).collect();
        line.pop();
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        if !line.is_empty() {
            forward_line(&String::from_utf8_lossy(&line));
        }
    }
}

fn forward_line(line: &str) {
    logcat(line);
    emit_log(line);
}

fn logcat(line: &str) {
    const ANDROID_LOG_INFO: i32 = 4;
    #[link(name = "log")]
    unsafe extern "C" {
        fn __android_log_write(
            prio: i32,
            tag: *const std::ffi::c_char,
            text: *const std::ffi::c_char,
        ) -> i32;
    }

    let Ok(text) = std::ffi::CString::new(line.replace('\0', "")) else {
        return;
    };
    let tag = c"tg-ws-proxy";
    // SAFETY: both pointers are valid C strings for the duration of the call.
    unsafe {
        __android_log_write(ANDROID_LOG_INFO, tag.as_ptr(), text.as_ptr());
    }
}

fn emit_log(line: &str) {
    call_static("onNativeLog", line);
}

/// Report a startup failure the way `nativeStart`'s return value would have,
/// had it been synchronous: the app flips out of the running state and shows
/// the message as the current error instead of a scrollback-only log line.
fn emit_error(line: &str) {
    logcat(line);
    call_static("onNativeError", line);
}

fn emit_listening(link: &str) {
    call_static("onNativeListening", link);
}

/// Report a clean worker exit — `--check` success or a graceful shutdown —
/// so the app leaves the running state even though no error occurred.
fn emit_stopped() {
    call_static_void("onNativeStopped");
}

fn call_static(method: &str, arg: &str) {
    call_static_impl(|env, class| {
        let jarg = JString::new(env, arg)?;
        let jarg: JObject = jarg.into();
        env.call_static_method(
            class,
            JNIString::new(method),
            &LOG_SIG,
            &[JValue::Object(&jarg)],
        )
    });
}

fn call_static_void(method: &str) {
    call_static_impl(|env, class| {
        env.call_static_method(class, JNIString::new(method), &VOID_SIG, &[])
    });
}

/// Attach a Tokio runtime thread to the JVM as it starts.
///
/// [`jni::JavaVM::attach_current_thread`] requests a *permanent* attachment
/// (jni's `AttachConfig::scoped` defaults to false), tracked in the crate's own
/// thread-local, so this is not about avoiding an attach per log line —
/// [`call_static_impl`] already pays that only once per thread.  It is about
/// tokio's blocking-pool threads, which are created on demand and reaped after
/// ten idle seconds: every replacement thread that logged used to pay a full
/// `AttachCurrentThread` on its first line.  Doing it here instead makes the
/// attachment lifetime match the thread's, for workers and pool threads alike
/// (tokio spawns its multi-thread workers onto the blocking pool, so both kinds
/// run this callback).
fn attach_runtime_thread() {
    let Some(_reentry) = JniReentryGuard::acquire() else {
        return;
    };
    if let Some(vm) = JVM.get() {
        let _ = vm.attach_current_thread(|_env| jni::errors::Result::Ok(()));
    }
}

/// Detach explicitly rather than leaving it to the thread-local destructor, so
/// a pool thread the runtime is recycling is off the JVM's books before it
/// exits.  Safe here because tokio runs this after the pool loop has returned:
/// no `Env` or `AttachGuard` is left on the stack, which is the one case
/// `detach_current_thread` refuses.
fn detach_runtime_thread() {
    // Held across the detach itself, not just around a callback: jni logs the
    // detach, and forwarding that line would re-attach this thread on its way
    // out. That is the abort described on [`IN_JNI_CALL`].
    let Some(_reentry) = JniReentryGuard::acquire() else {
        return;
    };
    if let Some(vm) = JVM.get() {
        let _ = vm.detach_current_thread();
    }
}

thread_local! {
    /// True while this thread is inside a JNI call this file made.
    ///
    /// `jni` reports every attach and detach through the `log` crate, and
    /// `SubscriberInitExt::init` installs tracing-log's bridge, so those
    /// records reach [`AndroidMakeWriter`] like any other line.  Forwarding one
    /// enters [`call_static_impl`], which attaches the thread, which logs.  On
    /// the detach path it is worse than a loop: the forwarded line re-attaches
    /// the very thread jni is in the middle of detaching, and the process took
    /// SIGABRT on a tokio worker the first time anything ran with `--verbose`.
    ///
    /// Every line still reaches logcat, which is a plain FFI write and cannot
    /// re-enter.  What this suppresses is only the nested trip into Kotlin.
    static IN_JNI_CALL: Cell<bool> = const { Cell::new(false) };
}

/// Holds [`IN_JNI_CALL`] for as long as it lives, so an early return or an
/// unwind cannot leave the flag stuck — which would silence the app's Log
/// panel for the rest of the process while logcat kept filling.
struct JniReentryGuard;

impl JniReentryGuard {
    /// `None` when this thread is already inside a JNI call, i.e. when the
    /// caller is a log line produced by one.
    fn acquire() -> Option<Self> {
        // `try_with`, not `with`: a line can still be forwarded while the
        // thread is being torn down (`AndroidWriter` flushes from its `Drop`),
        // and by then the thread-local may already be gone. `with` panics
        // there, and a panic across the JNI boundary aborts the process --
        // which is the failure this whole guard exists to prevent. A destroyed
        // flag reads as "already inside a call", so the line goes to logcat
        // and no further.
        IN_JNI_CALL
            .try_with(|in_call| {
                if in_call.get() {
                    None
                } else {
                    in_call.set(true);
                    Some(Self)
                }
            })
            .unwrap_or(None)
    }
}

impl Drop for JniReentryGuard {
    fn drop(&mut self) {
        let _ = IN_JNI_CALL.try_with(|in_call| in_call.set(false));
    }
}

/// Left deliberately self-attaching: most startup lines are emitted from the
/// "tg-ws-proxy" thread that drives `block_on`, which the runtime did not
/// create and [`attach_runtime_thread`] therefore never sees.
fn call_static_impl(
    call: impl for<'l> FnOnce(
        &mut jni::Env<'l>,
        &Global<JClass<'static>>,
    ) -> jni::errors::Result<JValueOwned<'l>>,
) {
    // Dropped at the end of the call; see [`IN_JNI_CALL`].
    let Some(_reentry) = JniReentryGuard::acquire() else {
        return;
    };
    let Some(vm) = JVM.get() else {
        return;
    };
    let Some(class) = NATIVE_CLASS.get() else {
        return;
    };
    let _ = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
        if call(env, class).is_err() {
            // A pending exception on a detached worker is fatal (this is what
            // crashed Start: ClassNotFoundException from FindClass).
            env.exception_clear();
        }
        Ok(())
    });
}
