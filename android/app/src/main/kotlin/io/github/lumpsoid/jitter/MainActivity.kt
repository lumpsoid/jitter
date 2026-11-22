package io.github.lumpsoid.jitter

import android.os.Build
import android.os.Environment
import android.os.Process
import com.ryanheise.audioservice.AudioServiceActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

// audio_service's activity shares one Flutter engine with the media
// service, so the player keeps running while the app is in the background.
class MainActivity : AudioServiceActivity() {
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        // The engine keeps its listeners in a set, so adding this on every
        // activity attach registers it once.
        flutterEngine.addEngineLifecycleListener(EndProcessWithEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "jitter/storage")
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "sharedDownloadsDir" -> result.success(sharedDownloadsDir())
                    else -> result.notImplemented()
                }
            }
    }
}

// The shared Download folder, where the user reaches downloads from any
// file manager. From Android 11 an app may create files of any type there
// by path without a storage permission (Movies/ takes only video types,
// and a download's .part and .part.resume files are not). Older versions
// need the permission, so they get null and keep the app's own folder.
@Suppress("DEPRECATION")
private fun sharedDownloadsDir(): String? =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
        Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS).path
    } else {
        null
    }

// audio_service destroys the engine once no activity or service holds it,
// but Android keeps the process, and the next launch starts a new engine
// in it. libmpv outlives the old engine's Dart isolate: its player stays
// alive with a wakeup callback into the dead isolate, and since mpv routes
// every FFmpeg log line to the first player created, the new player's
// first error log calls that callback and crashes the process (SIGSEGV).
// With nothing on screen and nothing playing, ending the process is free
// and lets the next launch start clean.
private object EndProcessWithEngine : FlutterEngine.EngineLifecycleListener {
    override fun onPreEngineRestart() {}

    override fun onEngineWillDestroy() = Process.killProcess(Process.myPid())
}
