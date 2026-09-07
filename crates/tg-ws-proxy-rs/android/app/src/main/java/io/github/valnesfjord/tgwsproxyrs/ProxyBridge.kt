package io.github.valnesfjord.tgwsproxyrs

import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * Process-wide state the service and the UI both read.
 *
 * Native callbacks land here so a configuration change cannot drop the
 * listener that `crates/android-jni/src/android.rs` calls by class name.
 */
object ProxyBridge {
    private val tgLinkRegex = Regex("""tg://proxy\?[^\s]+""")

    private val _logs = MutableSharedFlow<String>(extraBufferCapacity = 256)
    val logs: SharedFlow<String> = _logs.asSharedFlow()

    /**
     * Whether a proxy is up.
     *
     * `false` is a promise as much as a description.  The UI turns it straight
     * into a Start button, and the shim refuses a start while the previous run
     * is still winding down, so this may only drop once that run is startable
     * again — every writer below owes it that.
     */
    private val _running = MutableStateFlow(false)
    val running: StateFlow<Boolean> = _running.asStateFlow()

    private val _tgLink = MutableStateFlow<String?>(null)
    val tgLink: StateFlow<String?> = _tgLink.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    fun setRunning(value: Boolean) {
        _running.value = value
        if (!value) {
            _tgLink.value = null
        }
    }

    fun reportError(message: String) {
        _error.value = message
        _logs.tryEmit(message)
        setRunning(false)
    }

    fun reportMessage(message: String) {
        _error.value = message
        _logs.tryEmit(message)
    }

    fun clearError() {
        _error.value = null
    }

    fun onLog(line: String) {
        _logs.tryEmit(line)
        tgLinkRegex.find(line)?.let { _tgLink.value = it.value }
    }

    fun onListening(link: String) {
        _tgLink.value = link
        _running.value = true
    }

    /**
     * Adopt a worker the shim still has while [running] says otherwise — the
     * state a fresh activity or a fresh process comes up in.
     *
     * One-way on purpose.  `nativeIsRunning()` answers false from the moment
     * Stop is pressed, whatever the worker is still doing, while the shim goes
     * on refusing starts until that worker is reaped; lowering [running] on a
     * false answer would break the promise above and put a Start button in
     * front of a start the shim answers with "proxy is still stopping".  That
     * window is reachable from the notification shade: tapping Stop there
     * resumes [MainActivity], and `onResume` calls this.  A true answer carries
     * no such ambiguity — the shim refuses starts for exactly as long as it
     * says true.
     *
     * So leaving the running state stays the stop path's job: the worker's
     * report, or [ProxyService] once it can see no report is coming.  A state
     * stuck on true is not a dead end either — the button that stale state
     * shows is Stop, and taking it clears it.
     */
    fun syncFromNative() {
        if (NativeProxy.nativeIsRunning()) {
            _running.value = true
        }
    }
}
