package saito.updater

import android.app.Activity
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageInstaller
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import java.io.File

/**
 * The app's update put in place by the system's installer (crates/saito-updater/src/platform/android.rs): first tries
 * Android's [PackageInstaller] session (when unknown-apps install permission is already granted), and falls back to
 * [FileProvider] + [Intent.ACTION_VIEW] when permission is not yet granted or OEM ROMs block/abort silent session
 * installation.
 */
object SaitoUpdater {
    /** The installer's answers, to this app alone. */
    private const val ANSWER = "saito.updater.ANSWER"

    /** Handed off to the system's package-installer Activity (treat like declined/external so no false error dialog shows). */
    private const val HANDED_OFF = -100

    /** The installed app's version (the APK's versionName). */
    @JvmStatic
    fun version(activity: Activity): String {
        val info = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            activity.packageManager.getPackageInfo(activity.packageName, PackageManager.PackageInfoFlags.of(0))
        } else {
            @Suppress("DEPRECATION")
            activity.packageManager.getPackageInfo(activity.packageName, 0)
        }
        return info.versionName ?: ""
    }

    /** Launch the standard Android package installer UI via FileProvider for [file]. */
    private fun launchInstallerIntent(activity: Activity, file: File): String? =
        try {
            val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.fileprovider", file)
            val intent = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, "application/vnd.android.package-archive")
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            activity.startActivity(intent)
            null
        } catch (e: Exception) {
            e.toString()
        }

    /** Install [apk] over the app: null once the session or installer intent is launched, else why it couldn't be. */
    @JvmStatic
    fun install(activity: Activity, apk: String): String? {
        val file = File(apk)
        if (!file.isFile) {
            return "update APK missing: $apk"
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !activity.packageManager.canRequestPackageInstalls()) {
            val err = launchInstallerIntent(activity, file)
            return if (err == null) {
                ended(HANDED_OFF, "")
                null
            } else {
                err
            }
        }
        val installer = activity.packageManager.packageInstaller
        val params = PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL).apply {
            setAppPackageName(activity.packageName)
            setSize(file.length())
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_NOT_REQUIRED)
            }
        }
        var id = -1
        return try {
            id = installer.createSession(params)
            installer.openSession(id).use { session ->
                file.inputStream().use { input ->
                    session.openWrite("saito.apk", 0, file.length()).use { output ->
                        input.copyTo(output)
                        session.fsync(output)
                    }
                }
                ContextCompat.registerReceiver(activity, Answer(activity, file), IntentFilter(ANSWER), ContextCompat.RECEIVER_NOT_EXPORTED)
                // (Mutable: the installer adds its answer to it, which Android 12 and later must be told. To this app
                // alone: the receiver isn't exported.)
                val answer = Intent(ANSWER).setPackage(activity.packageName)
                val mutable = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) PendingIntent.FLAG_MUTABLE else 0
                val flags = PendingIntent.FLAG_UPDATE_CURRENT or mutable
                session.commit(PendingIntent.getBroadcast(activity, id, answer, flags).intentSender)
            }
            null
        } catch (e: Exception) {
            if (id != -1) {
                runCatching { installer.abandonSession(id) }
            }
            val fallbackErr = launchInstallerIntent(activity, file)
            if (fallbackErr == null) {
                ended(HANDED_OFF, "")
                null
            } else {
                "$e; fallback: $fallbackErr"
            }
        }
    }

    /** The installer's answer: confirmation UI, ACTION_VIEW fallback on OEM failures, or result reported to Rust. */
    private class Answer(private val activity: Activity, private val file: File) : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            val status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)
            if (status == PackageInstaller.STATUS_PENDING_USER_ACTION) {
                val asking = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    intent.getParcelableExtra(Intent.EXTRA_INTENT, Intent::class.java)
                } else {
                    @Suppress("DEPRECATION")
                    intent.getParcelableExtra(Intent.EXTRA_INTENT)
                }
                if (asking != null) {
                    asking.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                    try {
                        activity.startActivity(asking)
                        return
                    } catch (_: Exception) {
                        // Fall through to FileProvider ACTION_VIEW fallback below.
                    }
                }
            }
            runCatching { activity.unregisterReceiver(this) }
            // (Success never comes here: Android ends the app as it installs the update.)
            if (status != PackageInstaller.STATUS_SUCCESS) {
                val message = intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE) ?: ""
                if (status != PackageInstaller.STATUS_FAILURE_ABORTED) {
                    val fallbackErr = launchInstallerIntent(activity, file)
                    if (fallbackErr == null) {
                        ended(HANDED_OFF, "")
                    } else {
                        ended(status, if (message.isEmpty()) "fallback: $fallbackErr" else "$message; fallback: $fallbackErr")
                    }
                } else {
                    ended(status, message)
                }
            }
        }
    }

    /** The installer's answer where the update wasn't installed: its status, and its message. */
    @JvmStatic
    private external fun ended(status: Int, message: String)
}
