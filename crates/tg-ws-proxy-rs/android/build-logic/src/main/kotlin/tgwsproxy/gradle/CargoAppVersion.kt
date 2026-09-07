package tgwsproxy.gradle

import org.gradle.api.GradleException
import org.gradle.api.file.RegularFile
import org.gradle.api.provider.Provider
import org.gradle.api.provider.ProviderFactory

class CargoAppVersion(
    val versionName: Provider<String>,
    val versionCode: Provider<Int>,
)

fun ProviderFactory.cargoAppVersion(cargoToml: RegularFile): CargoAppVersion {
    val version = fileContents(cargoToml).asText.map { text ->
        val parts = Regex("""^version\s*=\s*"(\d+)\.(\d+)\.(\d+)"""", RegexOption.MULTILINE)
            .find(text)
            ?.groupValues
            ?.drop(1)
            ?: throw GradleException("could not parse package.version from Cargo.toml")
        val versionCode = Regex("""^version_code\s*=\s*(\d+)""", RegexOption.MULTILINE)
            .find(text)
            ?.groupValues
            ?.get(1)
            ?.toInt()
            ?: throw GradleException("could not parse package.metadata.android.version_code from Cargo.toml")
        val (major, minor, patch) = parts.map(String::toInt)
        val expectedVersionCode = (major * 10000 + minor * 100 + patch) * 10
        if (versionCode != expectedVersionCode) {
            throw GradleException(
                "package.metadata.android.version_code must be $expectedVersionCode for ${parts.joinToString(".")}",
            )
        }
        parts.joinToString(".") to versionCode
    }
    return CargoAppVersion(
        versionName = version.map { it.first },
        versionCode = version.map { it.second },
    )
}
