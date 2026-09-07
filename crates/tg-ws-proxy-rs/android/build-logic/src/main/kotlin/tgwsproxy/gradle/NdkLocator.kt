package tgwsproxy.gradle

import org.gradle.api.GradleException
import java.io.File
import java.util.Locale

internal object NdkLocator {
    fun resolveNdkRoot(explicitNdkRoot: String, androidSdkRoot: String, pinnedVersion: String): File {
        val explicit = explicitNdkRoot.trim()
        if (explicit.isNotEmpty()) {
            return File(explicit)
        }

        val pinned = File(androidSdkRoot).resolve("ndk/$pinnedVersion")
        if (!pinned.isDirectory) {
            throw GradleException(
                "pinned NDK $pinnedVersion not found at ${pinned.absolutePath}; " +
                    "run: sdkmanager --install \"ndk;$pinnedVersion\"",
            )
        }

        return pinned
    }

    fun hostTag(ndkRoot: File): String {
        val os = System.getProperty("os.name").lowercase(Locale.US)
        val arch = System.getProperty("os.arch").lowercase(Locale.US)
        return when {
            os.contains("windows") -> "windows-x86_64"
            os.contains("mac") || os.contains("darwin") -> {
                if ((arch == "aarch64" || arch == "arm64") &&
                    ndkRoot.resolve("toolchains/llvm/prebuilt/darwin-arm64").isDirectory
                ) {
                    "darwin-arm64"
                } else {
                    "darwin-x86_64"
                }
            }
            else -> "linux-x86_64"
        }
    }

    fun ndkExecutable(path: File): File {
        if (path.exists()) {
            return path
        }
        if (isWindows()) {
            val cmd = File("${path.absolutePath}.cmd")
            if (cmd.exists()) {
                return cmd
            }
            val exe = File("${path.absolutePath}.exe")
            if (exe.exists()) {
                return exe
            }
        }
        return path
    }

    fun hostExecutable(name: String): String =
        if (isWindows()) "$name.cmd" else name

    private fun isWindows(): Boolean =
        System.getProperty("os.name").lowercase(Locale.US).contains("windows")
}
