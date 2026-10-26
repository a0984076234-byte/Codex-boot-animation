import AppKit
import AVKit
import CoreGraphics
import CoreImage
import Darwin
import Foundation
#if canImport(DragonCodexBootCore)
import DragonCodexBootCore
#endif

@MainActor
private func activateLauncher() {
    if #available(macOS 14.0, *) {
        NSApplication.shared.activate()
    } else {
        NSApplication.shared.activate(ignoringOtherApps: true)
    }
}

private final class BootWindow: NSWindow {
    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { true }
}

private struct PreviewOptions {
    let enabled: Bool
    let screenIndex: Int?
    let stillTime: Double?
    let readyAfter: Double
    let timeout: Double?
    let moveAfter: Double?
    let windowed: Bool

    init(arguments: [String] = CommandLine.arguments) throws {
        let isEnabled = arguments.contains("--preview")
        enabled = isEnabled
        func value(_ flag: String) throws -> String? {
            guard isEnabled, let index = arguments.firstIndex(of: flag) else { return nil }
            guard arguments.indices.contains(index + 1) else {
                throw LauncherConfigurationError("\(flag) 缺少参数。")
            }
            return arguments[index + 1]
        }
        func seconds(_ flag: String) throws -> Double? {
            guard let raw = try value(flag) else { return nil }
            guard let result = Double(raw), result.isFinite, result >= 0 else {
                throw LauncherConfigurationError("\(flag) 必须为非负秒数。")
            }
            return result
        }
        if let raw = try value("--preview-screen") {
            guard let index = Int(raw), NSScreen.screens.indices.contains(index) else {
                throw LauncherConfigurationError("--preview-screen 超出可用显示器范围。")
            }
            screenIndex = index
        } else {
            screenIndex = nil
        }
        stillTime = try seconds("--preview-at")
        readyAfter = try seconds("--preview-ready-after") ?? 0
        timeout = try seconds("--preview-timeout")
        moveAfter = try seconds("--preview-move-after")
        windowed = enabled && arguments.contains("--preview-windowed")
        if let stillTime, stillTime >= 13.65 {
            throw LauncherConfigurationError("--preview-at 必须小于 13.65 秒。")
        }
    }
}

@MainActor
private final class DarkOutroView: NSView {
    private let stack = NSStackView()

    init(frame: NSRect, appURL: URL) {
        super.init(frame: frame)
        wantsLayer = true
        layer?.backgroundColor = NSColor(srgbRed: 24 / 255, green: 24 / 255, blue: 24 / 255, alpha: 1).cgColor
        appearance = NSAppearance(named: .darkAqua)
        let icon = NSImageView()
        icon.image = NSWorkspace.shared.icon(forFile: appURL.path)
        icon.imageScaling = .scaleProportionallyDown
        icon.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            icon.widthAnchor.constraint(equalToConstant: 64),
            icon.heightAnchor.constraint(equalToConstant: 64),
        ])
        stack.orientation = .vertical
        stack.alignment = .centerX
        stack.spacing = 16
        stack.translatesAutoresizingMaskIntoConstraints = false
        addSubview(stack)
        stack.addArrangedSubview(icon)
        for (text, size, weight, color) in [
            ("Codex", CGFloat(36), NSFont.Weight.semibold, NSColor.white),
            ("开始构建", CGFloat(22), NSFont.Weight.medium, NSColor.white),
            ("让想法成为现实。", CGFloat(16), NSFont.Weight.regular, NSColor(white: 0.72, alpha: 1)),
        ] {
            let label = NSTextField(labelWithString: text)
            label.font = .systemFont(ofSize: size, weight: weight)
            label.textColor = color
            label.alignment = .center
            stack.addArrangedSubview(label)
        }
        NSLayoutConstraint.activate([
            stack.centerXAnchor.constraint(equalTo: centerXAnchor),
            stack.centerYAnchor.constraint(equalTo: centerYAnchor),
            stack.widthAnchor.constraint(lessThanOrEqualTo: widthAnchor, constant: -48),
        ])
        stack.alphaValue = 0
    }

    required init?(coder: NSCoder) { nil }

    func update(textOpacity: Double) {
        stack.alphaValue = textOpacity
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        updateBackingScale()
    }

    override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        updateBackingScale()
    }

    private func updateBackingScale() {
        let scale = window?.backingScaleFactor ?? 1
        func update(_ view: NSView) {
            view.layer?.contentsScale = scale
            for child in view.subviews { update(child) }
            view.needsDisplay = true
        }
        update(self)
    }
}

private enum LauncherError: LocalizedError {
    case missingConfig
    case missingVideo(URL)
    case missingApp(String)

