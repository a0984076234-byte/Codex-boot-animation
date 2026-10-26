import React, { useEffect, useMemo, useState } from "react";
import ReactDOM from "react-dom/client";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Activity,
  ArchiveRestore,
  Check,
  ChevronRight,
  Download,
  FolderOpen,
  Github,
  Languages,
  LayoutDashboard,
  MonitorPlay,
  Palette,
  Play,
  Power,
  RefreshCw,
  Settings2,
  ShieldCheck,
  Sparkles,
  Trash2,
  Upload,
  X
} from "lucide-react";
import "./styles.css";

type Theme = {
  id: string;
  name: string;
  version: string;
  description: string;
  license: string;
  source: string;
  videoPath: string;
  posterPath?: string;
  timeline: {
    holdAt: number;
    transitionStart: number;
    transitionEnd: number;
    maxWaitSeconds: number;
    volume: number;
    screenFrames: Array<{ time: number; x: number; y: number; width: number; height: number }>;
  };
};

type Timeline = Theme["timeline"];

type LauncherStatus = {
  platform: string;
  enabled: boolean;
  activeTheme?: string;
  nativeAvailable: boolean;
  message: string;
  dataPath: string;
  backupAvailable: boolean;
};

type Locale = "zh" | "en";
type View = "overview" | "themes" | "settings";

