package io.github.valnesfjord.tgwsproxyrs

/**
 * JNI boundary for `crates/android-jni/src/android.rs`.
 *
 * The native side calls [onNativeLog], [onNativeListening], [onNativeError]
 * and [onNativeStopped] on whichever thread the Tokio runtime happens to be
 * on; [ProxyBridge] hops to the main thread for UI observers.
 */
object NativeProxy {
    init {
        System.loadLibrary("tg_ws_proxy_jni")
    }

    fun load() {
        // Touch the object so `init` runs from Application.onCreate.
    }

    @JvmStatic
    external fun nativeStart(args: String): String?

    @JvmStatic
    external fun nativeStop()

    @JvmStatic
    external fun nativeIsRunning(): Boolean

    @JvmStatic
    fun onNativeLog(line: String) {
        ProxyBridge.onLog(line)
    }

    @JvmStatic
    fun onNativeListening(link: String) {
        ProxyBridge.onListening(link)
    }

    @JvmStatic
    fun onNativeError(message: String) {
        ProxyBridge.reportError(message)
    }

    @JvmStatic
    fun onNativeStopped() {
        ProxyBridge.setRunning(false)
    }
}
