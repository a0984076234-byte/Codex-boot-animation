use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
use zip::ZipArchive;

const MAX_VIDEO_BYTES: u64 = 250 * 1024 * 1024;
const MAX_POSTER_BYTES: u64 = 10 * 1024 * 1024;
const MAX_THEME_ARCHIVE_BYTES: usize = 300 * 1024 * 1024;
const MAX_THEME_TOTAL_BYTES: u64 = 300 * 1024 * 1024;
static OPERATIONS: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScreenFrame {
    time: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Timeline {
    hold_at: f64,
    transition_start: f64,
    transition_end: f64,
    max_wait_seconds: f64,
    volume: f64,
    screen_frames: Vec<ScreenFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThemeManifest {
    schema_version: u32,
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    description: String,
    video: String,
    #[serde(default)]
    poster: Option<String>,
    license: String,
    source: String,
    #[serde(default)]
    media_notice: Option<String>,
    timeline: Timeline,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThemeSummary {
    id: String,
    name: String,
    version: String,
    description: String,
    license: String,
    source: String,
    video_path: String,
    poster_path: Option<String>,
    timeline: Timeline,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LauncherStatus {
    platform: String,
    enabled: bool,
    active_theme: Option<String>,
    native_available: bool,
    message: String,
    data_path: String,
    backup_available: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActionRequest {
    action: String,
    #[serde(default)]
    theme_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimelineUpdateRequest {
    theme_id: String,
    timeline: Timeline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LauncherBackup {
    version: u32,
    created_at: u128,
    active_theme: Option<String>,
    enabled: bool,
    #[serde(default)]
    launcher_config_present: bool,
    #[serde(default, deserialize_with = "deserialize_config_bytes")]
    launcher_config: Option<Vec<u8>>,
    #[serde(default)]
    mac_launch_agent: Option<Vec<u8>>,
    #[serde(default)]
    mac_agent_loaded: bool,
}

fn deserialize_config_bytes<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    value
        .map(|value| {
            if value.is_array() {
                serde_json::from_value(value).map_err(serde::de::Error::custom)
            } else {
                serde_json::to_vec_pretty(&value).map_err(serde::de::Error::custom)
            }
        })
        .transpose()
}

fn data_root(app: &AppHandle) -> Result<PathBuf, String> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法确定应用数据目录：{error}"))?;
    fs::create_dir_all(root.join("themes")).map_err(|error| error.to_string())?;
    Ok(root)
}

fn atomic_write(path: &Path, data: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("无法确定文件目录：{}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("codex"),
        unix_id()
    ));
    fs::write(&temporary, data).map_err(|error| error.to_string())?;
    // Windows rename does not replace an existing file. Keep the old bytes until
    // the complete replacement has been written, then restore them on failure.
    let previous = path.with_extension(format!("previous-{}", unix_id()));
    if path.exists() {
        fs::rename(path, &previous).map_err(|error| error.to_string())?;
    }
    let result = fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        if previous.exists() {
            let _ = fs::rename(&previous, path);
        }
        error.to_string()
    });
    if result.is_ok() {
        let _ = fs::remove_file(previous);
    }
    result
}

fn home_dir() -> Result<PathBuf, String> {
    let variable = if cfg!(target_os = "windows") {
        "USERPROFILE"
    } else {
        "HOME"
    };
    std::env::var_os(variable)
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .ok_or_else(|| format!("无法确定用户目录（{variable}）"))
}

fn mac_native_root() -> Result<PathBuf, String> {
    Ok(home_dir()?.join("Library/Application Support/DragonCodexBoot"))
}

fn mac_launch_agent_path() -> Result<PathBuf, String> {
    Ok(home_dir()?
        .join("Library/LaunchAgents")
        .join("community.dragoncodexboot.codex-watcher.plist"))
}

fn read_manifest(path: &Path) -> Result<ThemeManifest, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 1024 * 1024 {
        return Err("主题清单必须是小于 1 MB 的普通 JSON 文件".into());
    }
    let data = fs::read(path).map_err(|error| format!("无法读取主题清单：{error}"))?;
    let manifest: ThemeManifest =
        serde_json::from_slice(&data).map_err(|error| format!("主题清单 JSON 无效：{error}"))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_manifest(manifest: &ThemeManifest) -> Result<(), String> {
    if manifest.schema_version != 1 {
        return Err("只支持 schemaVersion 1 的主题清单".into());
    }
    for (label, value) in [
        ("主题 id", manifest.id.as_str()),
        ("主题名称", manifest.name.as_str()),
        ("主题版本", manifest.version.as_str()),
        ("许可证", manifest.license.as_str()),
        ("来源", manifest.source.as_str()),
    ] {
        if value.trim().is_empty() || value.len() > 2048 {
            return Err(format!("{label}不能为空"));
        }
    }
    if manifest.name.len() > 256 || manifest.description.len() > 4096 {
        return Err("主题名称或简介超过长度限制".into());
    }
    for value in [&manifest.id, &manifest.version] {
        if value.is_empty()
            || value.len() > 80
            || value.starts_with('.')
            || !is_safe_relative_path(value)
            || value
                .chars()
                .any(|c| !c.is_ascii_alphanumeric() && !"-_.".contains(c))
        {
            return Err("主题 id 和版本只允许 1–80 个英文字母、数字、点、短横线和下划线".into());
        }
    }
    if manifest.id != manifest.id.to_ascii_lowercase() {
        return Err("主题 id 只允许小写字母、数字、点、短横线和下划线".into());
    }
    if manifest.video.trim().is_empty() || !is_safe_relative_path(&manifest.video) {
        return Err("video 必须是主题目录内的相对路径".into());
    }
    github_parts(&manifest.source)?;
    for (label, value) in [
        ("video", &manifest.video),
        ("poster", &manifest.poster.clone().unwrap_or_default()),
        (
            "mediaNotice",
            &manifest.media_notice.clone().unwrap_or_default(),
        ),
    ] {
        if !value.is_empty() && !is_safe_relative_path(value) {
            return Err(format!("{label} 必须是主题目录内的相对路径"));
        }
    }
    let timeline = &manifest.timeline;
    if !timeline.hold_at.is_finite()
        || !timeline.transition_start.is_finite()
        || !timeline.transition_end.is_finite()
        || !timeline.max_wait_seconds.is_finite()
        || !timeline.volume.is_finite()
        || !(0.0..=1.0).contains(&timeline.volume)
        || timeline.hold_at <= 0.0
        || timeline.hold_at >= timeline.transition_start
        || timeline.transition_start >= timeline.transition_end
        || timeline.max_wait_seconds < 10.0
        || timeline.max_wait_seconds > 3600.0
    {
        return Err("主题时间线或音量范围无效".into());
    }
    if timeline.screen_frames.len() < 2 || timeline.screen_frames.len() > 200 {
        return Err("主题至少需要两个屏幕关键帧".into());
    }
    let mut previous_time = -1.0;
    for frame in &timeline.screen_frames {
        if !frame.time.is_finite()
            || !frame.x.is_finite()
            || !frame.y.is_finite()
            || !frame.width.is_finite()
            || !frame.height.is_finite()
            || frame.time <= previous_time
            || frame.x < 0.0
            || frame.y < 0.0
            || frame.width <= 0.0
            || frame.height <= 0.0
            || frame.x + frame.width > 1.0001
            || frame.y + frame.height > 1.0001
        {
            return Err("主题屏幕关键帧无效或未按时间递增".into());
        }
        previous_time = frame.time;
    }
    Ok(())
}

fn is_safe_relative_path(value: &str) -> bool {
    if value.trim().is_empty()
        || value.contains('\0')
        || value.starts_with('/')
        || value
            .chars()
            .any(|c| c.is_control() || "\\:<>\"|?*".contains(c))
    {
        return false;
    }
    value.split('/').all(|component| {
        let stem = component
            .split('.')
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        !component.is_empty()
            && component != "."
            && component != ".."
            && !component.ends_with('.')
            && !component.ends_with(' ')
            && ![
                "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
                "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
                "LPT9",
            ]
            .contains(&stem.as_str())
    })
}

fn allowed_extension(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "mp4" | "webm" | "mov" | "png" | "jpg" | "jpeg" | "webp" | "md"
    )
}

fn validate_files(root: &Path, manifest: &ThemeManifest) -> Result<(), String> {
    for relative in [
        Some(manifest.video.as_str()),
        manifest.poster.as_deref(),
        manifest.media_notice.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        let file = root.join(relative);
        let mut current = file.parent();
        while let Some(path) = current {
            let metadata =
                fs::symlink_metadata(path).map_err(|error| format!("无法检查主题目录：{error}"))?;
            if metadata.file_type().is_symlink() {
                return Err(format!("主题路径包含符号链接：{relative}"));
            }
            if path == root {
                break;
            }
            current = path.parent();
        }
        let metadata = fs::symlink_metadata(&file)
            .map_err(|error| format!("主题文件不存在 {relative}：{error}"))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || !allowed_extension(&file) {
            return Err(format!("主题文件类型不受支持：{relative}"));
        }
        let limit = if relative == manifest.video.as_str() {
            MAX_VIDEO_BYTES
        } else if manifest.poster.as_deref() == Some(relative) {
            MAX_POSTER_BYTES
        } else {
            2 * 1024 * 1024
        };
        if metadata.len() > limit {
            return Err(format!("主题文件超过大小限制：{relative}"));
        }
    }
    let video_extension = Path::new(&manifest.video)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["mp4", "mov", "webm"].contains(&video_extension.as_str()) {
        return Err("video 必须是 MP4、MOV 或 WebM".into());
    }
    if let Some(poster) = &manifest.poster {
        let extension = Path::new(poster)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !["png", "jpg", "jpeg", "webp"].contains(&extension.as_str()) {
            return Err("poster 必须是 PNG、JPG、JPEG 或 WebP".into());
        }
    }
    Ok(())
}

fn import_theme_dir(app: &AppHandle, source: &Path) -> Result<ThemeSummary, String> {
    let _guard = OPERATIONS
        .lock()
        .map_err(|_| "主题操作正在进行，请稍后重试".to_string())?;
    let manifest = read_manifest(&source.join("codex-theme.json"))?;
    if integration_enabled(app)? && active_theme_id(app)?.as_deref() == Some(manifest.id.as_str()) {
        return Err("请先停用启动动画，再替换正在使用的主题".into());
    }
    import_into(&data_root(app)?.join("themes"), source)
}

fn import_into(root: &Path, source: &Path) -> Result<ThemeSummary, String> {
    let source_metadata = fs::symlink_metadata(source).map_err(|error| error.to_string())?;
    if !source_metadata.is_dir() || source_metadata.file_type().is_symlink() {
        return Err("主题来源必须是普通目录".into());
    }
    let manifest_path = source.join("codex-theme.json");
    let manifest = read_manifest(&manifest_path)?;
    validate_files(source, &manifest)?;
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let folder = manifest.id.to_ascii_lowercase();
    if folder.is_empty() {
        return Err("主题 id 无法生成安全目录名".into());
    }
    let destination = root.join(&folder);
    let staging = root.join(format!(".staging-{}-{}", folder, unix_id()));
    let previous = root.join(format!(".previous-{}-{}", folder, unix_id()));
    let result = (|| {
        fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
        for relative in [
            Some("codex-theme.json"),
            Some(manifest.video.as_str()),
            manifest.poster.as_deref(),
            manifest.media_notice.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            let target = staging.join(relative);
            fs::create_dir_all(target.parent().ok_or("主题路径无效")?)
                .map_err(|error| error.to_string())?;
            fs::copy(source.join(relative), target).map_err(|error| error.to_string())?;
        }
        let copied = read_manifest(&staging.join("codex-theme.json"))?;
        validate_files(&staging, &copied)?;
        if destination.exists() {
            fs::rename(&destination, &previous).map_err(|error| error.to_string())?;
        }
        if let Err(error) = fs::rename(&staging, &destination) {
            let _ = fs::rename(&previous, &destination);
            return Err(error.to_string());
        }
        let _ = fs::remove_dir_all(&previous);
        Ok(summary_for(&destination, copied))
    })();
    let _ = fs::remove_dir_all(&staging);
    result
}

fn summary_for(root: &Path, manifest: ThemeManifest) -> ThemeSummary {
    ThemeSummary {
        id: manifest.id,
        name: manifest.name,
        version: manifest.version,
        description: manifest.description,
        license: manifest.license,
        source: manifest.source,
        video_path: root.join(manifest.video).to_string_lossy().to_string(),
        poster_path: manifest
            .poster
            .map(|poster| root.join(poster).to_string_lossy().to_string()),
        timeline: manifest.timeline,
    }
}

fn unix_id() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| "0".into())
}

