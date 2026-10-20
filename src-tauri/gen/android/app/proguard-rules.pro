# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# If your project uses WebView with JS, uncomment the following
# and specify the fully qualified class name to the JavaScript interface
# class:
#-keepclassmembers class fqcn.of.javascript.interface.for.webview {
#   public *;
#}

# Uncomment this to preserve the line number information for
# debugging stack traces.
#-keepattributes SourceFile,LineNumberTable

# If you keep the line number information, uncomment this to
# hide the original source file name.
#-renamesourcefileattribute SourceFile

# P19/D33: youtubedl-android (python + quickjs + ffmpeg jniLibs) loads
# classes reflectively via its JNI bootstrap — R8 must not rename or strip
# them. Without these, the release APK crashes on first init with
# "ExceptionInInitializerError ... class <obfuscated> is not a concrete
# class" (gate195, 2026-10-20).
-keep class com.yausername.youtubedl_android.** { *; }
-keep class com.yausername.youtubedl_common.** { *; }
-keep class com.yausername.youtubedl_android.**$* { *; }
-dontwarn com.yausername.youtubedl_android.**
-dontwarn com.yausername.youtubedl_common.**
# Keep any JNI-bearing classes and their native method names.
-keepclasseswithmembernames,includedescriptorclasses class * {
    native <methods>;
}
# The library's Jackson mapper + Apache commons-io are part of its
# reflective surface too.
-keep class com.fasterxml.jackson.databind.** { *; }
-dontwarn com.fasterxml.jackson.**
-dontwarn org.apache.commons.io.**
# Library AAR already consumer-rules these; belt-and-suspenders for AGP 9.
-keep class app.tauri.plugin.** { *; }
-keep class app.tauri.annotation.** { *; }
# Line numbers for any future device crash triage.
-keepattributes SourceFile,LineNumberTable