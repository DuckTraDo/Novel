import { useEffect, useState } from "react";
import { api } from "../api";
import { useUi } from "../i18n";
import { Banner, Spinner } from "./common";

type Bi = { zh: string; en: string };

interface FileDef {
  rel: string;
  label: Bi;
  group: "core" | "auto";
  note?: Bi;
}

const FILES: FileDef[] = [
  {
    rel: "memory/story_bible.yaml",
    label: { zh: "故事圣经", en: "Story Bible" },
    group: "core",
    note: { zh: "世界观、主题、写作规则、禁止模式", en: "World rules, themes, writing rules, forbidden patterns" },
  },
  {
    rel: "memory/characters.yaml",
    label: { zh: "角色档案", en: "Characters" },
    group: "core",
    note: { zh: "人物状态、knows、secrets、关系", en: "Status, knows, secrets, relationships" },
  },
  {
    rel: "outlines/book_outline.yaml",
    label: { zh: "全书大纲", en: "Book Outline" },
    group: "core",
    note: { zh: "全书方向与章节规划", en: "Overall direction and chapter plan" },
  },
  {
    rel: "memory/foreshadowing.yaml",
    label: { zh: "伏笔", en: "Foreshadowing" },
    group: "core",
    note: { zh: "active / resolved 两态", en: "active / resolved states" },
  },
  {
    rel: "memory/style_bank.jsonl",
    label: { zh: "风格库", en: "Style Bank" },
    group: "core",
    note: { zh: "每行一条 {id, text}", en: "One {id, text} per line" },
  },
  {
    rel: "config.yaml",
    label: { zh: "流水线配置", en: "Pipeline Config" },
    group: "core",
    note: { zh: "生成参数与默认风格规则", en: "Generation params and default style rules" },
  },
  {
    rel: "memory/chapter_summaries.jsonl",
    label: { zh: "章节摘要", en: "Chapter Summaries" },
    group: "auto",
  },
  { rel: "memory/events.jsonl", label: { zh: "事件流水", en: "Events" }, group: "auto" },
  { rel: "memory/timeline.jsonl", label: { zh: "时间线", en: "Timeline" }, group: "auto" },
  { rel: "memory/relationships.json", label: { zh: "关系图", en: "Relationships" }, group: "auto" },
];

export default function MemoryEditor() {
  const { t, lang } = useUi();
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
      setBanner({ kind: "ok", msg: `${active.label[lang]} ${t("me.savedSuffix")}` });
    } catch (e) {
      setBanner({ kind: "err", msg: String(e) });
    } finally {
      setBusy(false);
    }
  }

  const groups: { key: "core" | "auto"; label: string }[] = [
    { key: "core", label: t("me.groupCore") },
    { key: "auto", label: t("me.groupAuto") },
  ];

  return (
    <div className="memory">
      <aside className="memory-list">
        {groups.map((g) => (
          <div key={g.key} className="memory-group">
            <div className="memory-group-title">{g.label}</div>
            {FILES.filter((f) => f.group === g.key).map((f) => (
              <button
                key={f.rel}
                className={f.rel === active.rel ? "memory-item active" : "memory-item"}
                onClick={() => {
                  if (dirty && !window.confirm(t("me.switchConfirm"))) return;
                  setActive(f);
                }}
              >
                {f.label[lang]}
              </button>
            ))}
          </div>
        ))}
      </aside>

      <div className="memory-edit">
        <div className="panel-head">
          <h2>
            {active.label[lang]} <span className="path">{active.rel}</span>
          </h2>
          <div className="panel-actions">
            <button className="ghost" onClick={() => load(active)} disabled={busy}>
              {t("me.reload")}
            </button>
            <button className="primary" onClick={onSave} disabled={busy || !dirty}>
              {busy ? <Spinner /> : null} {t("me.save")}
              {dirty ? " *" : ""}
            </button>
          </div>
        </div>
        {active.note ? <div className="memory-note">{active.note[lang]}</div> : null}
        <textarea
          className="code"
          spellCheck={false}
          value={content}
          onChange={(e) => {
            setContent(e.target.value);
            setDirty(true);
          }}
        />
        <div className="memory-hint">{t("me.hint")}</div>
        {banner ? <Banner kind={banner.kind}>{banner.msg}</Banner> : null}
      </div>
    </div>
  );
}
