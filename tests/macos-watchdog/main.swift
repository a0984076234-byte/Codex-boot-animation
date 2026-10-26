import Foundation

func check(_ condition: Bool, _ message: String) {
    if !condition { fatalError(message) }
}

var advancing = PlaybackProgressWatchdog()
for second in 0...50 {
    check(!advancing.isStalled(position: Double(second), now: Double(second), pausedForLaunch: false), "Advancing playback was interrupted")
}

var waiting = PlaybackProgressWatchdog()
check(!waiting.isStalled(position: 11.3, now: 11.3, pausedForLaunch: false), "Initial progress")
for second in 12...55 {
    check(!waiting.isStalled(position: 11.3, now: Double(second), pausedForLaunch: true), "Intentional launch wait was treated as a stall")
}
check(!waiting.isStalled(position: 11.3, now: 55.2, pausedForLaunch: false), "Playback resumed after a slow launch and was skipped")
check(!waiting.isStalled(position: 11.5, now: 55.4, pausedForLaunch: false), "Resumed progress was skipped")
check(!waiting.isStalled(position: 11.5, now: 80.3, pausedForLaunch: false), "Stall budget ended too early")
check(waiting.isStalled(position: 11.5, now: 80.5, pausedForLaunch: false), "Actual stalled playback was not detected")

var unreadable = PlaybackProgressWatchdog()
check(!unreadable.isStalled(position: .nan, now: 0, pausedForLaunch: false), "Invalid position failed immediately")
check(unreadable.isStalled(position: .nan, now: 26, pausedForLaunch: false), "Invalid media position never timed out")
print("PASS: progressing playback, slow launch/resume, real stall and invalid-position watchdog checks")