    var errorDescription: String? {
        switch self {
        case .missingConfig:
            return "找不到 macOS 配置文件。请重新运行构建脚本。"
        case .missingVideo(let url):
            return "找不到启动视频：\n\(url.path)\n\n请把 MP4 放到该位置，或修改 launcher.json 中的 Video。"
        case .missingApp(let detail):
            return "找不到 Codex 桌面 App。\n\(detail)\n\n请在 launcher.json 中填写正确的 AppBundleIdentifier 或 AppPath。"
        }
    }
}

private struct RuntimeConfiguration {
    let configuration: LauncherConfiguration
    let root: URL

    static func load() throws -> RuntimeConfiguration {
        let fileManager = FileManager.default
        if let index = CommandLine.arguments.firstIndex(of: "--config") {
            guard CommandLine.arguments.contains("--preview"), index + 1 < CommandLine.arguments.count else {
                throw LauncherConfigurationError("--config 只允许用于 --preview，且必须提供文件路径。")
            }
            let url = URL(fileURLWithPath: CommandLine.arguments[index + 1])
            let configuration = try LauncherConfiguration.decode(from: Data(contentsOf: url))
            return RuntimeConfiguration(configuration: configuration, root: url.deletingLastPathComponent())
        }
        let supportRoot = try fileManager.url(
            for: .applicationSupportDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: true
        ).appendingPathComponent("DragonCodexBoot", isDirectory: true)
        try fileManager.createDirectory(at: supportRoot, withIntermediateDirectories: true)

        let configURL = supportRoot.appendingPathComponent("launcher.json")
        if !fileManager.fileExists(atPath: configURL.path) {
            guard let template = Bundle.main.resourceURL?.appendingPathComponent("launcher.macos.example.json"),
                  fileManager.fileExists(atPath: template.path) else {
                throw LauncherError.missingConfig
            }
            try fileManager.copyItem(at: template, to: configURL)
        }

        let data = try Data(contentsOf: configURL)
        let configuration = try LauncherConfiguration.decode(from: data)
        let runtime = RuntimeConfiguration(configuration: configuration, root: supportRoot)
        runtime.copyBundledVideoIfNeeded()
        return runtime
    }

    private func copyBundledVideoIfNeeded() {
        let target = configuration.videoURL(relativeTo: root)
        guard !FileManager.default.fileExists(atPath: target.path),
              let resourceRoot = Bundle.main.resourceURL else { return }
        let relativeVideo = (configuration.video as NSString).expandingTildeInPath
        guard !relativeVideo.hasPrefix("/") else { return }
        let bundled = resourceRoot.appendingPathComponent(relativeVideo).standardizedFileURL
        guard FileManager.default.fileExists(atPath: bundled.path) else { return }
        do {
            try FileManager.default.createDirectory(
                at: target.deletingLastPathComponent(),
                withIntermediateDirectories: true
            )
            try FileManager.default.copyItem(at: bundled, to: target)
        } catch {
            NSLog("Could not copy bundled startup video: %@", error.localizedDescription)
        }
    }
}

private enum LaunchAgentIntegration {
    static let label = "community.dragoncodexboot.codex-watcher"
    static let plistName = "\(label).plist"

    static func propertyList(executablePath: String) -> [String: Any] {
        [
            "Label": label,
            "ProgramArguments": [executablePath, "--watch"],
            "RunAtLoad": true,
            "LimitLoadToSessionType": "Aqua",
        ]
    }

    static func install(executablePath: String) throws -> URL {
        let libraryDirectory = try FileManager.default.url(
            for: .libraryDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: true
        )
        let launchAgentsDirectory = libraryDirectory.appendingPathComponent("LaunchAgents", isDirectory: true)
        try FileManager.default.createDirectory(at: launchAgentsDirectory, withIntermediateDirectories: true)
        let plistURL = launchAgentsDirectory.appendingPathComponent(plistName)
        let data = try PropertyListSerialization.data(
            fromPropertyList: propertyList(executablePath: executablePath),
            format: .xml,
            options: 0
        )
        try data.write(to: plistURL, options: .atomic)
        return plistURL
    }

    static func uninstall() throws -> URL {
        let libraryDirectory = try FileManager.default.url(
            for: .libraryDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: false
        )
        let plistURL = libraryDirectory
            .appendingPathComponent("LaunchAgents", isDirectory: true)
            .appendingPathComponent(plistName)
        if FileManager.default.fileExists(atPath: plistURL.path) {
            try FileManager.default.removeItem(at: plistURL)
        }
        return plistURL
    }
}