const copy = {
  zh: {
    appName: "Codex Boot Animation",
    subtitle: "启动动画控制中心",
    overview: "总览",
    themes: "主题库",
    settings: "设置",
    activeTheme: "当前主题",
    noTheme: "尚未选择主题",
    platform: "运行平台",
    integration: "启动接管",
    enabled: "已启用",
    disabled: "未启用",
    preview: "预览",
    enable: "启用启动动画",
    disable: "停用启动动画",
    backup: "创建备份",
    restore: "恢复备份",
    importLocal: "导入本地主题",
    importGithub: "从 GitHub 导入",
    refresh: "刷新状态",
    addTheme: "添加主题",
    themeSource: "主题来源",
    license: "许可",
    remove: "删除主题",
    useTheme: "设为当前主题",
    current: "当前使用",
    emptyTitle: "先添加一个启动主题",
    emptyBody: "导入本地主题目录，或粘贴公开 GitHub 仓库地址。",
    safeTitle: "离线优先，明确启用",
    safeBody: "主题只读取声明式清单和媒体文件。预览不会改写系统入口。",
    githubPlaceholder: "https://github.com/owner/codex-theme",
    cancel: "取消",
    close: "关闭",
    importAction: "导入",
    diagnostics: "诊断",
    runtime: "运行时",
    appData: "用户数据由本地控制中心管理",
    nativeReady: "原生引擎可用",
    nativeMissing: "等待原生引擎",
    version: "版本",
    manifest: "codex-theme.json",
    security: "安全检查",
    securityBody: "公开仓库导入不会执行脚本，也不会读取 GitHub 凭据。",
    actionDone: "操作已完成",
    actionFailed: "操作失败",
    saveTimeline: "保存时间线",
    timelineSaved: "时间线已保存",
    frameTime: "关键帧时间",
    frameX: "X",
    frameY: "Y",
    frameWidth: "宽度",
    frameHeight: "高度",
    addFrame: "添加关键帧",
    removeFrame: "删除关键帧",
    chooseFolder: "选择主题目录",
    about: "关于",
    aboutBody: "社区独立项目，与 OpenAI 无隶属关系，不修改官方 Codex 应用。",
    timeline: "时间线",
    holdAt: "停留点（秒）",
    transitionStart: "过渡开始（秒）",
    transitionEnd: "过渡结束（秒）",
    maxWait: "最长等待（秒）",
    volume: "音量",
    themeCount: "个主题",
    disableHint: "恢复可识别的原始启动入口",
    enableHint: "自动备份后启用，仅影响 Codex 启动",
    backupHint: "保存当前启动器状态",
    restoreHint: "恢复至最近一次保存的状态",
    lockedTheme: "启用期间请先停用，再切换、删除或修改当前主题。",
    enableConfirm: "启用后将自动备份并接管可识别的 Codex 启动入口。是否继续？",
    restoreConfirm: "恢复最近备份会替换当前启动器配置和集成状态。是否继续？",
    loading: "正在读取状态",
    processing: "正在处理",
    statusEnabled: "Codex 启动动画已启用",
    statusDisabled: "尚未接管 Codex 启动入口",
    videoUnsupported: "此系统无法在控制中心播放该视频，请使用原生预览。",
    language: "语言",
    backupState: "可恢复备份",
    available: "可用",
    unavailable: "暂无",
    dataDirectory: "用户数据目录"
  },
  en: {
    appName: "Codex Boot Animation",
    subtitle: "Startup animation control center",
    overview: "Overview",
    themes: "Themes",
    settings: "Settings",
    activeTheme: "Active theme",
    noTheme: "No theme selected",
    platform: "Platform",
    integration: "Launch integration",
    enabled: "Enabled",
    disabled: "Disabled",
    preview: "Preview",
    enable: "Enable startup animation",
    disable: "Disable startup animation",
    backup: "Create backup",
    restore: "Restore backup",
    importLocal: "Import local theme",
    importGithub: "Import from GitHub",
    refresh: "Refresh status",
    addTheme: "Add theme",
    themeSource: "Theme source",
    license: "License",
    remove: "Remove theme",
    useTheme: "Use this theme",
    current: "In use",
    emptyTitle: "Add a startup theme",
    emptyBody: "Import a local theme directory or paste a public GitHub repository.",
    safeTitle: "Offline-first, explicit enable",
    safeBody: "Themes are declarative media only. Preview never changes system entry points.",
    githubPlaceholder: "https://github.com/owner/codex-theme",
    cancel: "Cancel",
    close: "Close",
    importAction: "Import",
    diagnostics: "Diagnostics",
    runtime: "Runtime",
    appData: "User data is managed locally",
    nativeReady: "Native engine ready",
    nativeMissing: "Native engine pending",
    version: "Version",
    manifest: "codex-theme.json",
    security: "Security",
    securityBody: "Public repository imports never execute scripts or read GitHub credentials.",
    actionDone: "Action completed",
    actionFailed: "Action failed",
    saveTimeline: "Save timeline",
    timelineSaved: "Timeline saved",
    frameTime: "Frame time",
    frameX: "X",
    frameY: "Y",
    frameWidth: "Width",
    frameHeight: "Height",
    addFrame: "Add keyframe",
    removeFrame: "Remove keyframe",
    chooseFolder: "Choose theme folder",
    language: "Language",
    about: "About",
    aboutBody: "Independent community project, not affiliated with OpenAI. Does not modify the official Codex app.",
    timeline: "Timeline",
    holdAt: "Hold at (s)",
    transitionStart: "Transition start (s)",
    transitionEnd: "Transition end (s)",
    maxWait: "Max wait (s)",
    volume: "Volume",
    themeCount: "themes",
    disableHint: "Restore recognized original entry points",
    enableHint: "Back up, then enable for Codex launches only",
    backupHint: "Save the current launcher state",
    restoreHint: "Return to the latest saved state",
    lockedTheme: "Disable integration before switching, removing, or editing the active theme.",
    enableConfirm: "This backs up and takes over recognized Codex launch entry points. Continue?",
    restoreConfirm: "Restore the latest backup, replacing the current launcher configuration and integration state?",
    loading: "Reading status",
    processing: "Working",
    statusEnabled: "Codex startup animation is enabled",
    statusDisabled: "Codex launch integration is not enabled",
    videoUnsupported: "This system cannot play this video in the control center. Use native preview instead.",
    backupState: "Restorable backup",
    available: "Available",
    unavailable: "None",
    dataDirectory: "User data directory"
  }
} as const;
type Copy = { [Key in keyof typeof copy.zh]: string };

