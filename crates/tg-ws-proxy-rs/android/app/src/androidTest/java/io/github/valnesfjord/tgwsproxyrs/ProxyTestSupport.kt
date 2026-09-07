package io.github.valnesfjord.tgwsproxyrs

import android.os.SystemClock
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.onSubscription
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import java.util.concurrent.CountDownLatch
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

/**
 * A secret spelled out instead of generated.
 *
 * `Config::with_defaults` invents a random one when `--secret` is absent, and a
 * fixed value is what lets the link test assert the whole `secret=` field
 * rather than just its shape.
 */
internal const val TEST_SECRET = "00112233445566778899aabbccddeeff"

/**
 * Arguments that bring a listener up with no startup network I/O whatsoever.
 *
 * Deliberately not [ProxyService.DEFAULT_ARGS]: that carries
 * `--default-domains`, which fetches a domain list over HTTPS *before* the bind
 * in `src/server.rs`, so on a CI emulator with no route out every start would
 * first have to wait out that failure before `onNativeListening` could fire.
 *
 * `--host`/`--link-ip` pin both addresses so neither goes through LAN-IP
 * autodetect, `--port 0` lets the kernel choose so nothing can collide with a
 * socket a previous test has not finished releasing, and `--pool-size 0` drops
 * the upstream warm-up, which is the only outbound work left after the bind.
 */
internal const val OFFLINE_ARGS =
    "--host 127.0.0.1 --port 0 --link-ip 127.0.0.1 --pool-size 0 --secret $TEST_SECRET"

/** How long a start is given to reach `onNativeListening`. */
internal const val LISTEN_TIMEOUT_MS = 15_000L

/**
 * How long the native worker is given to wind down after a stop.
 *
 * Generous on purpose: the shim's own tail is a bounded
 * `rt.shutdown_timeout(2s)`, and this is the budget for "the worker eventually
 * goes away", which is a different question from the tightly bounded one
 * [NativeProxyStopLatencyTest] asks about the calling thread.
 */
internal const val SHUTDOWN_TIMEOUT_MS = 20_000L

/**
 * `nativeStart`, with the one refusal that is a timing artefact waited out.
 *
 * `start_proxy` refuses — before it so much as looks at the arguments — while
 * it still holds a worker it cannot reap, and `stop_proxy` deliberately does
 * not join that worker (joining on the Android main thread is the ANR this
 * suite exists to prevent). The window is now microseconds wide, since the shim
 * reports a stop only once the runtime has wound down, but it is still the gap
 * between that report and the worker thread actually returning.
 *
 * Every other error is handed back to the caller on the first try, so a
 * rejected argument or a failed bind fails a test where it happens instead of
 * being retried into a timeout.
 */
private fun startPastWindDown(args: String): String? {
    val deadline = SystemClock.elapsedRealtime() + SHUTDOWN_TIMEOUT_MS
    while (true) {
        val error = NativeProxy.nativeStart(args) ?: return null
        if (!error.contains("still stopping")) {
            return error
        }
        if (SystemClock.elapsedRealtime() > deadline) {
            throw AssertionError("previous run never finished winding down: $error")
        }
        Thread.sleep(50)
    }
}

/** Start the proxy, failing the test on any error the shim reports. */
internal fun startProxy(args: String) {
    val error = startPastWindDown(args) ?: return
    throw AssertionError("nativeStart(\"$args\") failed: $error")
}

/**
 * Ask for a start the caller expects the shim to reject on its *arguments*, and
 * hand back that rejection.
 *
 * Telling the two refusals apart matters to more than the error text. "still
 * stopping" is produced by the guard at the top of `start_proxy` and leaves the
 * `stopping` flag set, which pins `nativeIsRunning()` to false; the argument
 * rejection is reached only once that guard has passed, having reaped any
 * finished worker and cleared the flag with it. So a caller that mistook the
 * first for the second would go on to assert with a `nativeIsRunning()` that
 * can no longer answer anything.
 */
internal fun awaitRejection(args: String): String =
    startPastWindDown(args)
        ?: throw AssertionError("nativeStart(\"$args\") was accepted; expected a rejection")