@MainActor
private final class BootController: NSObject, NSWindowDelegate {
    private let configuration: LauncherConfiguration
    private let videoURL: URL
    private let appURL: URL
    private let targetBundleIdentifier: String
    private var targetApplication: NSRunningApplication?
    private var window: NSWindow?
    private var player: AVPlayer?
    private var timer: Timer?
    private var endObserver: NSObjectProtocol?
    private var statusObservation: NSKeyValueObservation?
    private var keyMonitor: Any?
    private var statusLabel: NSTextField?
    private var videoView: AVPlayerView?
    private var outroView: DarkOutroView?
    private var handedOff = false
    private var waitingAtHold = false
    private var startedAt = Date()
    private let watchMode: Bool
    private let onFinish: (@MainActor () -> Void)?
    private let preview: PreviewOptions
    private var movedPreview = false
    private var lastPhase = ""
    private var loggedFadeMidpoint = false

    init(
        runtime: RuntimeConfiguration,
        targetApplication: NSRunningApplication? = nil,
        watchMode: Bool = false,
        onFinish: (@MainActor () -> Void)? = nil
    ) throws {
        preview = try PreviewOptions()
        var selectedConfiguration = runtime.configuration
        if preview.enabled {
            if CommandLine.arguments.contains("--preview-dark") { selectedConfiguration.outroStyle = .darkNative }
            if CommandLine.arguments.contains("--preview-original") { selectedConfiguration.outroStyle = .original }
            if !CommandLine.arguments.contains("--preview-sound") { selectedConfiguration.volume = 0 }
            try selectedConfiguration.validate()
        }
        configuration = selectedConfiguration
        videoURL = runtime.configuration.videoURL(relativeTo: runtime.root)
        let resolved: (url: URL, bundleIdentifier: String)
        if preview.enabled {
            // Preview must remain usable on a machine without the Codex app.
            resolved = (
                URL(fileURLWithPath: "/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericApplicationIcon.icns"),
                ""
            )
        } else {
            resolved = try Self.resolveApplication(for: runtime.configuration)
        }
        appURL = resolved.url
        targetBundleIdentifier = resolved.bundleIdentifier
        self.targetApplication = targetApplication
        self.watchMode = watchMode
        self.onFinish = onFinish
        super.init()
        guard FileManager.default.fileExists(atPath: videoURL.path) else {
            throw LauncherError.missingVideo(videoURL)
        }
    }

    func start(matching targetFrame: CGRect? = nil) {
        let app = NSApplication.shared
        app.setActivationPolicy(watchMode ? .accessory : .regular)
        if !watchMode, !preview.enabled { targetApplication = runningTargetApplication() }

        let screen = preview.screenIndex.map { NSScreen.screens[$0] } ?? NSScreen.main ?? NSScreen.screens.first
        let visibleFrame = screen?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1280, height: 800)
        let frame: NSRect
        if let targetFrame {
            frame = targetFrame
        } else if configuration.fullscreen, !preview.windowed, let screenFrame = screen?.frame {
            frame = screenFrame
        } else {
            frame = NSRect(
                x: visibleFrame.midX - CGFloat(preview.windowed ? 960 : configuration.playerWidth) / 2,
                y: visibleFrame.midY - CGFloat(preview.windowed ? 620 : configuration.playerHeight) / 2,
                width: CGFloat(preview.windowed ? 960 : configuration.playerWidth),
                height: CGFloat(preview.windowed ? 620 : configuration.playerHeight)
            )
        }

        let playerWindow = BootWindow(
            contentRect: frame,
            styleMask: [.borderless],
            backing: .buffered,
            defer: false
        )
        let matchesCodexWindow = targetFrame != nil
        playerWindow.title = configuration.displayName
        playerWindow.backgroundColor = matchesCodexWindow ? .clear : .black
        playerWindow.isOpaque = !matchesCodexWindow
        playerWindow.hasShadow = false
        playerWindow.level = preview.windowed ? .floating : .screenSaver
        playerWindow.isMovableByWindowBackground = preview.windowed
        playerWindow.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        playerWindow.isReleasedWhenClosed = false
        playerWindow.delegate = self
        window = playerWindow

        let rootView = NSView(frame: NSRect(origin: .zero, size: frame.size))
        rootView.wantsLayer = true
        rootView.appearance = NSAppearance(named: .darkAqua)
        rootView.layer?.backgroundColor = matchesCodexWindow ? NSColor.clear.cgColor : NSColor.black.cgColor
        if matchesCodexWindow {
            rootView.layer?.cornerRadius = 12
            rootView.layer?.cornerCurve = .continuous
            rootView.layer?.masksToBounds = true
        }