function App() {
  const [locale, setLocale] = useState<Locale>(() => {
    try { return localStorage.getItem("codex-boot-locale") === "en" ? "en" : "zh"; }
    catch { return "zh"; }
  });
  const [view, setView] = useState<View>("overview");
  const [themes, setThemes] = useState<Theme[]>([]);
  const [status, setStatus] = useState<LauncherStatus | null>(null);
  const [selected, setSelected] = useState<Theme | null>(null);
  const [githubUrl, setGithubUrl] = useState("");
  const [showGithub, setShowGithub] = useState(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<{ kind: "success" | "error"; text: string } | null>(null);
  const t = copy[locale] as Copy;

  const load = async () => {
    try {
      const [nextThemes, nextStatus] = await Promise.all([
        invoke<Theme[]>("list_themes"),
        invoke<LauncherStatus>("launcher_status")
      ]);
      setThemes(nextThemes);
      setStatus(nextStatus);
      if (selected) {
        setSelected(nextThemes.find((theme) => theme.id === selected.id) ?? nextThemes[0] ?? null);
      } else {
        setSelected(nextThemes[0] ?? null);
      }
    } catch (error) {
      setNotice({ kind: "error", text: String(error) });
    }
  };

  useEffect(() => {
    void load();
  }, []);

  useEffect(() => {
    document.documentElement.lang = locale === "zh" ? "zh-CN" : "en";
    try { localStorage.setItem("codex-boot-locale", locale); } catch { /* Storage may be disabled. */ }
  }, [locale]);

  const runAction = async (action: string, themeId?: string) => {
    if (action === "enable" && !window.confirm(t.enableConfirm)) return;
    if (action === "restore" && !window.confirm(t.restoreConfirm)) return;
    setBusy(true);
    try {
      const request = themeId ? { action, themeId } : { action };
      const nextStatus = await invoke<LauncherStatus>("launcher_action", { request });
      setStatus(nextStatus);
      setNotice({ kind: "success", text: t.actionDone });
    } catch (error) {
      setNotice({ kind: "error", text: `${t.actionFailed}: ${String(error)}` });
    } finally {
      setBusy(false);
    }
  };

  const importLocal = async () => {
    const path = await open({ directory: true, multiple: false, title: t.chooseFolder });
    if (!path || Array.isArray(path)) return;
    setBusy(true);
    try {
      await invoke<Theme>("import_local_theme", { path });
      await load();
      setNotice({ kind: "success", text: t.actionDone });
      setView("themes");
    } catch (error) {
      setNotice({ kind: "error", text: `${t.actionFailed}: ${String(error)}` });
    } finally {
      setBusy(false);
    }
  };

  const importGithub = async () => {
    if (!githubUrl.trim()) return;
    setBusy(true);
    try {
      await invoke<Theme>("import_github_theme", { url: githubUrl.trim() });
      setGithubUrl("");
      setShowGithub(false);
      await load();
      setNotice({ kind: "success", text: t.actionDone });
      setView("themes");
    } catch (error) {
      setNotice({ kind: "error", text: `${t.actionFailed}: ${String(error)}` });
    } finally {
      setBusy(false);
    }
  };

  const chooseTheme = async (theme: Theme) => {
    setBusy(true);
    try {
      await invoke("set_active_theme", { id: theme.id });
      setSelected(theme);
      await load();
      setNotice({ kind: "success", text: t.actionDone });
    } catch (error) {
      setNotice({ kind: "error", text: `${t.actionFailed}: ${String(error)}` });
    } finally {
      setBusy(false);
    }
  };

  const updateTimeline = async (themeId: string, timeline: Timeline) => {
    setBusy(true);
    try {
      const updated = await invoke<Theme>("update_theme_timeline", {
        request: { themeId, timeline }
      });
      setThemes((current) => current.map((theme) => theme.id === updated.id ? updated : theme));
      setSelected(updated);
      setNotice({ kind: "success", text: t.timelineSaved });
    } catch (error) {
      setNotice({ kind: "error", text: `${t.actionFailed}: ${String(error)}` });
    } finally {
      setBusy(false);
    }
  };

  const removeTheme = async (theme: Theme) => {
    if (!window.confirm(`${t.remove} “${theme.name}”?`)) return;
    setBusy(true);
    try {
      await invoke("remove_theme", { id: theme.id });
      await load();
    } catch (error) {
      setNotice({ kind: "error", text: `${t.actionFailed}: ${String(error)}` });
    } finally {
      setBusy(false);
    }
  };

  const activeTheme = useMemo(
    () => themes.find((theme) => theme.id === status?.activeTheme) ?? selected ?? themes[0] ?? null,
    [status, themes, selected]
  );

  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark"><Sparkles size={18} /></div>
          <div>
            <strong>{t.appName}</strong>
            <span>{t.subtitle}</span>
          </div>
        </div>
        <nav className="nav">
          <NavItem active={view === "overview"} icon={<LayoutDashboard size={17} />} label={t.overview} onClick={() => setView("overview")} />
          <NavItem active={view === "themes"} icon={<Palette size={17} />} label={t.themes} onClick={() => setView("themes")} />
          <NavItem active={view === "settings"} icon={<Settings2 size={17} />} label={t.settings} onClick={() => setView("settings")} />
        </nav>
        <div className="sidebar-foot">
          <div className="mini-status"><ShieldCheck size={16} /><span>{t.safeTitle}</span></div>
          <button className="link-button" onClick={() => setLocale(locale === "zh" ? "en" : "zh")}>
            <Languages size={15} /> {locale === "zh" ? "English" : "简体中文"}
          </button>
        </div>
      </aside>

      <main className="main">
        <header className="topbar">
          <div>
            <div className="eyebrow">{view === "overview" ? t.overview : view === "themes" ? t.themes : t.settings}</div>
            <h1>{view === "overview" ? t.activeTheme : view === "themes" ? t.addTheme : t.diagnostics}</h1>
          </div>
          <button className="icon-button" title={t.refresh} aria-label={t.refresh} disabled={busy} onClick={() => void load()}><RefreshCw size={18} /></button>
        </header>

        {notice && <div role="status" className={`notice ${notice.kind}`}><span>{notice.text}</span><button aria-label={t.close} onClick={() => setNotice(null)}><X size={15} /></button></div>}
        {busy && <p className="working-status" role="status">{t.processing}…</p>}

        {view === "overview" && (
          <Overview
            t={t}
            activeTheme={activeTheme}
            status={status}
            busy={busy}
            onPreview={() => void runAction("preview", activeTheme?.id)}
            onEnable={() => void runAction("enable", activeTheme?.id)}
            onDisable={() => void runAction("disable")}
            onBackup={() => void runAction("backup")}
            onRestore={() => void runAction("restore")}
            onThemes={() => setView("themes")}
          />
        )}
        {view === "themes" && (
          <ThemeLibrary
            t={t}
            themes={themes}
            selected={selected}
            busy={busy}
            status={status}
            onSelect={setSelected}
            onUse={(theme) => void chooseTheme(theme)}
            onRemove={(theme) => void removeTheme(theme)}
            onPreview={(theme) => void runAction("preview", theme.id)}
            onSaveTimeline={(theme, timeline) => void updateTimeline(theme.id, timeline)}
            onImportLocal={() => void importLocal()}
            onImportGithub={() => setShowGithub(true)}
          />
        )}
        {view === "settings" && <Settings t={t} status={status} />}
      </main>

      {showGithub && (
        <div className="modal-backdrop" onMouseDown={() => setShowGithub(false)}>
          <div className="modal" role="dialog" aria-modal="true" aria-label={t.importGithub} onKeyDown={(event) => { if (event.key === "Escape" && !busy) setShowGithub(false); }} onMouseDown={(event) => event.stopPropagation()}>
            <div className="modal-heading"><div><span className="eyebrow">{t.themeSource}</span><h2>{t.importGithub}</h2></div><button className="icon-button" onClick={() => setShowGithub(false)}><X size={18} /></button></div>
            <label className="field-label" htmlFor="github-url">GitHub URL</label>
            <input id="github-url" autoFocus value={githubUrl} onChange={(event) => setGithubUrl(event.target.value)} placeholder={t.githubPlaceholder} />
            <p className="modal-note">{t.securityBody}</p>
            <div className="modal-actions"><button className="button secondary" onClick={() => setShowGithub(false)}>{t.cancel}</button><button className="button primary" disabled={busy || !githubUrl.trim()} onClick={() => void importGithub()}><Github size={16} /> {t.importAction}</button></div>
          </div>
        </div>
      )}
    </div>
  );
}

