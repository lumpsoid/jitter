package dev.jitter.jitter

import com.ryanheise.audioservice.AudioServiceActivity

// audio_service's activity shares one Flutter engine with the media
// service, so the player keeps running while the app is in the background.
class MainActivity : AudioServiceActivity()