fn active_theme_id(app: &AppHandle) -> Result<Option<String>, String> {
    let root = data_root(app)?;
    Ok(fs::read_to_string(root.join("active-theme.json"))
        .ok()
        .and_then(|data| serde_json::from_str::<serde_json::Value>(&data).ok())
        .and_then(|value| {
            value
                .get("id")
                .and_then(|id| id.as_str())
                .map(str::to_string)
        }))
}

fn integration_enabled(app: &AppHandle) -> Result<bool, String> {
    if cfg!(target_os = "macos") {
        return mac_agent_loaded();
    }
    Ok(data_root(app)?.join("launcher-enabled").is_file())
}

fn mac_domain() -> Result<String, String> {
    let output = Command::new("/usr/bin/id")
        .arg("-u")
        .output()
        .map_err(|error| error.to_string())?;
    let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || uid.is_empty() || !uid.chars().all(|c| c.is_ascii_digit()) {
        return Err("无法确定当前 macOS 登录会话".into());
    }
    Ok(format!("gui/{uid}"))
}

fn mac_agent_loaded() -> Result<bool, String> {
    Ok(Command::new("/bin/launchctl")
        .args([
            "print",
            &format!("{}/community.dragoncodexboot.codex-watcher", mac_domain()?),
        ])
        .output()
        .map_err(|error| error.to_string())?
        .status
        .success())
}

