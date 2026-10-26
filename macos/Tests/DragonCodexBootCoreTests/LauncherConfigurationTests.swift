import Foundation
import XCTest
@testable import DragonCodexBootCore

final class LauncherConfigurationTests: XCTestCase {
    func testExampleConfigurationDecodesAndValidates() throws {
        let json = #"{"DisplayName":"Dragon Codex Boot","Video":"media/startup.mp4","AppBundleIdentifier":"com.openai.codex","AppPath":"","HoldAt":11.3,"TransitionStart":12.7,"TransitionEnd":13.65,"MaxWaitSeconds":60,"Volume":1,"Fullscreen":true,"PlayerWidth":1920,"PlayerHeight":1080}"#
        let configuration = try LauncherConfiguration.decode(from: Data(json.utf8))
        XCTAssertEqual(configuration.appBundleIdentifier, "com.openai.codex")
        XCTAssertTrue(configuration.fullscreen)
        XCTAssertEqual(configuration.outroStyle, .original)
    }

    func testDarkOutroCoversWhiteFramesBeforeTextAppears() throws {
        var configuration = LauncherConfiguration(outroStyle: .darkNative)
        XCTAssertEqual(configuration.outroOpacity(at: 8), 0, accuracy: 0.0001)
        XCTAssertEqual(configuration.outroOpacity(at: 8.25), 0.5, accuracy: 0.0001)
        XCTAssertEqual(configuration.outroOpacity(at: 8.5), 1, accuracy: 0.0001)
        XCTAssertEqual(configuration.outroOpacity(at: 13.65), 1, accuracy: 0.0001)
        XCTAssertEqual(configuration.outroTextOpacity(at: 8.5), 0, accuracy: 0.0001)
        XCTAssertEqual(configuration.outroTextOpacity(at: 9), 1, accuracy: 0.0001)
        let decoded = try LauncherConfiguration.decode(from: JSONEncoder().encode(configuration))
        XCTAssertEqual(decoded, configuration)
        configuration.holdAt = 7
        XCTAssertThrowsError(try configuration.validate())
        XCTAssertEqual(LauncherConfiguration().outroOpacity(at: 12), 0)
    }

    func testPlayerFadesSmoothlyAcrossConfiguredTransition() {
        let configuration = LauncherConfiguration()
        XCTAssertEqual(configuration.playerOpacity(at: 12), 1, accuracy: 0.0001)
        XCTAssertEqual(configuration.playerOpacity(at: 12.7), 1, accuracy: 0.0001)
        XCTAssertEqual(configuration.playerOpacity(at: 13.175), 0.5, accuracy: 0.0001)
        XCTAssertEqual(configuration.playerOpacity(at: 13.65), 0, accuracy: 0.0001)
        XCTAssertEqual(configuration.playerOpacity(at: 20), 0, accuracy: 0.0001)
    }

    func testRelativeVideoPathIsResolvedAgainstApplicationSupportDirectory() {
        let root = URL(fileURLWithPath: "/tmp/DragonCodexBoot", isDirectory: true)
        XCTAssertEqual(LauncherConfiguration().videoURL(relativeTo: root).path, "/tmp/DragonCodexBoot/media/startup.mp4")
    }

    func testInvalidTimingAndMissingAppAreRejected() {
        var invalidTiming = LauncherConfiguration()
        invalidTiming.transitionEnd = invalidTiming.transitionStart
        XCTAssertThrowsError(try invalidTiming.validate())

        var missingApp = LauncherConfiguration()
        missingApp.appBundleIdentifier = ""
        XCTAssertThrowsError(try missingApp.validate())
    }
}
