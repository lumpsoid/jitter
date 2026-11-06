import java.util.Properties

plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

// Release signing key, outside git (android/.gitignore). A release build
// without it fails rather than ship an APK no update can follow; debug and
// profile builds don't need it. See docs/maintaining.md.
val keyProperties = Properties().apply {
    val file = rootProject.file("key.properties")
    if (file.exists()) file.inputStream().use { load(it) }
}

fun keyProperty(name: String): String =
    keyProperties.getProperty(name)
        ?: throw GradleException("android/key.properties has no $name")

android {
    namespace = "io.github.lumpsoid.jitter"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        applicationId = "io.github.lumpsoid.jitter"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        // Uses the version code from pubspec.yaml. When using split APKs, 1000 * ABI_VERSION
        // is added automatically by Flutter. (https://developer.android.com/studio/build/configure-apk-splits#configure-APK-versions)
        // You can force using the value of versionCode by specifying the `-P force-version-code-ignoring-abi=true`
        // flag during build.
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    signingConfigs {
        if (!keyProperties.isEmpty) {
            create("release") {
                // May be absolute, so the key can stay outside the repo.
                storeFile = file(keyProperty("storeFile"))
                storePassword = keyProperty("storePassword")
                keyAlias = keyProperty("keyAlias")
                keyPassword = keyProperty("keyPassword")
            }
        }
    }

    buildTypes {
        release {
            signingConfig = signingConfigs.findByName("release")
        }
    }
}

// Checked when a release build runs, not at configuration: Gradle
// configures every variant, and a debug build must not need the key.
tasks.matching { it.name == "preReleaseBuild" }.configureEach {
    doFirst {
        if (keyProperties.isEmpty) {
            throw GradleException(
                "No release signing key: create android/key.properties " +
                    "(docs/maintaining.md). Use a debug build to test.",
            )
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}