/** Wait for the `tg://` link the `on_listen` callback publishes. */
internal fun awaitLink(): String = runBlocking {
    withTimeout(LISTEN_TIMEOUT_MS) {
        // `running` before the link, because [ProxyBridge.tgLink] has two
        // writers and only one of them means "this run is established".
        // `onListening` sets the link and `running` together, but
        // `ProxyBridge.onLog` also scrapes the `tg://` line out of the startup
        // banner, and that arrives first and sets the link alone.
        //
        // Returning on the scraped link leaves `running` false, and then
        // [stopAndWait]'s wait for it to go false is satisfied instantly by a
        // run that has not even reported itself yet -- so a caller that stops
        // and then asserts the port is closed races the listener it is asking
        // about. That is exactly how stopReturnsPromptlyOnTheMainThread failed
        // on the API 26 leg while passing on the faster API 36 one.
        ProxyBridge.running.first { it }
        ProxyBridge.tgLink.filterNotNull().first()
    }
}

/** The port `--port 0` settled on, as published in the `tg://` link. */
internal fun portOf(link: String): Int =
    requireNotNull(Regex("&port=(\\d+)&").find(link)) { "no port in $link" }
        .groupValues[1]
        .toInt()

/**
 * Stop, and wait until the run has actually reported itself finished.
 *
 * `nativeIsRunning()` answers false the instant Stop is requested — that is
 * what keeps the UI from repainting "Running" over a proxy with no listener —
 * so it cannot be the signal here. `ProxyBridge.running` can: it only goes
 * false once the worker has called `onNativeStopped` (or `onNativeError`),
 * which the shim emits as the last statement of the worker thread, after
 * `run_with_listen` has returned *and* the runtime has been torn down. So when
 * this returns, the port is free and nothing of that run is still running to
 * emit a log line.
 *
 * The wait is bounded and throws on timeout, which is what makes it, rather
 * than any follow-up assertion, the thing that proves a stop took effect.
 */
internal fun stopAndWait() {
    if (NativeProxy.nativeIsRunning()) {
        NativeProxy.nativeStop()
    }
    runBlocking { withTimeout(SHUTDOWN_TIMEOUT_MS) { ProxyBridge.running.first { !it } } }
}

/**
 * Drop the state a previous test left behind.
 *
 * [ProxyBridge.tgLink] is a `StateFlow`, so without this a test could satisfy
 * [awaitLink] instantly with the *previous* run's link and never actually wait
 * for its own listener. `setRunning(false)` is what nulls it; call this only
 * once [stopAndWait] has confirmed nothing is running.
 */
internal fun resetBridge() {
    ProxyBridge.setRunning(false)
    ProxyBridge.clearError()
}

/**
 * Collects [ProxyBridge.logs] from before the proxy is started.
 *
 * The flow replays nothing, so a collector attached after `nativeStart` would
 * miss the startup banner and everything racing it. `onSubscription` fires
 * once the collector is registered and is the only race-free "I am attached"
 * signal; sleeping here instead would be the flakiest line in the suite.
 *
 * Replay-0 also cuts the other way, which is half of what makes the negative
 * assertion safe: a line a *previous* run emits before this capture subscribes
 * can never be delivered to it. The other half is [stopAndWait], which the
 * previous phase ends with — the run that emitted those lines is torn down, not
 * merely asked to stop, before this capture exists.
 */
internal class LogCapture : AutoCloseable {
    private val scope = CoroutineScope(Dispatchers.Default)
    private val delivered = LinkedBlockingQueue<String>()
    private val job: Job

    init {
        val attached = CountDownLatch(1)
        job = scope.launch {
            ProxyBridge.logs
                .onSubscription { attached.countDown() }
                .collect { line -> delivered.put(line) }
        }
        check(attached.await(5, TimeUnit.SECONDS)) { "log collector never attached" }
    }

    /**
     * Block for the next line this capture has not been handed yet, so "a line
     * arrived" is a wait rather than a sleep. Null means the timeout expired.
     *
     * A `SharedFlow` hands one collector its values in emission order, so a
     * caller can emit a line of its own and read this returning that line as
     * proof that nothing emitted earlier is merely still in flight.
     */
    fun awaitLine(timeoutMs: Long = LISTEN_TIMEOUT_MS): String? =
        delivered.poll(timeoutMs, TimeUnit.MILLISECONDS)

    override fun close() {
        job.cancel()
    }
}