        let videoView = AVPlayerView(frame: rootView.bounds)
        videoView.translatesAutoresizingMaskIntoConstraints = false
        videoView.controlsStyle = .none
        videoView.videoGravity = .resizeAspect
        self.videoView = videoView
        rootView.addSubview(videoView)
        NSLayoutConstraint.activate([
            videoView.leadingAnchor.constraint(equalTo: rootView.leadingAnchor),
            videoView.trailingAnchor.constraint(equalTo: rootView.trailingAnchor),
            videoView.topAnchor.constraint(equalTo: rootView.topAnchor),
            videoView.bottomAnchor.constraint(equalTo: rootView.bottomAnchor),
        ])

        if configuration.outroStyle == .darkNative {
            let outro = DarkOutroView(frame: rootView.bounds, appURL: appURL)
            outro.translatesAutoresizingMaskIntoConstraints = false
            outro.alphaValue = 0
            rootView.addSubview(outro)
            NSLayoutConstraint.activate([
                outro.leadingAnchor.constraint(equalTo: rootView.leadingAnchor),
                outro.trailingAnchor.constraint(equalTo: rootView.trailingAnchor),
                outro.topAnchor.constraint(equalTo: rootView.topAnchor),
                outro.bottomAnchor.constraint(equalTo: rootView.bottomAnchor),
            ])
            outroView = outro
        }

