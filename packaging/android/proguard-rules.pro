# What native code calls by name through JNI, which R8 can't see: the app's Kotlin (saito-dialogs' SaitoFiles,
# the app's SaitoAwake and SaitoHaptics, saito-gamepad's SaitoGamepads, saito-updater's SaitoUpdater and their
# native methods), GameActivity's (its C calls into it), and the certificate verifier's (rustls-platform-verifier's
# Kotlin, which reqwest's checks call).
-keep class saito.** { *; }
-keep class com.google.androidgamesdk.** { *; }
-keep class org.rustls.platformverifier.** { *; }
