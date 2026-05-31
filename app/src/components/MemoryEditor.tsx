import { useEffect, useState } from "react";
import { api } from "../api";
import { Banner, Spinner } from "./common";

interface FileDef {
  rel: string;
  label: string;
  group: string;
  note?: string;
}

const FILES: FileDef[] = [
  { rel: "memory/story_bible.yaml", label: "故事圣经", group: "核心设定", note: "世界观、主题、写作规则、禁止模式" },
  { rel: "memory/characters.yaml", label: "角色档案", group: "核心设定", note: "人物状态、knows、secrets、关系" },
  { rel: "outlines/book_outline.yaml", label: "全书大纲", group: "核心设定", note: "全书方向与章节规划" },
  { rel: "memory/foreshadowing.yaml", label: "伏笔", group: "核心设定", note: "active / resolved 两态" },
  { rel: "memory/style_bank.jsonl", label: "风格库", group: "核心设定", note: "每行一条 {id, text}" },
  { rel: "config.yaml", label: "流水线配置", group: "核心设定", note: "生成参数与默认风格规则" },
  { rel: "memory/chapter_summaries.jsonl", label: "章节摘要", group: "自动维护（谨慎手改）" },
  { rel: "memory/events.jsonl", label: "事件流水", group: "自动维护（谨慎手改）" },
  { rel: "memory/timeline.jsonl", label: "时间线", group: "自动维护（谨慎手改）" },
  { rel: "memory/relationships.json", label: "关系图", group: "自动维护（谨慎手改）" },
];

export default function MemoryEditor() {
  const [active, setActive] = useState<FileDef>(FILES[0]);
  const [content, setContent] = useState("");
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState(false);
  const [banner, setBanner] = useState<{ kind: "ok" | "err" | "info"; msg: string } | null>(null);

  async function load(def: FileDef) {
    try {
      const text = await api.readMemoryFile(def.rel);
      setContent(text);
      setDirty(false);
      setBanner(null);
    } catch (e) {
      setContent("");
      setBanner({ kind: "err", msg: String(e) });
    }
  }

  useEffect(() => {
    load(active);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  async function onSave() {
    setBusy(true);
    try {
      await api.saveMemoryFile(active.rel, content);
      setDirty(false);
      setBanner({ kind: "ok", msg: `${active.label} 已保存。` });
    } catch (e) {
      setBanner({ kind: "err", msg: String(e) });
    } finally {
      setBusy(false);
    }
  }

  const groups = Array.from(new Set(FILES.map((f) => f.group)));

  return (
    <div className="memory">
      <aside className="memory-list">
        {groups.map((g) => (
          <div key={g} className="memory-group">
            <div className="memory-group-title">{g}</div>
            {FILES.filter((f) => f.group === g).map((f) => (
              <button
                key={f.rel}
                className={f.rel === active.rel ? "memory-item active" : "memory-item"}
                onClick={() => {
                  if (dirty && !window.confirm("当前文件未保存，确定切换？")) return;
                  setActive(f);
                }}
              >
                {f.label}
              </button>
            ))}
          </div>
        ))}
      </aside>

      <div className="memory-edit">
        <div className="panel-head">
          <h2>
            {active.label} <span className="path">{active.rel}</span>
          </h2>
          <div className="panel-actions">
            <button className="ghost" onClick={() => load(active)} disabled={busy}>
              重新加载
            </button>
            <button className="primary" onClick={onSave} disabled={busy || !dirty}>
              {busy ? <Spinner /> : null} 保存{dirty ? " *" : ""}
            </button>
          </div>
        </div>
        {active.note ? <div className="memory-note">{active.note}</div> : null}
        <textarea
          className="code"
          spellCheck={false}
          value={content}
          onChange={(e) => {
            setContent(e.target.value);
            setDirty(true);
          }}
        />
        <div className="memory-hint">
          直接编辑原始内容（YAML / JSON / JSONL）。保存后若格式有误，运行脚本时会报错，请按提示修正。
        </div>
        {banner ? <Banner kind={banner.kind}>{banner.msg}</Banner> : null}
      </div>
    </div>
  );
}