        let skipButton = NSButton(title: "跳过  Esc", target: self, action: #selector(skip))
        skipButton.isBordered = false
        skipButton.wantsLayer = true
        skipButton.layer?.backgroundColor = NSColor(white: 0.14, alpha: 1).cgColor
        skipButton.layer?.cornerRadius = 6
        skipButton.contentTintColor = .white
        skipButton.font = .systemFont(ofSize: 14, weight: .medium)
        skipButton.alphaValue = 0.92
        skipButton.translatesAutoresizingMaskIntoConstraints = false
        rootView.addSubview(skipButton)
        NSLayoutConstraint.activate([
            skipButton.widthAnchor.constraint(equalToConstant: 96),
            skipButton.heightAnchor.constraint(equalToConstant: 32),
            skipButton.trailingAnchor.constraint(equalTo: rootView.trailingAnchor, constant: -20),
            skipButton.topAnchor.constraint(equalTo: rootView.topAnchor, constant: 20),
        ])

        let label = NSTextField(labelWithString: "Codex 正在启动…  Esc 跳过")
        label.textColor = .white
        label.font = .systemFont(ofSize: 16, weight: .medium)
        label.alignment = .center
        label.isHidden = true
        label.translatesAutoresizingMaskIntoConstraints = false
        rootView.addSubview(label)
        NSLayoutConstraint.activate([
            label.centerXAnchor.constraint(equalTo: rootView.centerXAnchor),
            label.bottomAnchor.constraint(equalTo: rootView.bottomAnchor, constant: -36),
        ])
        statusLabel = label

        playerWindow.contentView = rootView
        playerWindow.makeKeyAndOrderFront(nil)
        activateLauncher()
        installEscapeMonitor()
        if preview.enabled {
            NSLog("Preview only; target application will not be opened or activated")
        } else if let targetApplication {
            revealTargetBehindOverlay(targetApplication)
        } else {
            beginOpeningTarget()
        }
        beginPlayback(in: videoView)
        startedAt = Date()
        let playbackTimer = Timer.scheduledTimer(withTimeInterval: 1.0 / 30.0, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in self?.tick() }
        }
        timer = playbackTimer
        RunLoop.main.add(playbackTimer, forMode: .common)
        NSLog("Dragon Codex Boot started; video=%@ style=%@ preview=%d screen=%@ scale=%.1f",
              videoURL.path, configuration.outroStyle.rawValue, preview.enabled ? 1 : 0,
              screen?.localizedName ?? "unknown", playerWindow.backingScaleFactor)
    }

    private func beginPlayback(in view: AVPlayerView) {
        let item = AVPlayerItem(url: videoURL)
        if configuration.outroStyle == .darkNative {
            item.videoComposition = AVVideoComposition(asset: item.asset, applyingCIFiltersWithHandler: { request in
                let source = request.sourceImage
                let sharpened = source.clampedToExtent()
                    .applyingFilter("CISharpenLuminance", parameters: [kCIInputSharpnessKey: 0.22])
                    .cropped(to: source.extent)
                request.finish(with: sharpened, context: nil)
            })
        }
        let mediaPlayer = AVPlayer(playerItem: item)
        mediaPlayer.volume = Float(configuration.volume)
        view.player = mediaPlayer
        player = mediaPlayer

        endObserver = NotificationCenter.default.addObserver(
            forName: AVPlayerItem.didPlayToEndTimeNotification,
            object: item,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor [weak self] in self?.handOff() }
        }
        statusObservation = item.observe(\.status, options: [.initial, .new]) { [weak self] item, _ in
            guard item.status == .failed else { return }
            let error = item.error
            Task { @MainActor [weak self] in self?.showPlaybackFailure(error) }
        }
        if let time = preview.stillTime {
            mediaPlayer.seek(to: CMTime(seconds: time, preferredTimescale: 600), toleranceBefore: .zero, toleranceAfter: .zero)
        } else {
            mediaPlayer.play()
        }
    }

    private func beginOpeningTarget() {
        if let running = targetApplication, !running.isTerminated {
            revealTargetBehindOverlay(running)
            return
        }
        let openConfiguration = NSWorkspace.OpenConfiguration()
        openConfiguration.activates = true
        NSWorkspace.shared.openApplication(at: appURL, configuration: openConfiguration) { [weak self] app, error in
            Task { @MainActor in
                guard let self else { return }
                if let error {
                    self.showPlaybackFailure(error)
                    return
                }
                self.targetApplication = app
                if let app { self.revealTargetBehindOverlay(app) }
            }
        }
    }

    private func revealTargetBehindOverlay(_ app: NSRunningApplication) {
        targetApplication = app
        app.unhide()
        _ = app.activate(options: [.activateAllWindows])
        // Keep the animation in front and keep Esc available while Codex opens underneath it.
        window?.makeKeyAndOrderFront(nil)
        activateLauncher()
    }

    private func runningTargetApplication() -> NSRunningApplication? {
        NSWorkspace.shared.runningApplications.first {
            $0.bundleIdentifier == targetBundleIdentifier && !$0.isTerminated
        }
    }

    private func targetIsReady() -> Bool {
        if preview.enabled { return Date().timeIntervalSince(startedAt) >= preview.readyAfter }
        guard let app = targetApplication, !app.isTerminated, app.isFinishedLaunching else {
            targetApplication = runningTargetApplication()
            return targetApplication?.isFinishedLaunching == true
        }
        return true
    }

    private func tick() {
        guard !handedOff, let player, let window else { return }
        if watchMode, targetApplication?.isTerminated != false {
            handOff()
            return
        }
        let elapsed = Date().timeIntervalSince(startedAt)
        let timeout = preview.enabled ? (preview.timeout ?? (preview.stillTime == nil ? configuration.maxWaitSeconds + 20 : 30)) : configuration.maxWaitSeconds + 20
        if elapsed > timeout {
            NSLog("Playback timeout after %.2f seconds", elapsed)
            handOff()
            return
        }
        if preview.enabled, !movedPreview, let moveAfter = preview.moveAfter, elapsed >= moveAfter,
           let screen = window.screen, let index = NSScreen.screens.firstIndex(of: screen), NSScreen.screens.count > 1 {
            movedPreview = true
            let destination = NSScreen.screens[(index + 1) % NSScreen.screens.count]
            window.setFrame(destination.frame, display: true)
            NSLog("Preview moved to %@; scale=%.1f", destination.localizedName, window.backingScaleFactor)
        }

        let time = player.currentTime().seconds
        guard time.isFinite else { return }
        let outroOpacity = configuration.outroOpacity(at: time)
        outroView?.alphaValue = outroOpacity
        outroView?.update(textOpacity: configuration.outroTextOpacity(at: time))
        // Stop compositing the original white frames once the native outro covers them.
        videoView?.isHidden = outroOpacity >= 1
        let ready = targetIsReady()
        if !ready, !waitingAtHold, time >= configuration.holdAt {
            waitingAtHold = true
            player.pause()
            statusLabel?.isHidden = false
        } else if ready, waitingAtHold {
            waitingAtHold = false
            statusLabel?.isHidden = true
            if preview.stillTime == nil {
                // The player is already paused at the hold point; a tolerant seek can skip the outro.
                player.play()
            }
        }

        let phase = waitingAtHold ? "waiting" : (outroOpacity >= 1 ? "native-outro" : "video")
        if phase != lastPhase {
            lastPhase = phase
            NSLog("Playback phase=%@ media=%.3f outro=%.2f scale=%.1f", phase, time, outroOpacity, window.backingScaleFactor)
        }
        if ready, time >= configuration.transitionStart {
            window.alphaValue = configuration.playerOpacity(at: time)
            player.volume = Float(configuration.volume * window.alphaValue)
            if !loggedFadeMidpoint, window.alphaValue <= 0.5 {
                loggedFadeMidpoint = true
                NSLog("Playback fade; media=%.3f opacity=%.3f volume=%.3f", time, window.alphaValue, player.volume)
            }
            if time >= configuration.transitionEnd {
                handOff()
            }
        }
    }

    @objc private func skip() {
        NSLog("Playback skipped")
        handOff()
    }

    private func installEscapeMonitor() {
        keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard event.keyCode == 53 else { return event }
            Task { @MainActor in self?.handOff() }
            return nil
        }
    }

    private func showPlaybackFailure(_ error: Error?) {
        guard !handedOff else { return }
        player?.pause()
        // A normal modal alert must not sit underneath the screen-saver-level video.
        window?.orderOut(nil)
        activateLauncher()
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = "启动动画或 Codex 启动失败"
        alert.informativeText = error?.localizedDescription ?? "无法播放启动视频。按确定后将尝试切换到 Codex。"
        alert.addButton(withTitle: "切换到 Codex")
        alert.runModal()
        handOff()
    }

    private func handOff() {
        guard !handedOff else { return }
        handedOff = true
        NSLog("Playback finished; media=%.3f preview=%d", player?.currentTime().seconds ?? 0, preview.enabled ? 1 : 0)
        timer?.invalidate()
        timer = nil
        player?.pause()
        if let endObserver { NotificationCenter.default.removeObserver(endObserver) }
        if let keyMonitor { NSEvent.removeMonitor(keyMonitor) }
        endObserver = nil
        statusObservation?.invalidate()
        statusObservation = nil
        keyMonitor = nil

        window?.orderOut(nil)
        window?.close()
        window = nil

        if preview.enabled {
            NSApplication.shared.terminate(nil)
            return
        }

        if watchMode {
            if let app = targetApplication, !app.isTerminated {
                app.unhide()
                _ = app.activate(options: [.activateAllWindows])
            }
            onFinish?()
            return
        }

        if let app = targetApplication, !app.isTerminated {
            app.unhide()
            _ = app.activate(options: [.activateAllWindows])
            NSApplication.shared.terminate(nil)
        } else {
            let openConfiguration = NSWorkspace.OpenConfiguration()
            openConfiguration.activates = true
            NSWorkspace.shared.openApplication(at: appURL, configuration: openConfiguration) { _, error in
                if let error { NSLog("Codex handoff launch failed: %@", error.localizedDescription) }
                NSApplication.shared.terminate(nil)
            }
        }
    }

    func windowWillClose(_ notification: Notification) {
        if !handedOff { handOff() }
    }

    fileprivate static func resolveApplication(for configuration: LauncherConfiguration) throws -> (url: URL, bundleIdentifier: String) {
        let fileManager = FileManager.default
        let bundleIdentifier = configuration.appBundleIdentifier.trimmingCharacters(in: .whitespacesAndNewlines)
        let rawPath = configuration.appPath.trimmingCharacters(in: .whitespacesAndNewlines)
        if !rawPath.isEmpty {
            let expanded = (rawPath as NSString).expandingTildeInPath
            let url = URL(fileURLWithPath: expanded, isDirectory: true).standardizedFileURL
            guard fileManager.fileExists(atPath: url.path) else {
                throw LauncherError.missingApp("AppPath 不存在：\(url.path)")
            }
            let discoveredBundleID = Bundle(url: url)?.bundleIdentifier ?? bundleIdentifier
            guard !discoveredBundleID.isEmpty else {
                throw LauncherError.missingApp("无法从 AppPath 读取 Bundle Identifier。")
            }
            return (url, discoveredBundleID)
        }

        if let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleIdentifier),
           fileManager.fileExists(atPath: url.path) {
            return (url, bundleIdentifier)
        }
        throw LauncherError.missingApp("未通过 Bundle Identifier 找到应用：\(bundleIdentifier)")
    }
}