fn mac_bootout() -> Result<(), String> {
    if mac_agent_loaded()? {
        let output = Command::new("/bin/launchctl")
            .args([
                "bootout",
                &format!("{}/community.dragoncodexboot.codex-watcher", mac_domain()?),
            ])
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "无法停用 LaunchAgent：{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    Ok(())
}

fn mac_bootstrap() -> Result<(), String> {
    if mac_agent_loaded()? {
        return Ok(());
    }
    let output = Command::new("/bin/launchctl")
        .arg("bootstrap")
        .arg(mac_domain()?)
        .arg(mac_launch_agent_path()?)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() || !mac_agent_loaded()? {
        return Err(format!(
            "无法启用 LaunchAgent：{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn find_theme(app: &AppHandle, id: &str) -> Result<(PathBuf, ThemeManifest), String> {
    let root = data_root(app)?.join("themes");
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let manifest_path = entry.path().join("codex-theme.json");
        if !manifest_path.is_file() {
            continue;
        }
        let manifest = read_manifest(&manifest_path)?;
        if manifest.id == id {
            validate_files(&entry.path(), &manifest)?;
            return Ok((entry.path(), manifest));
        }
    }
    Err("主题不存在或清单校验失败".into())
}

fn repository_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        roots.push(current.clone());
        if let Some(parent) = current.parent() {
            roots.push(parent.to_path_buf());
        }
    }
    if let Ok(executable) = std::env::current_exe() {
        let mut directory = executable.parent().map(Path::to_path_buf);
        for _ in 0..5 {
            if let Some(path) = directory {
                roots.push(path.clone());
                directory = path.parent().map(Path::to_path_buf);
            }
        }
    }
    roots
}

fn find_repository_file(relative: &str) -> Option<PathBuf> {
    repository_roots()
        .into_iter()
        .map(|root| root.join(relative))
        .find(|path| path.is_file())
}

fn mac_native_path(app: &AppHandle) -> Option<PathBuf> {
    let relative = "DragonCodexBoot.app/Contents/MacOS/DragonCodexBootMac";
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("CODEX_BOOT_NATIVE_COMMAND").map(PathBuf::from) {
        candidates.push(path);
    }
    if let Ok(root) = app.path().resource_dir() {
        candidates.push(root.join("native/macos").join(relative));
    }
    candidates.extend(
        repository_roots()
            .into_iter()
            .map(|root| root.join("build").join(relative)),
    );
    candidates.into_iter().find(|path| path.is_file())
}

fn stage_mac_native(app: &AppHandle) -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("CODEX_BOOT_NATIVE_COMMAND").map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }
    }
    let source =
        mac_native_path(app).ok_or("未找到 macOS 原生引擎，请先运行 scripts/build-macos.sh")?;
    let source_bundle = source
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or("macOS 原生引擎路径无效")?;
    let destination = data_root(app)?
        .join("native/macos")
        .join(env!("CARGO_PKG_VERSION"))
        .join("DragonCodexBoot.app");
    copy_native_bundle(source_bundle, &destination)?;
    Ok(destination.join("Contents/MacOS/DragonCodexBootMac"))
}

fn windows_resource_root(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .resource_dir()
        .ok()
        .map(|root| root.join("native/windows/DragonCodexBoot"))
        .filter(|path| path.is_dir())
}

fn copy_native_bundle(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Err(format!("原生引擎目录不存在：{}", source.display()));
    }
    if destination.exists() {
        return Ok(());
    }
    let staging = destination.with_extension(format!("staging-{}", unix_id()));
    let result = (|| {
        fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
        for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_symlink() {
                return Err("原生引擎资源不能包含符号链接".into());
            }
            let target = staging.join(entry.file_name());
            if file_type.is_dir() {
                copy_native_bundle(&entry.path(), &target)?;
            } else if file_type.is_file() {
                fs::copy(entry.path(), target).map_err(|error| error.to_string())?;
            }
        }
        if destination.exists() {
            return Ok(());
        }
        fs::rename(&staging, destination).map_err(|error| error.to_string())
    })();
    let _ = fs::remove_dir_all(&staging);
    result
}

fn stage_windows_native(app: &AppHandle) -> Result<PathBuf, String> {
    let root = data_root(app)?.join("native/windows/DragonCodexBoot");
    if !root.join("DragonCodexBoot.exe").is_file() {
        let source = windows_resource_root(app)
            .ok_or_else(|| "发行包中没有 Windows 原生 sidecar".to_string())?;
        copy_native_bundle(&source, &root)?;
    }
    if !root.join("DragonCodexBoot.exe").is_file() {
        return Err("Windows 原生 sidecar 不完整".into());
    }
    Ok(root.join("DragonCodexBoot.exe"))
}

