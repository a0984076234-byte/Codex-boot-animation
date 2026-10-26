# Architecture

## Runtime layers

```text
Tauri control center
  ├─ theme catalog and safe import
  ├─ preview and user configuration
  └─ platform adapter
       ├─ Windows C# WPF/DWM launcher
       └─ macOS Swift/LaunchAgent launcher
```

The control center owns user intent and theme metadata. The native launchers own
window discovery, media playback, DPI/window geometry, and handoff to Codex.
This keeps platform-specific desktop APIs out of the web UI.

## User data

Themes, active theme state, launcher configuration, backups, and logs are stored
under the operating-system application data directory. The repository itself is
never used as a mutable runtime directory in a packaged build.

## Trust model

Theme repositories are untrusted input. Import is declarative and offline-first:
the importer downloads a public GitHub archive, requires `codex-theme.json`, checks
relative paths and file limits, and never executes scripts or binaries from the
archive. Enabling a launcher is an explicit user action after preview.
