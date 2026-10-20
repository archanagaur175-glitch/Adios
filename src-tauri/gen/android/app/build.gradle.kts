import java.util.Properties
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

android {
    compileSdk = 37
    namespace = "com.nullbrain.adios"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "com.nullbrain.adios"
        minSdk = 24
        targetSdk = 37
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
        // P19/D33: ship only the device ABI — the yt-dlp library's jniLibs
        // (python/quickjs) exist per-ABI and would ~4x the APK otherwise.
        ndk { abiFilters += "arm64-v8a" }
    }
    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
        }
        getByName("release") {
            // P19/D33: R8 minification is OFF. youtubedl-android does deep
            // reflection into its bundled python/jackson/commons-io deps
            // (ZipUtils.unzip -> static init chain); R8 class-merging kept
            // producing "class <obfuscated> is not a concrete class" crashes
            // on first init even with explicit keep rules (gate195). This is
            // a personal-use app — the python/quickjs .so files already
            // dominate the APK, so Java-side shrinking isn't worth the risk.
            optimization {
               enable = false
            }
            proguardFiles(
                *fileTree(".") {
                  include("**/*.pro")
                  exclude("build/**")
                }.files.toTypedArray()
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    buildFeatures {
        buildConfig = true
    }
    // P19/D33: youtubedl-android loads python/quickjs/ffmpeg .so files from
    // nativeLibraryDir (file I/O) — extraction is mandatory. AGP 9 ignores
    // the manifest's extractNativeLibs attribute; this is the authoritative
    // switch (README's requirement under the modern DSL).
    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_1_8
    }
}

rust {
    rootDirRel = "../../../"
}

dependencies {
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    // P19/D33: in-process yt-dlp (python + lazy extractors bundled).
    implementation("io.github.junkfood02.youtubedl-android:library:0.18.1")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = file("tauri.build.gradle.kts"))