fn windows_native_path(app: Option<&AppHandle>) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("CODEX_BOOT_NATIVE_COMMAND").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }
    let mut candidates = Vec::new();
    if let Some(app) = app {
        if cfg!(target_os = "windows") {
            if let Ok(root) = data_root(app) {
                candidates.push(root.join("native/windows/DragonCodexBoot/DragonCodexBoot.exe"));
            }
        }
        if let Ok(resource_root) = app.path().resource_dir() {
            candidates.extend([
                resource_root.join("native/windows/DragonCodexBoot/DragonCodexBoot.exe"),
                resource_root.join("windows/DragonCodexBoot/DragonCodexBoot.exe"),
                resource_root.join("native/DragonCodexBoot/DragonCodexBoot.exe"),
                resource_root.join("build/DragonCodexBoot/DragonCodexBoot.exe"),
            ]);
        }
    }
    candidates.extend(repository_roots().into_iter().flat_map(|root| {
        [
            root.join("DragonCodexBoot.exe"),
            root.join("build/DragonCodexBoot/DragonCodexBoot.exe"),
            root.join("control-center/src-tauri/DragonCodexBoot.exe"),
            root.join("native/DragonCodexBoot.exe"),
        ]
    }));
    candidates.into_iter().find(|path| path.is_file())
}

fn native_config_root(app: &AppHandle) -> Result<PathBuf, String> {
    if cfg!(target_os = "macos") {
        return mac_native_root();
    }
    if cfg!(target_os = "windows") {
        return data_root(app).map(|root| root.join("native/windows/DragonCodexBoot"));
    }
    let _ = app;
    Err("当前平台不受支持".into())
}

fn native_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(native_config_root(app)?.join("launcher.json"))
}

fn launcher_config_for_theme(app: &AppHandle, theme_id: &str) -> Result<serde_json::Value, String> {
    let (theme_root, manifest) = find_theme(app, theme_id)?;
    let video_path = theme_root.join(&manifest.video);
    if !video_path.is_file() {
        return Err("主题启动视频不存在".into());
    }
    let template = if cfg!(target_os = "windows") {
        include_str!("../../../config/launcher.example.json")
    } else {
        include_str!("../../../config/launcher.macos.example.json")
    };
    let template: serde_json::Value =
        serde_json::from_str(template).map_err(|error| format!("原生配置模板无效：{error}"))?;
    let mut config = match fs::read(native_config_path(app)?) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("现有 launcher.json 无效；已保留原文件：{error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => template,
        Err(error) => return Err(error.to_string()),
    };
    let object = config
        .as_object_mut()
        .ok_or_else(|| "原生配置模板必须是 JSON 对象".to_string())?;
    object.insert(
        "DisplayName".into(),
        serde_json::Value::String("Codex Boot Animation".into()),
    );
    object.insert(
        "Video".into(),
        serde_json::Value::String(video_path.to_string_lossy().to_string()),
    );
    object.insert(
        "HoldAt".into(),
        serde_json::json!(manifest.timeline.hold_at),
    );
    object.insert(
        "TransitionStart".into(),
        serde_json::json!(manifest.timeline.transition_start),
    );
    object.insert(
        "TransitionEnd".into(),
        serde_json::json!(manifest.timeline.transition_end),
    );
    object.insert(
        "MaxWaitSeconds".into(),
        serde_json::json!(manifest.timeline.max_wait_seconds),
    );
    object.insert("Volume".into(), serde_json::json!(manifest.timeline.volume));
    object.insert("AutoReplaceEntrypoints".into(), serde_json::json!(false));
    if cfg!(target_os = "macos") {
        object.insert("OutroStyle".into(), serde_json::json!("original"));
    }
    if cfg!(target_os = "windows") {
        object.insert(
            "ScreenFrames".into(),
            serde_json::Value::Array(
                manifest
                    .timeline
                    .screen_frames
                    .iter()
                    .map(|frame| {
                        serde_json::json!({
                            "Time": frame.time,
                            "X": frame.x,
                            "Y": frame.y,
                            "Width": frame.width,
                            "Height": frame.height
                        })
                    })
                    .collect(),
            ),
        );
    }
    Ok(config)
}

fn write_theme_config(app: &AppHandle, theme_id: &str) -> Result<serde_json::Value, String> {
    let config = launcher_config_for_theme(app, theme_id)?;
    let bytes = serde_json::to_vec_pretty(&config).map_err(|error| error.to_string())?;
    atomic_write(&native_config_path(app)?, &bytes)?;
    Ok(config)
}

fn write_preview_config(app: &AppHandle, theme_id: &str) -> Result<PathBuf, String> {
    let config = launcher_config_for_theme(app, theme_id)?;
    isolated_preview_config(&config)
}

fn isolated_preview_config(config: &serde_json::Value) -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join(format!("codex-boot-preview-{}", unix_id()));
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec_pretty(config).map_err(|error| error.to_string())?;
    let path = directory.join("launcher.json");
    if let Err(error) = atomic_write(&path, &bytes) {
        let _ = fs::remove_dir_all(&directory);
        return Err(error);
    }
    Ok(path)
}

fn native_available(app: &AppHandle) -> bool {
    if cfg!(target_os = "windows") {
        return windows_native_path(Some(app)).is_some() || windows_resource_root(app).is_some();
    }
    if cfg!(target_os = "macos") {
        return mac_native_path(app).is_some();
    }
    false
}

fn github_parts(url: &str) -> Result<(String, String), String> {
    let trimmed = url.trim().trim_end_matches('/');
    let rest = trimmed
        .strip_prefix("https://github.com/")
        .ok_or_else(|| "只支持公开 GitHub HTTPS 地址".to_string())?;
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|part| {
            part.is_empty()
                || part.contains("..")
                || part
                    .chars()
                    .any(|c| !c.is_ascii_alphanumeric() && !"-_.".contains(c))
        })
    {
        return Err("GitHub 地址必须是 https://github.com/<owner>/<repo>".into());
    }
    let repo = parts[1].strip_suffix(".git").unwrap_or(parts[1]);
    if repo.is_empty() || parts[0].starts_with('.') || repo.starts_with('.') {
        return Err("GitHub 仓库名称无效".into());
    }
    Ok((parts[0].to_string(), repo.to_string()))
}

