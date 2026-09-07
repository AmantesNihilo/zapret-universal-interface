import com.android.build.api.dsl.ApplicationExtension
import com.android.build.api.variant.ApplicationAndroidComponentsExtension
import com.android.build.api.variant.FilterConfiguration
import org.gradle.api.GradleException
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.dsl.KotlinAndroidProjectExtension
import tgwsproxy.gradle.AndroidAbi
import tgwsproxy.gradle.CargoNdkBuildTask
import tgwsproxy.gradle.cargoAppVersion
import tgwsproxy.gradle.releaseSigning
import java.io.File

// Cargo NDK, Cargo.toml versioning, and optional release signing live here so
// :app's build script can stay the module identity + dependencies.

val libs = extensions.getByType<VersionCatalogsExtension>().named("libs")
val repositoryRoot = layout.settingsDirectory.dir("..")
val generatedJniLibsDir = layout.buildDirectory.dir("generated/rustJniLibs")

// Native code is always release unless TG_ANDROID_RUST_PROFILE=debug because a
// debug libtg_ws_proxy_jni.so is roughly 90 MB per ABI and too slow for a proxy.
val rustProfileProvider = providers.environmentVariable("TG_ANDROID_RUST_PROFILE").orElse("release")
val gradleAbisProvider = providers.gradleProperty("TG_ANDROID_ABIS").map { value ->
    value.split(Regex("[,\\s]+")).filter(String::isNotBlank).also { abis ->
        if (abis.size != 1 || abis.single() !in AndroidAbi.defaultAbis) {
            throw GradleException(
                "Gradle property TG_ANDROID_ABIS must be one of ${AndroidAbi.defaultAbis.joinToString()}, got '$value'",
            )
        }
    }
}
val rustAbisProvider = gradleAbisProvider
    .orElse(
        providers.environmentVariable("TG_ANDROID_ABIS")
            .map { value -> value.split(Regex("[,\\s]+")).filter(String::isNotBlank) },
    )
    .orElse(AndroidAbi.defaultAbis)
val androidApiProvider = providers.environmentVariable("TG_ANDROID_API")
    .map(String::toInt)
    .orElse(26)
val androidSdkRootProvider = providers.environmentVariable("ANDROID_HOME")
    .orElse(providers.environmentVariable("ANDROID_SDK_ROOT"))
    .orElse(providers.provider { File(System.getProperty("user.home"), "Android/Sdk").absolutePath })
val androidNdkRootProvider = providers.environmentVariable("ANDROID_NDK_HOME")
    .orElse(providers.environmentVariable("ANDROID_NDK"))
    .orElse("")

// Cargo.toml is the app version source so F-Droid and Gradle read the same
// fixed values. CargoAppVersion rejects a version/version_code mismatch.
val cargoVersion = providers.cargoAppVersion(repositoryRoot.file("Cargo.toml"))
val abiCodes = mapOf("armeabi-v7a" to 1, "arm64-v8a" to 2, "x86_64" to 3)

// Single source of truth for the NDK: AGP's strip pass and cargoNdk's
// toolchain both resolve this, CI installs it from the same line, and the
// F-Droid recipe reads the same file.
val pinnedNdk = libs.findVersion("ndk").get().requiredVersion

// Release signing. Keep android/keystore.properties out of version control and
// fill in storeFile/storePassword/keyAlias/keyPassword; CI and scripts can pass
// the same values as the TG_ANDROID_STORE_FILE / TG_ANDROID_STORE_PASSWORD /
// TG_ANDROID_KEY_ALIAS / TG_ANDROID_KEY_PASSWORD environment variables. With no
// keystore configured the release APK is left unsigned, as before. Providers
// are used (not System.getenv) so the configuration cache invalidates when the
// keystore or environment changes.
val signing = providers.releaseSigning(layout.settingsDirectory.file("keystore.properties"))

