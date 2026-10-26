import Foundation

// Measure playback inactivity, not time spent waiting for the client to launch.
struct PlaybackProgressWatchdog {
    let timeout: TimeInterval
    private var lastProgressAt: TimeInterval?
    private var lastPosition: Double?

    init(timeout: TimeInterval = 25) {
        self.timeout = timeout
    }

    mutating func isStalled(position: Double, now: TimeInterval, pausedForLaunch: Bool) -> Bool {
        guard now.isFinite else { return false }
        if pausedForLaunch {
            lastProgressAt = now
            lastPosition = position.isFinite ? position : nil
            return false
        }
        if lastProgressAt == nil || (position.isFinite && (lastPosition == nil || abs(position - lastPosition!) > 0.01)) {
            lastProgressAt = now
            lastPosition = position.isFinite ? position : nil
            return false
        }
        return now - lastProgressAt! >= timeout
    }
}