fn extract_zip(data: &[u8], destination: &Path) -> Result<(), String> {
    if data.len() > MAX_THEME_ARCHIVE_BYTES {
        return Err("GitHub 主题压缩包超过 300 MB 限制".into());
    }
    let mut archive =
        ZipArchive::new(Cursor::new(data)).map_err(|error| format!("主题压缩包无效：{error}"))?;
    let mut total_uncompressed = 0_u64;
    if archive.len() > 10000 {
        return Err("主题压缩包文件数超过 10000".into());
    }
    let mut archive_root: Option<String> = None;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = file.name().trim_end_matches('/').to_string();
        let path = Path::new(&name);
        if !is_safe_relative_path(path.to_str().unwrap_or("")) {
            return Err("GitHub 压缩包包含不安全路径".into());
        }
        if file
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("主题压缩包不能包含符号链接".into());
        }
        let first = name.split('/').next().unwrap_or("");
        if let Some(expected) = &archive_root {
            if expected != first {
                return Err("GitHub 压缩包必须只有一个仓库根目录".into());
            }
        } else {
            archive_root = Some(first.to_string());
        }
        let relative = path.components().skip(1).collect::<PathBuf>();
        if relative.as_os_str().is_empty() {
            continue;
        }
        let output = destination.join(relative);
        if file.is_dir() {
            fs::create_dir_all(&output).map_err(|error| error.to_string())?;
        } else {
            if !allowed_extension(&output)
                && output.file_name().and_then(|value| value.to_str()) != Some("codex-theme.json")
            {
                continue;
            }
            total_uncompressed = total_uncompressed.saturating_add(file.size());
            if total_uncompressed > MAX_THEME_TOTAL_BYTES {
                return Err("GitHub 主题解压内容超过 300 MB 限制".into());
            }
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let mut output_file = fs::File::create(&output).map_err(|error| error.to_string())?;
            std::io::copy(&mut file, &mut output_file).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn find_manifest(root: &Path) -> Result<PathBuf, String> {
    if root.join("codex-theme.json").is_file() {
        return Ok(root.to_path_buf());
    }
    Err("GitHub 仓库根目录没有 codex-theme.json".into())
}

#[tauri::command]
fn list_themes(app: AppHandle) -> Result<Vec<ThemeSummary>, String> {
    let root = data_root(&app)?.join("themes");
    let mut themes = Vec::new();
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            let manifest_path = entry.path().join("codex-theme.json");
            if manifest_path.is_file() {
                if let Ok(manifest) = read_manifest(&manifest_path) {
                    if validate_files(&entry.path(), &manifest).is_ok() {
                        themes.push(summary_for(&entry.path(), manifest));
                    }
                }
            }
        }
    }
    themes.sort_by_key(|theme| theme.name.to_lowercase());
    Ok(themes)
}

#[tauri::command]
fn import_local_theme(app: AppHandle, path: String) -> Result<ThemeSummary, String> {
    import_theme_dir(&app, Path::new(&path))
}

#[tauri::command]
async fn import_github_theme(app: AppHandle, url: String) -> Result<ThemeSummary, String> {
    tauri::async_runtime::spawn_blocking(move || download_github_theme(app, url))
        .await
        .map_err(|error| error.to_string())?
}

fn download_github_theme(app: AppHandle, url: String) -> Result<ThemeSummary, String> {
    let (owner, repo) = github_parts(&url)?;
    let client = Client::builder()
        .user_agent(concat!("Codex-Boot-Animation/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|error| error.to_string())?;
    let metadata: serde_json::Value = serde_json::from_reader(
        client
            .get(format!("https://api.github.com/repos/{owner}/{repo}"))
            .send()
            .map_err(|error| format!("无法连接 GitHub：{error}"))?
            .error_for_status()
            .map_err(|error| format!("仓库不可公开访问或 API 限流：{error}"))?
            .take(1024 * 1024),
    )
    .map_err(|error| format!("GitHub 仓库信息无效：{error}"))?;
    if metadata.get("private").and_then(serde_json::Value::as_bool) != Some(false) {
        return Err("只允许公开 GitHub 仓库".into());
    }
    let response = client
        .get(format!(
            "https://codeload.github.com/{owner}/{repo}/zip/HEAD"
        ))
        .send()
        .map_err(|error| format!("下载公开仓库失败：{error}"))?
        .error_for_status()
        .map_err(|error| format!("仓库默认分支不可下载：{error}"))?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_THEME_ARCHIVE_BYTES as u64)
    {
        return Err("GitHub 主题压缩包超过 300 MB 限制".into());
    }
    let mut bytes = Vec::new();
    response
        .take(MAX_THEME_ARCHIVE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("下载主题失败：{error}"))?;
    if bytes.len() > MAX_THEME_ARCHIVE_BYTES {
        return Err("GitHub 主题压缩包超过 300 MB 限制".into());
    }
    let temporary = std::env::temp_dir().join(format!("codex-theme-{}", unix_id()));
    fs::create_dir_all(&temporary).map_err(|error| error.to_string())?;
    let result = (|| {
        extract_zip(&bytes, &temporary)?;
        let theme_root = find_manifest(&temporary)?;
        import_theme_dir(&app, &theme_root)
    })();
    let _ = fs::remove_dir_all(&temporary);
    result
}

#[tauri::command]
fn remove_theme(app: AppHandle, id: String) -> Result<(), String> {
    let _guard = OPERATIONS.lock().map_err(|error| error.to_string())?;
    if integration_enabled(&app)? && active_theme_id(&app)?.as_deref() == Some(&id) {
        return Err("请先停用启动动画，再删除当前主题".into());
    }
    let root = data_root(&app)?.join("themes");
    if id.trim().is_empty() {
        return Err("主题 id 不能为空".into());
    }
    for entry in fs::read_dir(&root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let manifest_path = entry.path().join("codex-theme.json");
        if manifest_path.is_file()
            && read_manifest(&manifest_path)
                .map(|manifest| manifest.id == id)
                .unwrap_or(false)
        {
            fs::remove_dir_all(entry.path()).map_err(|error| error.to_string())?;
        }
    }
    if active_theme_id(&app)?.as_deref() == Some(id.as_str()) {
        let _ = fs::remove_file(data_root(&app)?.join("active-theme.json"));
    }
    Ok(())
}

#[tauri::command]
fn set_active_theme(app: AppHandle, id: String) -> Result<(), String> {
    let _guard = OPERATIONS.lock().map_err(|error| error.to_string())?;
    if integration_enabled(&app)? {
        return Err("请先停用启动动画，再切换主题".into());
    }
    let root = data_root(&app)?;
    let themes = list_themes(app.clone())?;
    if !themes.iter().any(|theme| theme.id == id) {
        return Err("主题不存在或清单校验失败".into());
    }
    atomic_write(
        &root.join("active-theme.json"),
        &serde_json::to_vec_pretty(&serde_json::json!({ "id": id }))
            .map_err(|error| error.to_string())?,
    )
}

#[tauri::command]
fn update_theme_timeline(
    app: AppHandle,
    request: TimelineUpdateRequest,
) -> Result<ThemeSummary, String> {
    let _guard = OPERATIONS.lock().map_err(|error| error.to_string())?;
    if integration_enabled(&app)? && active_theme_id(&app)?.as_deref() == Some(&request.theme_id) {
        return Err("请先停用启动动画，再修改当前主题时间线".into());
    }
    let (root, mut manifest) = find_theme(&app, &request.theme_id)?;
    manifest.timeline = request.timeline;
    validate_manifest(&manifest)?;
    validate_files(&root, &manifest)?;
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    atomic_write(&root.join("codex-theme.json"), &bytes)?;
    Ok(summary_for(&root, manifest))
}

#[tauri::command]
fn launcher_status(app: AppHandle) -> Result<LauncherStatus, String> {
    let root = data_root(&app)?;
    let active_theme = active_theme_id(&app)?;
    let enabled = integration_enabled(&app)?;
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "unsupported"
    };
    Ok(LauncherStatus {
        platform: platform.into(),
        enabled,
        active_theme,
        native_available: native_available(&app),
        data_path: root.to_string_lossy().to_string(),
        backup_available: root.join("launcher-backup.json").is_file(),
        message: if enabled {
            "启动动画已启用".into()
        } else {
            "尚未接管 Codex 启动入口".into()
        },
    })
}