private enum CodexWindowGeometry {
    static func frame(for processIdentifier: pid_t) -> CGRect? {
        guard let windowList = CGWindowListCopyWindowInfo(
            [.optionOnScreenOnly, .excludeDesktopElements],
            kCGNullWindowID
        ) as? [[String: Any]] else {
            return nil
        }

        let candidates: [(frame: CGRect, area: CGFloat)] = windowList.compactMap { window in
            guard let owner = window[kCGWindowOwnerPID as String] as? NSNumber,
                  owner.int32Value == processIdentifier,
                  (window[kCGWindowLayer as String] as? NSNumber)?.intValue == 0,
                  ((window[kCGWindowAlpha as String] as? NSNumber)?.doubleValue ?? 1) > 0.05,
                  let bounds = window[kCGWindowBounds as String] as? NSDictionary else {
                return nil
            }

            var frame = CGRect.zero
            guard CGRectMakeWithDictionaryRepresentation(bounds as CFDictionary, &frame),
                  frame.width >= 350,
                  frame.height >= 250 else {
                return nil
            }
            return (frame, frame.width * frame.height)
        }

        guard let quartzFrame = candidates.max(by: { $0.area < $1.area })?.frame else {
            return nil
        }

        // CGWindow bounds use a top-left origin; NSWindow frames use a bottom-left origin.
        let mainDisplayHeight = CGDisplayBounds(CGMainDisplayID()).height
        return CGRect(
            x: quartzFrame.minX,
            y: mainDisplayHeight - quartzFrame.maxY,
            width: quartzFrame.width,
            height: quartzFrame.height
        )
    }
}

