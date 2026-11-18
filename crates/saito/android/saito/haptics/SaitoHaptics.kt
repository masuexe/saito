package saito.haptics

import android.app.Activity
import android.view.HapticFeedbackConstants

/**
 * A light tap felt as a finger presses a button of the on-screen pad (crates/saito/src/haptics/android.rs): the
 * window's view performs a keyboard tap, posted to the activity's own thread (the app's loop doesn't wait for it). A
 * view's feedback follows the player's Touch feedback setting: off, nothing plays.
 */
object SaitoHaptics {
    @JvmStatic
    fun tap(activity: Activity) = activity.runOnUiThread {
        activity.window.decorView.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
    }
}
