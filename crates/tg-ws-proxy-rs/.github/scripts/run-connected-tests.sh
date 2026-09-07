#!/usr/bin/env bash
#
# Run the Android instrumentation suite on the emulator and pull back the APK
# the device was actually given.  Invoked from the `android` directory by the
# `android-emulator` job in .github/workflows/ci.yml.
#
# A file rather than lines in that job's `script:` block, for two reasons.
# reactivecircus/android-emulator-runner hands each line to `sh -c "<line>"`,
# so a line carrying double quotes of its own -- and the `adb pull "$(adb shell
# pm path ...)"` below needs them -- is cut short at the first inner quote and
# reaches sh as an unterminated command substitution.  And keeping both
# commands here puts them under this file's `set -euo pipefail` instead of
# leaving "did the suite actually pass?" resting on how the action chooses to
# execute the block.
set -euo pipefail

# The emulator dies with the action's step, so anything the device knows has to
# be collected here -- a later workflow step has no adb to talk to. An
# instrumentation process that crashes natively leaves no assertion text in the
# Gradle report at all ("Instrumentation run failed due to Process crashed"),
# so without this the only evidence of a JNI-level fault is that it happened.
dump_device_state() {
    local dest="$RUNNER_TEMP/device-logs"
    mkdir -p "$dest"
    adb logcat -d -b crash -v threadtime > "$dest/logcat-crash.txt" 2>&1 || true
    adb logcat -d -b main -v threadtime > "$dest/logcat-main.txt" 2>&1 || true
    # Root is available on the emulator images this job uses; a tombstone
    # carries the faulting frame that logcat's crash buffer can truncate.
    if adb root > /dev/null 2>&1; then
        adb shell 'ls /data/tombstones 2>/dev/null' > "$dest/tombstone-list.txt" 2>&1 || true
        adb shell 'cat /data/tombstones/* 2>/dev/null' > "$dest/tombstones.txt" 2>&1 || true
    fi
    # Echo the crash buffer into the job log too: the artifact is the full
    # story, but a red job should say why without a download.
    if [[ -s "$dest/logcat-crash.txt" ]]; then
        echo "----- logcat -b crash (tail) -----"
        tail -n 120 "$dest/logcat-crash.txt"
        echo "----- end logcat -----"
    fi
    # Native aborts from the shim land in the main buffer under our own tag or
    # as an ART fatal signal; surface those lines specifically.
    grep -aE 'tg-ws-proxy|libtg_ws_proxy_jni|Fatal signal|art::|JNI DETECTED|dalvikvm' \
        "$dest/logcat-main.txt" | tail -n 80 || true
}

# leaveApksInstalledAfterRun: AGP uninstalls both APKs the moment the suite
# ends, and the caller's next step has to see what the device was given.
status=0
./gradlew :app:connectedDebugAndroidTest \
    -Pandroid.injected.androidTest.leaveApksInstalledAfterRun=true || status=$?

dump_device_state

if [[ "$status" -ne 0 ]]; then
    exit "$status"
fi

# The bytes, not a line out of Gradle's log: the log says which file AGP meant
# to send, the device says which one it got, and telling those two apart is the
# entire point of the check that follows this script.
paths="$(adb shell pm path "$APP_ID" | tr -d '\r' | sed -n 's/^package://p')"
if [[ -z "$paths" ]]; then
    echo "::error::pm path reported no installed APK for $APP_ID -- the suite" \
        "cannot have run against this build"
    exit 1
fi
# More than one path means a split install (base.apk plus configuration APKs),
# which is not what this job builds; pulling an arbitrary one of them would
# make the hash comparison downstream meaningless rather than wrong-looking.
if [[ "$(wc -l <<<"$paths")" -ne 1 ]]; then
    echo "::error::expected exactly one installed APK for $APP_ID, got:"
    echo "$paths"
    exit 1
fi
adb pull "$paths" "$RUNNER_TEMP/installed.apk"