@MainActor
private final class CodexLaunchMonitor {
    private let runtime: RuntimeConfiguration
    private let targetBundleIdentifier: String
    private var launchObserver: NSObjectProtocol?
    private var windowPollTimer: Timer?
    private var pendingApplication: NSRunningApplication?
    private var pendingSince = Date()
    private var lastWindowFrame: CGRect?
    private var stableFrameCount = 0
    private var activeController: BootController?

    init(runtime: RuntimeConfiguration) throws {
        self.runtime = runtime
        targetBundleIdentifier = try BootController.resolveApplication(for: runtime.configuration).bundleIdentifier
    }

    func start() {
        let workspace = NSWorkspace.shared
        launchObserver = workspace.notificationCenter.addObserver(
            forName: NSWorkspace.didLaunchApplicationNotification,
            object: workspace,
            queue: .main
        ) { [weak self] notification in
            guard let application = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication else {
                return
            }
            Task { @MainActor [weak self] in self?.applicationDidLaunch(application) }
        }
        NSLog("Dragon Codex Boot is watching for %@ launches", targetBundleIdentifier)
    }

    private func applicationDidLaunch(_ application: NSRunningApplication) {
        guard application.bundleIdentifier == targetBundleIdentifier,
              activeController == nil else {
            return
        }

        pendingApplication = application
        pendingSince = Date()
        lastWindowFrame = nil
        stableFrameCount = 0
        windowPollTimer?.invalidate()
        let timer = Timer.scheduledTimer(withTimeInterval: 0.2, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in self?.pollForWindow() }
        }
        windowPollTimer = timer
        RunLoop.main.add(timer, forMode: .common)
        pollForWindow()
    }

    private func pollForWindow() {
        guard let application = pendingApplication, !application.isTerminated else {
            stopWaitingForWindow()
            return
        }

        if let frame = CodexWindowGeometry.frame(for: application.processIdentifier) {
            if frame == lastWindowFrame {
                stableFrameCount += 1
            } else {
                lastWindowFrame = frame
                stableFrameCount = 1
            }
            guard stableFrameCount >= 2 else { return }

            stopWaitingForWindow()
            do {
                let controller = try BootController(
                    runtime: runtime,
                    targetApplication: application,
                    watchMode: true,
                    onFinish: { [weak self] in self?.activeController = nil }
                )
                activeController = controller
                controller.start(matching: frame)
            } catch {
                NSLog("Could not start Codex launch animation: %@", error.localizedDescription)
            }
            return
        }

        if Date().timeIntervalSince(pendingSince) > 30 {
            NSLog("Codex launched without a visible main window; skipping startup animation")
            stopWaitingForWindow()
        }
    }

    private func stopWaitingForWindow() {
        windowPollTimer?.invalidate()
        windowPollTimer = nil
        pendingApplication = nil
        lastWindowFrame = nil
        stableFrameCount = 0
    }
}

@MainActor
private final class AppDelegate: NSObject, NSApplicationDelegate {
    private var controller: BootController?
    private var launchMonitor: CodexLaunchMonitor?
    private let watchMode = CommandLine.arguments.contains("--watch")

