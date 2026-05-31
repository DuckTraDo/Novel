import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { useUi, Lang, Theme } from "./i18n";
import logo from "./assets/logo.png";
import ChapterWorkbench from "./components/ChapterWorkbench";
import Reports from "./components/Reports";
import MemoryEditor from "./components/MemoryEditor";
import SettingsPanel from "./components/SettingsPanel";
import Guide from "./components/Guide";
import "./App.css";

type Tab = "workbench" | "reports" | "memory" | "settings" | "guide";

const NAV: { key: Tab; labelKey: string; icon: string }[] = [
  { key: "workbench", labelKey: "nav.workbench", icon: "✍️" },
  { key: "reports", labelKey: "nav.reports", icon: "📋" },
  { key: "memory", labelKey: "nav.memory", icon: "🧠" },
  { key: "settings", labelKey: "nav.settings", icon: "⚙️" },
  { key: "guide", labelKey: "nav.guide", icon: "📖" },
];

const GITHUB_URL = "https://github.com/DuckTraDo";

function GithubMark() {
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" fill="currentColor">
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8z" />
    </svg>
  );
}

const THEMES: { key: Theme; icon: string; labelKey: string }[] = [
  { key: "light", icon: "☀", labelKey: "theme.light" },
  { key: "dark", icon: "☾", labelKey: "theme.dark" },
  { key: "warm", icon: "◑", labelKey: "theme.warm" },
];

export default function App() {
  const { t, lang, setLang, theme, setTheme } = useUi();
  const [tab, setTab] = useState<Tab>("workbench");
  const [chapters, setChapters] = useState<string[]>([]);
  const [currentChapter, setCurrentChapter] = useState("");
  const [root, setRoot] = useState("");
  const [error, setError] = useState("");

  const refreshChapters = useCallback(async () => {
    try {
      setChapters(await api.listChapters());
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    api.getPipelineRoot().then(setRoot).catch((e) => setError(String(e)));
    refreshChapters();
  }, [refreshChapters]);

  const currentLabel = NAV.find((n) => n.key === tab)?.labelKey ?? "";

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="brand">
          <img className="brand-logo" src={logo} alt="Sodarie Novel" />
          <div>
            <div className="brand-title">Sodarie Novel</div>
            <div className="brand-sub">{t("brand.subtitle")}</div>
          </div>
        </div>

        <div className="nav-list">
          {NAV.map((n) => (
            <button
              key={n.key}
              className={tab === n.key ? "nav active" : "nav"}
              onClick={() => setTab(n.key)}
            >
              <span className="nav-icon">{n.icon}</span>
              {t(n.labelKey)}
            </button>
          ))}
        </div>

        <div className="sidebar-foot">
          <div className="chapters-count">
            {t("sidebar.chapters")} · {chapters.length}
          </div>
          <button className="madeby" onClick={() => api.openUrl(GITHUB_URL)}>
            <GithubMark />
            <span className="madeby-text">
              <span className="madeby-label">{t("madeBy")}</span>
              <span className="madeby-name">DuckTraDo</span>
            </span>
            <span className="madeby-arrow">↗</span>
          </button>
        </div>
      </nav>

      <main className="content">
        <header className="topbar">
          <h1 className="topbar-title">{t(currentLabel)}</h1>
          <div className="topbar-controls">
            <div className="seg" role="group" aria-label={t("topbar.theme")}>
              {THEMES.map((th) => (
                <button
                  key={th.key}
                  className={theme === th.key ? "seg-btn active" : "seg-btn"}
                  title={t(th.labelKey)}
                  onClick={() => setTheme(th.key)}
                >
                  <span className="seg-icon">{th.icon}</span>
                </button>
              ))}
            </div>
            <div className="seg" role="group" aria-label={t("topbar.lang")}>
              {(["zh", "en"] as Lang[]).map((l) => (
                <button
                  key={l}
                  className={lang === l ? "seg-btn active" : "seg-btn"}
                  onClick={() => setLang(l)}
                >
                  {l === "zh" ? "中" : "EN"}
                </button>
              ))}
            </div>
          </div>
        </header>

        <div className="page">
          {error ? <div className="banner banner-err">{error}</div> : null}

          {tab === "workbench" && (
            <ChapterWorkbench
              chapters={chapters}
              currentChapter={currentChapter}
              setCurrentChapter={setCurrentChapter}
              refreshChapters={refreshChapters}
            />
          )}
          {tab === "reports" && (
            <Reports
              chapters={chapters}
              currentChapter={currentChapter}
              setCurrentChapter={setCurrentChapter}
            />
          )}
          {tab === "memory" && <MemoryEditor />}
          {tab === "settings" && <SettingsPanel root={root} />}
          {tab === "guide" && <Guide />}
        </div>
      </main>
    </div>
  );
}
