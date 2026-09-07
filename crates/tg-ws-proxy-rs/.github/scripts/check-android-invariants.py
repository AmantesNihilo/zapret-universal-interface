#!/usr/bin/env python3
import re
import sys
import xml.etree.ElementTree as ET

ANDROID = "{http://schemas.android.com/apk/res/android}"

MANIFEST = "android/app/src/main/AndroidManifest.xml"
STRINGS = "android/app/src/main/res/values/strings.xml"
SHIM = "crates/android-jni/src/android.rs"
VIEW_MODEL = (
    "android/app/src/main/java/io/github/valnesfjord/tgwsproxyrs/ProxyViewModel.kt"
)

failed = False


def fail(path, message):
    global failed
    failed = True
    print("::error file=%s::%s" % (path, message))


def source(path):
    # Every check starts here, so a file that was moved or renamed
    # fails the step by name instead of leaving a check to search
    # nothing and pass. Whoever moved it has to move the path above.
    try:
        with open(path, encoding="utf-8") as handle:
            return handle.read()
    except OSError as exc:
        fail(path, "%s is gone (%s). It holds an invariant this step "
             "asserts; update the path in this step to wherever the "
             "file now lives." % (path, exc.strerror))
        return None


def parsed(path):
    text = source(path)
    if text is None:
        return None
    try:
        return ET.fromstring(text)
    except ET.ParseError as exc:
        fail(path, "is not parseable XML (%s), so the invariant it "
             "carries cannot be checked." % exc)
        return None


def check_allow_backup():
    root = parsed(MANIFEST)
    if root is None:
        return
    app = root.find("application")
    if app is None:
        fail(MANIFEST, "has no <application> element to carry "
             "android:allowBackup.")
        return
    value = app.get(ANDROID + "allowBackup")
    if value == "false":
        print("ok: %s keeps android:allowBackup=\"false\"" % MANIFEST)
        return
    seen = ("absent, and Android defaults it to true"
            if value is None else "\"%s\"" % value)
    fail(MANIFEST, "android:allowBackup on <application> is %s, "
         "expected \"false\". The one thing this app persists is the "
         "argument string, and that string can carry the proxy "
         "secret (--secret <32 hex>). With backup enabled that secret "
         "leaves the device: cloud backup, adb backup, and "
         "device-to-device transfer to a new phone." % seen)


def check_launcher_label():
    root = parsed(STRINGS)
    if root is None:
        return
    found = {}
    for node in root.findall("string"):
        name = node.get("name")
        if name in ("app_name", "app_title"):
            found[name] = "".join(node.itertext()).strip()
    missing = [k for k in ("app_name", "app_title") if k not in found]
    if missing:
        fail(STRINGS, "defines no %s. app_name is the launcher label "
             "and the notification title; app_title is the in-app "
             "header. They are two strings on purpose." %
             " and no ".join(missing))
        return
    if found["app_name"] == found["app_title"]:
        fail(STRINGS, "app_name and app_title are both \"%s\". They "
             "have to stay distinct: app_name is what the launcher "
             "prints under the icon and it truncates around 11 "
             "characters, so giving it the long in-app header is what "
             "puts a cut-off label on the home screen. Only the "
             "header inside the app has room for app_title." %
             found["app_name"])
        return
    print("ok: %s keeps app_name and app_title distinct" % STRINGS)


def check_worker_threads():
    text = source(SHIM)
    if text is None:
        return
    # \s* throughout, and an optional trailing comma, so a rustfmt
    # pass that wraps the builder chain does not read as a removal --
    # rustfmt adds that comma itself when the call goes multi-line.
    match = re.search(
        r"\.\s*worker_threads\s*\(\s*(\d[\d_]*)(?:usize)?\s*,?\s*\)",
        text,
    )
    if match is None:
        fail(SHIM, "the tokio runtime builder no longer sets "
             ".worker_threads(N). Left at the default tokio starts "
             "one worker per core, so a phone gets 8 or more threads, "
             "each with its own stack, to run an accept loop.")
        return
    threads = int(match.group(1).replace("_", ""))
    # A range, not the literal 2: retuning the pool is a real decision
    # someone may make, dropping the cap is the regression. Anything
    # this side of 4 is still a fixed small pool rather than the
    # per-core default.
    if not 1 <= threads <= 4:
        fail(SHIM, ".worker_threads(%d) is no longer a small fixed "
             "pool. The point of setting it at all is that a phone "
             "must not get one worker per core; if this really needs "
             "to grow, widen the bound in this step and say why."
             % threads)
        return
    print("ok: %s pins the runtime to %d workers" % (SHIM, threads))


def check_log_buffer():
    text = source(VIEW_MODEL)
    if text is None:
        return
    declaration = re.search(
        r"\bconst\s+val\s+MAX_LOG_LINES\s*(?::\s*Int\s*)?=\s*(\d[\d_]*)",
        text,
    )
    if declaration is None:
        fail(VIEW_MODEL, "declares no MAX_LOG_LINES. The log list the "
             "UI observes is appended to for the life of the process, "
             "so without a cap a long-running proxy keeps every line "
             "it ever emitted, and the Compose list keeps a row for "
             "each one.")
        return
    cap = int(declaration.group(1).replace("_", ""))
    if not 1 <= cap <= 100000:
        fail(VIEW_MODEL, "MAX_LOG_LINES is %d, which is not a bound "
             "that limits anything in practice." % cap)
        return
    # Declared is not enough: the regression that matters is the trim
    # going away and the constant being left behind, which reads as
    # bounded and is not. Requiring a use elsewhere in the file keeps
    # this honest without pinning the trim to one spelling -- removeAt
    # on the head, a ring buffer or a windowed copy all satisfy it.
    if len(re.findall(r"\bMAX_LOG_LINES\b", text)) < 2:
        fail(VIEW_MODEL, "MAX_LOG_LINES is declared but never read, "
             "so nothing trims the log list. The constant makes the "
             "buffer look bounded while it grows without limit.")
        return
    print("ok: %s bounds the log buffer at %d lines" % (VIEW_MODEL, cap))


check_allow_backup()
check_launcher_label()
check_worker_threads()
check_log_buffer()

if failed:
    sys.exit(1)
