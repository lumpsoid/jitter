package dev.jitter.jitter

import android.os.Process
import com.ryanheise.audioservice.AudioServiceActivity
import io.flutter.embedding.engine.FlutterEngine

// audio_service's activity shares one Flutter engine with the media
// service, so the player keeps running while the app is in the background.
class MainActivity : AudioServiceActivity() {
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        // The engine keeps its listeners in a set, so adding this on every
        // activity attach registers it once.
        flutterEngine.addEngineLifecycleListener(EndProcessWithEngine)
    }
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