function NavItem({ active, icon, label, onClick }: { active: boolean; icon: React.ReactNode; label: string; onClick: () => void }) {
  return <button className={`nav-item ${active ? "active" : ""}`} onClick={onClick}>{icon}<span>{label}</span>{active && <ChevronRight size={15} />}</button>;
}

function Overview({ t, activeTheme, status, busy, onPreview, onEnable, onDisable, onBackup, onRestore, onThemes }: {
  t: Copy;
  activeTheme: Theme | null | undefined;
  status: LauncherStatus | null;
  busy: boolean;
  onPreview: () => void;
  onEnable: () => void;
  onDisable: () => void;
  onBackup: () => void;
  onRestore: () => void;
  onThemes: () => void;
}) {
  return <div className="content-grid">
    <section className="hero-panel">
      <div className="hero-copy"><span className="eyebrow">{t.activeTheme}</span><h2>{activeTheme?.name ?? t.noTheme}</h2><p>{activeTheme?.description || t.emptyBody}</p><div className="hero-actions"><button className="button primary" disabled={!activeTheme || busy} onClick={onPreview}><Play size={16} /> {t.preview}</button><button className="button secondary" onClick={onThemes}><Palette size={16} /> {t.themes}</button></div></div>
      <div className="hero-art">{activeTheme?.posterPath ? <img src={convertFileSrc(activeTheme.posterPath)} alt={activeTheme.name} /> : <MonitorPlay size={54} strokeWidth={1.2} />}</div>
    </section>
    <section className="metric-row">
      <Metric label={t.platform} value={status?.platform === "macos" ? "macOS" : status?.platform === "windows" ? "Windows" : "—"} icon={<MonitorPlay size={17} />} />
      <Metric label={t.integration} value={status?.enabled ? t.enabled : t.disabled} tone={status?.enabled ? "green" : "neutral"} icon={<Power size={17} />} />
      <Metric label={t.runtime} value={status?.nativeAvailable ? t.nativeReady : t.nativeMissing} tone={status?.nativeAvailable ? "green" : "amber"} icon={<Activity size={17} />} />
    </section>
    <section className="action-panel">
      <div className="section-heading"><div><span className="eyebrow">{t.integration}</span><h3>{status ? status.enabled ? t.statusEnabled : t.statusDisabled : t.loading}</h3></div><span className={`status-dot ${status?.enabled ? "green" : "amber"}`} /></div>
      <div className="action-grid">
        {status?.enabled ? <button className="action-tile" disabled={busy} onClick={onDisable}><Power size={20} /><span>{t.disable}</span><small>{t.disableHint}</small></button> : <button className="action-tile featured" disabled={!activeTheme || busy || !status?.nativeAvailable} onClick={onEnable}><Check size={20} /><span>{t.enable}</span><small>{t.enableHint}</small></button>}
        <button className="action-tile" disabled={busy || !status} onClick={onBackup}><ArchiveRestore size={20} /><span>{t.backup}</span><small>{t.backupHint}</small></button>
        <button className="action-tile" disabled={busy || !status?.backupAvailable} onClick={onRestore}><RefreshCw size={20} /><span>{t.restore}</span><small>{t.restoreHint}</small></button>
      </div>
    </section>
    <section className="safe-panel"><ShieldCheck size={21} /><div><strong>{t.safeTitle}</strong><p>{t.safeBody}</p></div></section>
  </div>;
}