fn native_action(
    app: &AppHandle,
    action: &str,
    preview_config: Option<&Path>,
) -> Result<String, String> {
    if cfg!(target_os = "macos") && action == "disable" {
        mac_bootout()?;
        remove_if_present(&mac_launch_agent_path()?)?;
        return Ok("LaunchAgent disabled".into());
    }
    let path = if cfg!(target_os = "macos") {
        if action == "preview" {
            mac_native_path(app).ok_or("原生引擎缺失，请先构建 macOS 原生引擎")?
        } else {
            mac_bootout()?;
            stage_mac_native(app)?
        }
    } else if cfg!(target_os = "windows") {
        if action == "preview" {
            windows_native_path(Some(app)).ok_or("Windows 原生引擎缺失")?
        } else {
            stage_windows_native(app)?
        }
    } else {
        return Err("当前平台不受支持".into());
    };
    let mut command = Command::new(path);
    command.arg(native_argument(action)?);
    if action == "preview" {
        command
            .arg("--config")
            .arg(preview_config.ok_or("预览缺少隔离配置")?);
    }
    let output = command.output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(if stderr.is_empty() { stdout } else { stderr });
    }
    if cfg!(target_os = "macos") && action == "enable" {
        mac_bootstrap()?;
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn remove_if_present(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn restore_config(path: &Path, backup: &LauncherBackup) -> Result<(), String> {
    if let Some(bytes) = &backup.launcher_config {
        atomic_write(path, bytes)
    } else if backup.launcher_config_present {
        Err("启动器备份缺少原始配置内容".into())
    } else {
        remove_if_present(path)
    }
}

fn native_argument(action: &str) -> Result<&'static str, String> {
    match action {
        "preview" => Ok("--preview"),
        "enable" if cfg!(target_os = "windows") => Ok("--integrate-entrypoints"),
        "disable" if cfg!(target_os = "windows") => Ok("--restore-entrypoints"),
        "enable" if cfg!(target_os = "macos") => Ok("--install-integration"),
        "disable" if cfg!(target_os = "macos") => Ok("--uninstall-integration"),
        _ => Err(format!("不支持的原生操作：{action}")),
    }
}

fn read_backup(app: &AppHandle) -> Result<LauncherBackup, String> {
    let root = data_root(app)?;
    let bytes = fs::read(root.join("launcher-backup.json"))
        .map_err(|error| format!("没有可恢复的备份：{error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("启动器备份损坏：{error}"))
}

fn capture_backup(app: &AppHandle) -> Result<LauncherBackup, String> {
    let config_path = native_config_path(app)?;
    let launcher_config_present = config_path.is_file();
    let config = if launcher_config_present {
        Some(fs::read(&config_path).map_err(|error| format!("无法读取当前启动器配置：{error}"))?)
    } else {
        None
    };
    let mac_launch_agent = if cfg!(target_os = "macos") {
        mac_launch_agent_path()
            .ok()
            .and_then(|path| fs::read(path).ok())
    } else {
        None
    };
    Ok(LauncherBackup {
        version: 2,
        created_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default(),
        active_theme: active_theme_id(app)?,
        enabled: integration_enabled(app)?,
        launcher_config_present,
        launcher_config: config,
        mac_launch_agent,
        mac_agent_loaded: if cfg!(target_os = "macos") {
            mac_agent_loaded()?
        } else {
            false
        },
    })
}

fn create_backup(app: &AppHandle) -> Result<(), String> {
    let root = data_root(app)?;
    let backup = capture_backup(app)?;
    let bytes = serde_json::to_vec_pretty(&backup).map_err(|error| error.to_string())?;
    atomic_write(&root.join("launcher-backup.json"), &bytes)?;
    let backup_dir = root.join("backups");
    atomic_write(
        &backup_dir.join(format!("{}.json", backup.created_at)),
        &bytes,
    )
}

fn restore_backup(app: &AppHandle, backup: &LauncherBackup) -> Result<(), String> {
    let root = data_root(app)?;
    native_action(app, "disable", None)?;
    restore_config(&native_config_path(app)?, backup)?;
    if let Some(id) = &backup.active_theme {
        atomic_write(
            &root.join("active-theme.json"),
            &serde_json::to_vec(&serde_json::json!({"id": id})).map_err(|e| e.to_string())?,
        )?;
    } else {
        remove_if_present(&root.join("active-theme.json"))?;
    }
    remove_if_present(&root.join("launcher-enabled"))?;
    if cfg!(target_os = "macos") {
        if let Some(agent) = &backup.mac_launch_agent {
            atomic_write(&mac_launch_agent_path()?, agent)?;
        }
        if backup.mac_agent_loaded {
            mac_bootstrap()?;
        }
    } else if backup.enabled {
        native_action(app, "enable", None)?;
    }
    if backup.enabled {
        atomic_write(&root.join("launcher-enabled"), b"enabled")?;
    }
    Ok(())
}

#[tauri::command]
async fn launcher_action(app: AppHandle, request: ActionRequest) -> Result<LauncherStatus, String> {
    tauri::async_runtime::spawn_blocking(move || perform_launcher_action(app, request))
        .await
        .map_err(|error| error.to_string())?
}

fn perform_launcher_action(
    app: AppHandle,
    request: ActionRequest,
) -> Result<LauncherStatus, String> {
    let _guard = OPERATIONS.lock().map_err(|error| error.to_string())?;
    let root = data_root(&app)?;
    match request.action.as_str() {
        "preview" => {
            let theme_id = request
                .theme_id
                .or(active_theme_id(&app)?)
                .ok_or_else(|| "请先选择一个主题".to_string())?;
            let config = write_preview_config(&app, &theme_id)?;
            let result = native_action(&app, "preview", Some(&config));
            let _ = fs::remove_dir_all(config.parent().unwrap_or_else(|| Path::new(".")));
            result?;
        }
        "enable" => {
            let theme_id = request
                .theme_id
                .or(active_theme_id(&app)?)
                .ok_or_else(|| "请先选择一个主题".to_string())?;
            if integration_enabled(&app)? {
                return Err("启动动画已启用，请先停用再更改配置".into());
            }
            find_theme(&app, &theme_id)?;
            let before = capture_backup(&app)?;
            if !root.join("launcher-enabled").is_file() {
                create_backup(&app)?;
            }
            if cfg!(target_os = "windows") {
                stage_windows_native(&app)?;
            }
            let result = (|| {
                write_theme_config(&app, &theme_id)?;
                native_action(&app, "enable", None)?;
                atomic_write(
                    &root.join("active-theme.json"),
                    &serde_json::to_vec(&serde_json::json!({"id": theme_id}))
                        .map_err(|e| e.to_string())?,
                )?;
                atomic_write(&root.join("launcher-enabled"), b"enabled")
            })();
            if let Err(error) = result {
                return match restore_backup(&app, &before) {
                    Ok(()) => Err(format!("启用失败，已恢复原状态：{error}")),
                    Err(rollback) => Err(format!(
                        "启用失败：{error}；恢复失败：{rollback}。请保留备份并检查诊断。"
                    )),
                };
            }
        }
        "disable" => {
            native_action(&app, "disable", None)?;
            remove_if_present(&root.join("launcher-enabled"))?;
        }
        "backup" => {
            create_backup(&app)?;
        }
        "restore" => {
            let backup = read_backup(&app)?;
            restore_backup(&app, &backup)?;
        }
        other => return Err(format!("未知启动器操作：{other}")),
    }
    launcher_status(app)
}

fn ensure_default_theme(app: &AppHandle) -> Result<(), String> {
    let root = data_root(app)?;
    let initialized = root.join("default-theme-imported");
    if initialized.is_file() {
        return Ok(());
    }
    let packaged = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("demo");
    let source = if packaged.join("codex-theme.json").is_file() {
        packaged
    } else {
        find_repository_file("codex-theme.json")
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .ok_or("找不到默认主题资源")?
    };
    if !root.join("themes/bundled-demo/codex-theme.json").is_file() {
        import_into(&root.join("themes"), &source)?;
    }
    if active_theme_id(app)?.is_none() {
        atomic_write(&root.join("active-theme.json"), br#"{"id":"bundled-demo"}"#)?;
    }
    atomic_write(&initialized, b"1")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            ensure_default_theme(app.handle()).map_err(std::io::Error::other)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_themes,
            import_local_theme,
            import_github_theme,
            remove_theme,
            set_active_theme,
            update_theme_timeline,
            launcher_status,
            launcher_action
        ])
        .run(tauri::generate_context!())
        .expect("error while running Codex Boot Animation");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("codex-boot-test-{}", unix_id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fixture(root: &Path) {
        fs::create_dir_all(root.join("media")).unwrap();
        fs::write(
            root.join("codex-theme.json"),
            serde_json::to_vec(&valid_manifest()).unwrap(),
        )
        .unwrap();
        fs::write(root.join("media/startup.mp4"), b"video").unwrap();
        fs::write(root.join("media/poster.png"), b"poster").unwrap();
        fs::write(root.join("media/MEDIA_NOTICE.md"), b"fixture license").unwrap();
    }

    fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn valid_manifest() -> ThemeManifest {
        ThemeManifest {
            schema_version: 1,
            id: "sample".into(),
            name: "Sample".into(),
            version: "1.0.0".into(),
            description: String::new(),
            video: "media/startup.mp4".into(),
            poster: Some("media/poster.png".into()),
            license: "MIT".into(),
            source: "https://github.com/example/sample".into(),
            media_notice: Some("media/MEDIA_NOTICE.md".into()),
            timeline: Timeline {
                hold_at: 1.0,
                transition_start: 2.0,
                transition_end: 3.0,
                max_wait_seconds: 10.0,
                volume: 1.0,
                screen_frames: vec![
                    ScreenFrame {
                        time: 2.0,
                        x: 0.0,
                        y: 0.0,
                        width: 0.5,
                        height: 0.5,
                    },
                    ScreenFrame {
                        time: 3.0,
                        x: 0.0,
                        y: 0.0,
                        width: 1.0,
                        height: 1.0,
                    },
                ],
            },
        }
    }

    #[test]
    fn accepts_valid_manifest() {
        assert!(validate_manifest(&valid_manifest()).is_ok());
    }

    #[test]
    fn rejects_path_escape() {
        let mut manifest = valid_manifest();
        manifest.video = "../outside.mp4".into();
        assert!(validate_manifest(&manifest).is_err());
        assert!(!is_safe_relative_path("../outside.mp4"));
        assert!(!is_safe_relative_path("/tmp/video.mp4"));
    }

    #[test]
    fn rejects_non_monotonic_timeline() {
        let mut manifest = valid_manifest();
        manifest.timeline.screen_frames[1].time = 1.5;
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn parses_only_public_github_repositories() {
        assert_eq!(
            github_parts("https://github.com/example/theme.git").unwrap(),
            ("example".to_string(), "theme".to_string())
        );
        assert!(github_parts("git@github.com:example/theme.git").is_err());
        assert!(github_parts("https://github.com/example/theme/issues").is_err());
        assert!(github_parts("https://github.com/example/theme?ref=evil").is_err());
        assert!(github_parts("https://github.com.evil.test/example/theme").is_err());
        assert!(github_parts("https://github.com/../theme").is_err());
    }

    #[test]
    fn rejects_windows_path_aliases_on_every_platform() {
        for path in [
            "C:/video.mp4",
            "media\\video.mp4",
            "media/CON.mp4",
            "media/a./x.mp4",
            "media/x.mp4:stream",
            "media//x.mp4",
            "media/../x.mp4",
        ] {
            assert!(!is_safe_relative_path(path), "{path}");
        }
    }

    #[test]
    fn replaces_same_id_without_duplicate_versions_and_keeps_original_on_failure() {
        let scratch = Scratch::new();
        let source = scratch.0.join("source");
        let catalog = scratch.0.join("catalog");
        fixture(&source);
        import_into(&catalog, &source).unwrap();
        let mut manifest = valid_manifest();
        manifest.version = "2.0.0".into();
        fs::write(
            source.join("codex-theme.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        import_into(&catalog, &source).unwrap();
        assert_eq!(
            read_manifest(&catalog.join("sample/codex-theme.json"))
                .unwrap()
                .version,
            "2.0.0"
        );
        fs::remove_file(source.join("media/startup.mp4")).unwrap();
        assert!(import_into(&catalog, &source).is_err());
        assert_eq!(
            fs::read(catalog.join("sample/media/startup.mp4")).unwrap(),
            b"video"
        );
        assert_eq!(fs::read_dir(catalog).unwrap().count(), 1);
    }

    #[test]
    fn limits_media_size_before_copy() {
        let scratch = Scratch::new();
        fixture(&scratch.0);
        fs::OpenOptions::new()
            .write(true)
            .open(scratch.0.join("media/startup.mp4"))
            .unwrap()
            .set_len(MAX_VIDEO_BYTES + 1)
            .unwrap();
        assert!(validate_files(&scratch.0, &valid_manifest()).is_err());
    }

    #[test]
    fn extracts_github_root_and_ignores_executables() {
        let scratch = Scratch::new();
        let json = serde_json::to_vec(&valid_manifest()).unwrap();
        let bytes = archive(&[
            ("theme-main/codex-theme.json", &json),
            ("theme-main/media/startup.mp4", b"video"),
            ("theme-main/install.sh", b"exit 99"),
        ]);
        extract_zip(&bytes, &scratch.0).unwrap();
        assert!(find_manifest(&scratch.0).is_ok());
        assert!(!scratch.0.join("install.sh").exists());
    }

    #[test]
    fn rejects_archive_escape_multiple_roots_and_nested_manifest() {
        for entries in [
            vec![("theme-main/../../outside.mp4", b"bad".as_slice())],
            vec![
                ("one/video.mp4", b"a".as_slice()),
                ("two/video.mp4", b"b".as_slice()),
            ],
        ] {
            let scratch = Scratch::new();
            assert!(extract_zip(&archive(&entries), &scratch.0).is_err());
        }
        let scratch = Scratch::new();
        extract_zip(
            &archive(&[("repo/sub/codex-theme.json", b"{}")]),
            &scratch.0,
        )
        .unwrap();
        assert!(find_manifest(&scratch.0).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_local_media_symlinks() {
        let scratch = Scratch::new();
        fixture(&scratch.0);
        let video = scratch.0.join("media/startup.mp4");
        fs::remove_file(&video).unwrap();
        std::os::unix::fs::symlink("poster.png", &video).unwrap();
        assert!(validate_files(&scratch.0, &valid_manifest()).is_err());
    }

    #[test]
    fn backup_preserves_exact_bytes_and_restores_absent_config() {
        let scratch = Scratch::new();
        let path = scratch.0.join("launcher.json");
        let json = serde_json::json!({
            "version": 2, "createdAt": 1, "activeTheme": null, "enabled": false,
            "launcherConfigPresent": true, "launcherConfig": b"{ \"legacy\": true }\n".to_vec()
        });
        let mut backup: LauncherBackup = serde_json::from_value(json).unwrap();
        restore_config(&path, &backup).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"{ \"legacy\": true }\n");
        backup.launcher_config = None;
        backup.launcher_config_present = false;
        restore_config(&path, &backup).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn accepts_legacy_object_backup() {
        let backup: LauncherBackup = serde_json::from_value(serde_json::json!({
            "version": 1, "createdAt": 1, "activeTheme": null, "enabled": false,
            "launcherConfig": { "legacy": true }
        }))
        .unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(backup.launcher_config.as_ref().unwrap()).unwrap();
        assert_eq!(value["legacy"], true);
    }

    #[test]
    fn preview_uses_temporary_config_without_changing_saved_config() {
        let scratch = Scratch::new();
        let installed = scratch.0.join("launcher.json");
        fs::write(&installed, b"original").unwrap();
        let preview =
            isolated_preview_config(&serde_json::json!({"AutoReplaceEntrypoints":false})).unwrap();
        assert!(preview.starts_with(std::env::temp_dir()));
        assert_ne!(preview, installed);
        assert_eq!(fs::read(installed).unwrap(), b"original");
        fs::remove_dir_all(preview.parent().unwrap()).unwrap();
    }

    #[test]
    fn native_bundle_copy_preserves_executable_and_nested_resources() {
        let scratch = Scratch::new();
        let source = scratch.0.join("source");
        fs::create_dir_all(source.join("Contents/MacOS")).unwrap();
        fs::write(source.join("Contents/MacOS/player"), b"native").unwrap();
        copy_native_bundle(&source, &scratch.0.join("native/engine")).unwrap();
        assert_eq!(
            fs::read(scratch.0.join("native/engine/Contents/MacOS/player")).unwrap(),
            b"native"
        );
    }

    #[test]
    fn bundled_manifest_and_media_are_valid() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let manifest = read_manifest(&root.join("codex-theme.json")).unwrap();
        validate_files(&root, &manifest).unwrap();
    }
}
