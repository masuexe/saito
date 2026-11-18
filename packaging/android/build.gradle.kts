// saito's APK, which package.sh builds after cargo has built the native library and the rest it hands over, each
// a Gradle property:
//
//   saito.out          target/package/android: jniLibs/arm64-v8a/ (the library and libc++_shared.so), assets/ (the
//                       notices, the license), res/ (the launcher icon); this build's own files go in its gradle/
//   saito.version      the version, and saito.versionCode its number (0.17.0: 17000)
//   saito.verifier     rustls-platform-verifier's AAR (its Kotlin, which the certificate checks call through JNI,
//                       and its network security config), pinned by package.sh
//   saito.keystore     the keystore to sign with, saito.keystorePassword its password and saito.keyAlias the key's
//   saito.debuggable   1: the APK says it is debuggable (`adb shell run-as` reads and writes its files)
//
// The Kotlin is every crate's android/ folder's (the app's SaitoActivity, SaitoAwake and SaitoHaptics,
// saito-dialogs' SaitoFiles, saito-gamepad's SaitoGamepads), compiled by AGP's own Kotlin; R8 shrinks it with the
// libraries' and takes out what nothing uses (the Kotlin standard library's most of all), keeping what the native code
// calls by name (proguard-rules.pro).
plugins {
    id("com.android.application") version "9.5.0"
}

fun setting(name: String): String = providers.gradleProperty(name).orNull ?: error("package.sh gives -P$name")

val out = file(setting("saito.out"))
val crates = rootDir.resolve("../../crates").canonicalFile

layout.buildDirectory.set(out.resolve("gradle"))

android {
    namespace = "top.exessrhub.saito"
    compileSdk = 36

    defaultConfig {
        applicationId = "top.exessrhub.saito"
        // Android 8 (the oldest whose AAudio cpal plays through).
        minSdk = 26
        targetSdk = 34
        versionName = setting("saito.version")
        versionCode = setting("saito.versionCode").toInt()
    }

    sourceSets["main"].apply {
        manifest.srcFile("AndroidManifest.xml")
        kotlin.srcDirs(crates.listFiles()!!.map { it.resolve("android") }.filter { it.isDirectory }.sorted())
        jniLibs.srcDir(out.resolve("jniLibs"))
        assets.srcDir(out.resolve("assets"))
        res.srcDir(out.resolve("res"))
    }

    signingConfigs {
        create("saito") {
            storeFile = file(setting("saito.keystore"))
            storePassword = setting("saito.keystorePassword")
            keyAlias = setting("saito.keyAlias")
            keyPassword = setting("saito.keystorePassword")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            // (And the resources nothing uses: most of AppCompat's.)
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs["saito"]
            isDebuggable = providers.gradleProperty("saito.debuggable").orNull == "1"
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // (The library is stripped already, by package.sh: no NDK needed here.)
    packaging {
        jniLibs {
            keepDebugSymbols += "**/*.so"
        }
    }

    lint {
        checkReleaseBuilds = false
    }
}

dependencies {
    // The activity (android-activity 0.6's C is GameActivity 4.4.0's, exactly) and what it is built on.
    implementation("androidx.games:games-activity:4.4.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation(files(setting("saito.verifier")))
}