function Metric({ label, value, icon, tone = "neutral" }: { label: string; value: string; icon: React.ReactNode; tone?: string }) {
  return <div className="metric"><div className={`metric-icon ${tone}`}>{icon}</div><div><span>{label}</span><strong>{value}</strong></div></div>;
}

function ThemeLibrary({ t, themes, selected, busy, status, onSelect, onUse, onRemove, onPreview, onSaveTimeline, onImportLocal, onImportGithub }: {
  t: Copy;
  themes: Theme[];
  selected: Theme | null;
  busy: boolean;
  status: LauncherStatus | null;
  onSelect: (theme: Theme) => void;
  onUse: (theme: Theme) => void;
  onRemove: (theme: Theme) => void;
  onPreview: (theme: Theme) => void;
  onSaveTimeline: (theme: Theme, timeline: Timeline) => void;
  onImportLocal: () => void;
  onImportGithub: () => void;
}) {
  const locked = Boolean(status?.enabled && status.activeTheme === selected?.id);
  return <div className="library-layout">
    <section className="theme-list-panel"><div className="section-heading"><div><span className="eyebrow">{themes.length} {t.themeCount}</span><h3>{t.themes}</h3></div><button className="button primary compact" onClick={onImportLocal}><Upload size={15} /> {t.addTheme}</button></div><div className="import-row"><button className="button secondary compact" onClick={onImportLocal}><FolderOpen size={15} /> {t.importLocal}</button><button className="button secondary compact" onClick={onImportGithub}><Github size={15} /> {t.importGithub}</button></div>{themes.length === 0 ? <div className="empty-state"><Palette size={26} /><strong>{t.emptyTitle}</strong><p>{t.emptyBody}</p></div> : <div className="theme-list">{themes.map((theme) => <button className={`theme-row ${selected?.id === theme.id ? "selected" : ""}`} key={`${theme.id}-${theme.version}`} onClick={() => onSelect(theme)}><div className="theme-thumb">{theme.posterPath ? <img src={convertFileSrc(theme.posterPath)} alt="" /> : <MonitorPlay size={18} />}</div><div className="theme-row-copy"><strong>{theme.name}</strong><span>{theme.version} · {theme.license}</span></div>{selected?.id === theme.id && <Check size={16} />}</button>)}</div>}</section>
    <section className="theme-detail-panel">{selected ? <><div className="detail-preview">{selected.videoPath ? <video className="theme-video" src={convertFileSrc(selected.videoPath)} poster={selected.posterPath ? convertFileSrc(selected.posterPath) : undefined} controls preload="metadata" /> : <div className="preview-mark"><MonitorPlay size={42} strokeWidth={1.2} /><span>CODEx / BOOT</span></div>}<button className="preview-button" title={t.preview} aria-label={t.preview} disabled={busy} onClick={() => onPreview(selected)}><Play size={18} /></button></div><div className="detail-heading"><div><span className="eyebrow">{t.manifest}</span><h2>{selected.name}</h2></div><span className="version-pill">v{selected.version}</span></div><p className="detail-copy">{selected.description || t.emptyBody}</p><dl className="detail-meta"><div><dt>{t.license}</dt><dd>{selected.license}</dd></div><div><dt>{t.themeSource}</dt><dd>{new URL(selected.source).pathname}</dd></div></dl>{locked && <p className="locked-note">{t.lockedTheme}</p>}<TimelineEditor t={t} theme={selected} busy={busy || locked} onSave={onSaveTimeline} /><div className="detail-actions"><button className="button primary" disabled={busy || locked} onClick={() => onUse(selected)}><Check size={16} /> {t.useTheme}</button><button className="icon-button danger" title={t.remove} aria-label={t.remove} disabled={busy || locked} onClick={() => onRemove(selected)}><Trash2 size={17} /></button></div></> : <div className="empty-detail"><Sparkles size={28} /><strong>{t.emptyTitle}</strong><p>{t.emptyBody}</p></div>}</section>
  </div>;
}