    func applicationDidFinishLaunching(_ notification: Notification) {
        if watchMode {
            NSApplication.shared.setActivationPolicy(.accessory)
            do {
                let runtime = try RuntimeConfiguration.load()
                launchMonitor = try CodexLaunchMonitor(runtime: runtime)
                launchMonitor?.start()
            } catch {
                NSLog("Could not start Codex launch monitor: %@", error.localizedDescription)
                NSApplication.shared.terminate(nil)
            }
            return
        }

        do {
            let runtime = try RuntimeConfiguration.load()
            controller = try BootController(runtime: runtime)
            controller?.start()
        } catch {
            let alert = NSAlert(error: error)
            alert.runModal()
            NSApplication.shared.terminate(nil)
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { !watchMode }
}

private func exampleConfigurationURL() -> URL? {
    if let resource = Bundle.main.resourceURL?.appendingPathComponent("launcher.macos.example.json"),
       FileManager.default.fileExists(atPath: resource.path) {
        return resource
    }
    let current = URL(fileURLWithPath: FileManager.default.currentDirectoryPath, isDirectory: true)
    let candidates = [
        current.appendingPathComponent("config/launcher.macos.example.json"),
        current.appendingPathComponent("../config/launcher.macos.example.json"),
    ]
    return candidates.first { FileManager.default.fileExists(atPath: $0.path) }
}

@main
@MainActor
private struct DragonCodexBootMacApp {
    static func main() {
        if CommandLine.arguments.contains("--install-integration") {
            do {
                let executablePath = URL(fileURLWithPath: CommandLine.arguments[0]).standardizedFileURL.path
                let plistURL = try LaunchAgentIntegration.install(executablePath: executablePath)
                print("Codex launch monitor manifest written: \(plistURL.path)")
                exit(0)
            } catch {
                fputs("Could not write the Codex launch monitor manifest: \(error.localizedDescription)\n", stderr)
                exit(1)
            }
        }

        if CommandLine.arguments.contains("--uninstall-integration") {
            do {
                let plistURL = try LaunchAgentIntegration.uninstall()
                print("Codex launch monitor manifest removed: \(plistURL.path)")
                exit(0)
            } catch {
                fputs("Could not remove the Codex launch monitor: \(error.localizedDescription)\n", stderr)
                exit(1)
            }
        }

        if CommandLine.arguments.contains("--self-test") {
            do {
                guard let url = exampleConfigurationURL() else { throw LauncherError.missingConfig }
                let configuration = try LauncherConfiguration.decode(from: Data(contentsOf: url))
                let opacityChecks: [(Double, Double)] = [
                    (configuration.transitionStart, 1),
                    ((configuration.transitionStart + configuration.transitionEnd) / 2, 0.5),
                    (configuration.transitionEnd, 0),
                ]
                for (time, expected) in opacityChecks {
                    guard abs(configuration.playerOpacity(at: time) - expected) < 0.0001 else {
                        throw LauncherConfigurationError("播放器淡出曲线检查失败。")
                    }
                }
                guard configuration.outroStyle == .original else {
                    throw LauncherConfigurationError("旧版配置的结尾样式兼容性检查失败。")
                }
                var darkConfiguration = configuration
                darkConfiguration.outroStyle = .darkNative
                let darkChecks: [(Double, Double, Double)] = [
                    (0, 0, 0), (8, 0, 0), (8.25, 0.5, 0),
                    (8.5, 1, 0), (8.75, 1, 0.5), (9, 1, 1), (13.65, 1, 1),
                ]
                for (time, background, text) in darkChecks {
                    guard abs(darkConfiguration.outroOpacity(at: time) - background) < 0.0001,
                          abs(darkConfiguration.outroTextOpacity(at: time) - text) < 0.0001 else {
                        throw LauncherConfigurationError("暗色结尾时间线检查失败。")
                    }
                }
                let encoded = try JSONEncoder().encode(darkConfiguration)
                guard try LauncherConfiguration.decode(from: encoded) == darkConfiguration else {
                    throw LauncherConfigurationError("结尾样式配置读写检查失败。")
                }
                for step in 0...240 {
                    let time = Double(step) / 24
                    guard configuration.outroOpacity(at: time) == 0 else {
                        throw LauncherConfigurationError("原版样式不应显示暗色覆盖层。")
                    }
                }
                darkConfiguration.holdAt = 7
                do {
                    try darkConfiguration.validate()
                    throw LauncherConfigurationError("无效的 darkNative 时间线未被拒绝。")
                } catch let error as LauncherConfigurationError where error.message.contains("HoldAt") {
                    // The native outro must be fully opaque before playback can pause.
                }
                let agentManifest = LaunchAgentIntegration.propertyList(
                    executablePath: "/Applications/DragonCodexBoot.app/Contents/MacOS/DragonCodexBootMac"
                )
                guard (agentManifest["ProgramArguments"] as? [String])?.last == "--watch" else {
                    throw LauncherConfigurationError("登录监视器启动参数检查失败。")
                }
                _ = try PropertyListSerialization.data(fromPropertyList: agentManifest, format: .xml, options: 0)
                print("macOS legacy configuration, dark outro timeline, round-trip, fade/audio envelope, and launch monitor checks passed: \(url.path)")
                exit(0)
            } catch {
                fputs("macOS launcher self-test failed: \(error.localizedDescription)\n", stderr)
                exit(1)
            }
        }

        let application = NSApplication.shared
        let appDelegate = AppDelegate()
        application.delegate = appDelegate
        application.run()
    }
}
