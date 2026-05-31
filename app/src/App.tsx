import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import ChapterWorkbench from "./components/ChapterWorkbench";
import Reports from "./components/Reports";
import MemoryEditor from "./components/MemoryEditor";
import SettingsPanel from "./components/SettingsPanel";
import "./App.css";

type Tab = "workbench" | "reports" | "memory" | "settings";

const NAV: { key: Tab; label: string; icon: string }[] = [
  { key: "workbench", label: "章节工作台", icon: "✍️" },
  { key: "reports", label: "报告", icon: "📋" },
  { key: "memory", label: "记忆库", icon: "🧠" },
  { key: "settings", label: "设置", icon: "⚙️" },
];

export default function App() {
  const [tab, setTab] = useState<Tab>("workbench");
  const [chapters, setChapters] = useState<string[]>([]);
  const [currentChapter, setCurrentChapter] = useState("");
  const [root, setRoot] = useState("");
  const [error, setError] = useState("");

  const refreshChapters = useCallback(async () => {
    try {
      const list = await api.listChapters();
      setChapters(list);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    api.getPipelineRoot().then(setRoot).catch((e) => setError(String(e)));
    refreshChapters();
  }, [refreshChapters]);

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="brand">
          <div className="brand-title">小说写作台</div>
          <div className="brand-sub">本地大模型 · 结构化记忆</div>
        </div>
        {NAV.map((n) => (
          <button
            key={n.key}
            className={tab === n.key ? "nav active" : "nav"}
            onClick={() => setTab(n.key)}
          >
            <span className="nav-icon">{n.icon}</span>
            {n.label}
          </button>
        ))}
        <div className="sidebar-foot">
          <div className="muted small">章节数：{chapters.length}</div>
        </div>
      </nav>

      <main className="content">
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
      </main>
    </div>
  );
}