function TimelineEditor({ t, theme, busy, onSave }: { t: Copy; theme: Theme; busy: boolean; onSave: (theme: Theme, timeline: Timeline) => void }) {
  const [timeline, setTimeline] = useState<Timeline>(theme.timeline);
  useEffect(() => setTimeline(theme.timeline), [theme.id, theme.version, theme.timeline]);
  const update = (key: keyof Omit<Timeline, "screenFrames">, value: string) => {
    const number = Number(value);
    setTimeline((current) => ({ ...current, [key]: Number.isFinite(number) ? number : current[key] }));
  };
  const updateFrame = (index: number, key: keyof Timeline["screenFrames"][number], value: string) => {
    const number = Number(value);
    setTimeline((current) => ({
      ...current,
      screenFrames: current.screenFrames.map((frame, frameIndex) => frameIndex === index
        ? { ...frame, [key]: Number.isFinite(number) ? number : frame[key] }
        : frame)
    }));
  };
  const addFrame = () => {
    const previous = timeline.screenFrames[timeline.screenFrames.length - 1];
    setTimeline((current) => ({
      ...current,
      screenFrames: [...current.screenFrames, { ...previous, time: previous.time + 0.1 }]
    }));
  };
  const removeFrame = (index: number) => {
    if (timeline.screenFrames.length <= 2) return;
    setTimeline((current) => ({ ...current, screenFrames: current.screenFrames.filter((_, frameIndex) => frameIndex !== index) }));
  };
  return <section className="timeline-editor">
    <div className="section-heading"><div><span className="eyebrow">{t.timeline}</span><h3>{t.manifest}</h3></div><button className="button secondary compact" disabled={busy} onClick={addFrame}><Upload size={14} /> {t.addFrame}</button></div>
    <div className="timeline-fields">
      {([
        ["holdAt", t.holdAt],
        ["transitionStart", t.transitionStart],
        ["transitionEnd", t.transitionEnd],
        ["maxWaitSeconds", t.maxWait],
        ["volume", t.volume]
      ] as const).map(([key, label]) => <label key={key}><span>{label}</span><input type="number" step="0.01" min={key === "volume" ? 0 : 0.01} max={key === "volume" ? 1 : undefined} value={timeline[key]} onChange={(event) => update(key, event.target.value)} /></label>)}
    </div>
    <div className="frame-list">
      {timeline.screenFrames.map((frame, index) => <div className="frame-row" key={index}><span className="frame-index">{index + 1}</span>{([
        ["time", t.frameTime],
        ["x", t.frameX],
        ["y", t.frameY],
        ["width", t.frameWidth],
        ["height", t.frameHeight]
      ] as const).map(([key, label]) => <label key={key}><span>{label}</span><input aria-label={label} type="number" step="0.001" min={0} max={key === "time" ? undefined : 1} value={frame[key]} onChange={(event) => updateFrame(index, key, event.target.value)} /></label>)}<button className="icon-button danger" title={t.removeFrame} disabled={busy || timeline.screenFrames.length <= 2} onClick={() => removeFrame(index)}><Trash2 size={15} /></button></div>)}
    </div>
    <button className="button primary compact" disabled={busy} onClick={() => onSave(theme, timeline)}><Check size={14} /> {t.saveTimeline}</button>
  </section>;
}