val cargoNdk = tasks.register<CargoNdkBuildTask>("cargoNdk") {
    group = "build"
    description = "Cross-compile libtg_ws_proxy_jni.so into generated Android jniLibs"
    repoRoot.set(repositoryRoot)
    rustSources.set(repositoryRoot.dir("src"))
    jniSources.set(repositoryRoot.dir("crates/android-jni/src"))
    cargoToml.set(repositoryRoot.file("Cargo.toml"))
    jniCargoToml.set(repositoryRoot.file("crates/android-jni/Cargo.toml"))
    cargoLock.set(repositoryRoot.file("Cargo.lock"))
    cargoConfig.fileProvider(
        providers.provider {
            repositoryRoot.file(".cargo/config.toml").asFile.takeIf(File::isFile)
        },
    )
    jniLibsDir.set(generatedJniLibsDir)
    profile.set(rustProfileProvider)
    abis.set(rustAbisProvider)
    apiLevel.set(androidApiProvider)
    androidSdkRoot.set(androidSdkRootProvider)
    androidNdkRoot.set(androidNdkRootProvider)
    ndkVersion.set(pinnedNdk)
}

pluginManager.withPlugin("com.android.application") {
    extensions.configure<ApplicationExtension>("android") {
        compileSdk = libs.findVersion("compileSdk").get().requiredVersion.toInt()

        // The NDK AGP itself uses (its llvm-strip runs over every packaged
        // .so) must resolve everywhere or AGP silently skips the strip and
        // the .so layout differs between builders. Pinned via the version
        // catalog; cargoNdk resolves the same line.
        ndkVersion = pinnedNdk

        defaultConfig {
            minSdk = libs.findVersion("minSdk").get().requiredVersion.toInt()
            targetSdk = libs.findVersion("targetSdk").get().requiredVersion.toInt()
            versionCode = cargoVersion.versionCode.get()
            versionName = cargoVersion.versionName.get()
            // AGP 9 defaults this to the AndroidX runner, but the ABI splits
            // below mean the instrumentation tests are the only check that the
            // packaged .so is the one the device can load — too load-bearing to
            // leave resting on a flag's default value.
            testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        }

        sourceSets.named("main") {
            jniLibs.directories.clear()
            jniLibs.directories.add(generatedJniLibsDir.get().asFile.absolutePath)
        }

        // One APK per ABI plus a universal one. A GitHub release cannot pick an
        // ABI for the downloader the way Play does, so the universal APK stays
        // the "just download it" artifact and the per-ABI ones are the roughly
        // one-third-size alternative for anyone who knows their device. The
        // include set comes from rustAbisProvider: a split for an ABI cargoNdk
        // never cross-compiled still builds and installs, then dies in
        // System.loadLibrary on first launch.
        splits {
            abi {
                isEnable = true
                // AGP's default include set is every ABI it knows about, and
                // include() only adds to it, so without reset() a narrowed
                // ABI selection would still demand splits for ABIs it
                // deliberately excluded.
                reset()
                include(*rustAbisProvider.get().toTypedArray())
                isUniversalApk = !gradleAbisProvider.isPresent
            }
        }

        if (signing.isConfigured.get()) {
            signingConfigs.register("release") {
                storeFile = File(signing.storeFile.get())
                storePassword = signing.storePassword.get()
                keyAlias = signing.keyAlias.get()
                keyPassword = signing.keyPassword.get()
            }
        }

        buildTypes.named("release") {
            // R8 strips the NativeProxy callbacks only if told not to; the
            // native side reaches them by class name. See proguard-rules.pro.
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
            if (signing.isConfigured.get()) {
                signingConfig = signingConfigs.getByName("release")
            }
        }

        compileOptions {
            sourceCompatibility = JavaVersion.VERSION_17
            targetCompatibility = JavaVersion.VERSION_17
        }

        buildFeatures {
            compose = true
        }
    }

    tasks.named("preBuild") {
        dependsOn(cargoNdk)
    }

    extensions.configure<ApplicationAndroidComponentsExtension>("androidComponents") {
        onVariants { variant ->
            variant.outputs.forEach { output ->
                val abiCode = output.filters
                    .find { it.filterType == FilterConfiguration.FilterType.ABI }
                    ?.identifier
                    ?.let(abiCodes::get)
                    ?: 0
                output.versionCode.set(cargoVersion.versionCode.map { it + abiCode })
            }
        }
    }
}

pluginManager.withPlugin("org.jetbrains.kotlin.plugin.compose") {
    extensions.configure<KotlinAndroidProjectExtension>("kotlin") {
        compilerOptions {
            jvmTarget.set(JvmTarget.JVM_17)
        }
    }
}