function Settings({ t, status }: { t: Copy; status: LauncherStatus | null }) {
  return <div className="settings-layout"><section className="settings-section"><div className="section-heading"><div><span className="eyebrow">{t.diagnostics}</span><h3>{t.security}</h3></div><ShieldCheck size={20} /></div><div className="setting-line"><div><strong>{t.security}</strong><p>{t.securityBody}</p></div><span className="tag green"><Check size={13} /> OK</span></div><div className="setting-line"><div><strong>{t.runtime}</strong><p>{status?.nativeAvailable ? t.nativeReady : t.nativeMissing}</p></div><span className={`tag ${status?.nativeAvailable ? "green" : "amber"}`}>{status?.platform || "—"}</span></div><div className="setting-line"><div><strong>{t.backupState}</strong><p>{status?.backupAvailable ? t.available : t.unavailable}</p></div><span className={`tag ${status?.backupAvailable ? "green" : "amber"}`}>{status?.backupAvailable ? t.available : t.unavailable}</span></div></section><section className="settings-section"><div className="section-heading"><div><span className="eyebrow">{t.about}</span><h3>{t.appName}</h3></div><Sparkles size={20} /></div><p className="about-copy">{t.aboutBody}</p><div className="setting-line"><span>{t.version}</span><strong>0.2.0</strong></div><div className="setting-line"><div><span>{t.dataDirectory}</span><p className="data-path">{status?.dataPath || "—"}</p></div><Download size={16} /></div></section></div>;
}

ReactDOM.createRoot(document.getElementById("root")!).render(<React.StrictMode><App /></React.StrictMode>);
