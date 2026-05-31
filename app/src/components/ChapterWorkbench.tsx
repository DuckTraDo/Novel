import { useEffect, useState } from "react";
import { api, CommandResult } from "../api";
import { Banner, Field, LogBox, Spinner } from "./common";

interface Props {
  chapters: string[];
  currentChapter: string;
  setCurrentChapter: (id: string) => void;
  refreshChapters: () => Promise<void>;
}

type Busy = "" | "generate" | "consistency" | "memory" | "reset" | "save";

export default function ChapterWorkbench({
  chapters,
  currentChapter,
  setCurrentChapter,
  refreshChapters,
}: Props) {
  const [newId, setNewId] = useState("");
  const [idea, setIdea] = useState("");
  const [targetWords, setTargetWords] = useState(4000);
  const [useContext, setUseContext] = useState(true);
  const [overwrite, setOverwrite] = useState(true);
  const [includeMemory, setIncludeMemory] = useState(false);

  const [content, setContent] = useState("");
  const [dirty, setDirty] = useState(false);

  const [busy, setBusy] = useState<Busy>("");
  const [log, setLog] = useState("");
  const [banner, setBanner] = useState<{ kind: "ok" | "err" | "info"; msg: string } | null>(null);

  async function loadContent(id: string) {
    if (!id) {
      setContent("");
      setDirty(false);
      return;
    }
    try {
      const text = await api.readChapter(id);
      setContent(text);
      setDirty(false);
    } catch (e) {
      setContent("");
    }
  }

  useEffect(() => {
    loadContent(currentChapter);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentChapter]);

  function handleResult(res: CommandResult, okMsg: string) {
    setLog(res.log);
    setBanner(res.success ? { kind: "ok", msg: okMsg } : { kind: "err", msg: "执行失败，请看下方日志。" });
  }

  function nextChapterId(): string {
    let max = 0;
    for (const c of chapters) {
      const m = c.match(/(\d+)/);
      if (m) max = Math.max(max, parseInt(m[1], 10));
    }
    return "ch" + String(max + 1).padStart(3, "0");
  }

  async function onGenerate() {
    const id = (currentChapter || newId).trim();
    if (!id) {
      setBanner({ kind: "err", msg: "请先选择或新建一个章节 ID。" });
      return;
    }
    if (!idea.trim()) {
      setBanner({ kind: "err", msg: "请填写本章 idea。" });
      return;
    }
    setBusy("generate");
    setBanner({ kind: "info", msg: "正在调用本地模型生成整章，请稍候……" });
    try {
      const res = await api.generateChapter(id, idea.trim(), targetWords, useContext, overwrite);
      handleResult(res, `章节 ${id} 已生成。`);
      if (res.success) {
        await refreshChapters();
        setCurrentChapter(id);
        await loadContent(id);
      }
    } catch (e) {
      setBanner({ kind: "err", msg: String(e) });
    } finally {
      setBusy("");
    }
  }

  async function onSave() {
    if (!currentChapter) return;
    setBusy("save");
    try {
      await api.saveChapter(currentChapter, content);
      setDirty(false);
      setBanner({ kind: "ok", msg: "正文已保存。" });
    } catch (e) {
      setBanner({ kind: "err", msg: String(e) });
    } finally {
      setBusy("");
    }
  }

  async function onCheck() {
    if (!currentChapter) return;
    setBusy("consistency");
    setBanner({ kind: "info", msg: "正在进行一致性审查……" });
    try {
      const res = await api.checkConsistency(currentChapter);
      handleResult(res, "一致性审查完成，可到「报告」查看。");
    } catch (e) {
      setBanner({ kind: "err", msg: String(e) });
    } finally {
      setBusy("");
    }
  }

  async function onUpdateMemory() {
    if (!currentChapter) return;
    setBusy("memory");
    setBanner({ kind: "info", msg: "正在抽取并更新长期记忆……" });
    try {
      const res = await api.updateMemory(currentChapter);
      handleResult(res, "记忆更新完成。");
    } catch (e) {
      setBanner({ kind: "err", msg: String(e) });
    } finally {
      setBusy("");
    }
  }

  async function onReset() {
    if (!currentChapter) return;
    const warn = includeMemory
      ? `确定重置 ${currentChapter}？会删除正文、报告，并从摘要/事件/时间线中过滤该章记录。`
      : `确定重置 ${currentChapter}？会删除正文和报告（不动长期记忆）。`;
    if (!window.confirm(warn)) return;
    setBusy("reset");
    try {
      const res = await api.resetChapter(currentChapter, includeMemory);
      handleResult(res, `${currentChapter} 已重置。`);
      if (res.success) {
        await refreshChapters();
        setContent("");
      }
    } catch (e) {
      setBanner({ kind: "err", msg: String(e) });
    } finally {
      setBusy("");
    }
  }

  const running = busy !== "";

  return (
    <div className="workbench">
      <div className="panel">
        <h2>章节</h2>
        <div className="row">
          <Field label="选择章节">
            <select
              value={currentChapter}
              onChange={(e) => setCurrentChapter(e.target.value)}
            >
              <option value="">（未选择 / 新建）</option>
              {chapters.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </Field>
          <Field label="新建章节 ID" hint="留空选择时用此 ID 生成">
            <div className="inline">
              <input
                value={newId}
                placeholder="如 ch001"
                onChange={(e) => setNewId(e.target.value)}
              />
              <button
                type="button"
                className="ghost"
                onClick={() => setNewId(nextChapterId())}
              >
                自动编号
              </button>
            </div>
          </Field>
        </div>

        <Field label="本章 idea（一句话方向，最高优先级）">
          <textarea
            className="idea"
            value={idea}
            placeholder="例：第一章，主角回到故乡，发现父亲留下的一封信，决定调查多年前的旧事。"
            onChange={(e) => setIdea(e.target.value)}
          />
        </Field>

        <div className="row">
          <Field label="目标字数">
            <input
              type="number"
              value={targetWords}
              min={500}
              step={500}
              onChange={(e) => setTargetWords(parseInt(e.target.value || "0", 10))}
            />
          </Field>
          <div className="checks">
            <label>
              <input
                type="checkbox"
                checked={useContext}
                onChange={(e) => setUseContext(e.target.checked)}
              />
              使用长期记忆
            </label>
            <label>
              <input
                type="checkbox"
                checked={overwrite}
                onChange={(e) => setOverwrite(e.target.checked)}
              />
              覆盖已有正文
            </label>
          </div>
        </div>

        <div className="actions">
          <button className="primary" onClick={onGenerate} disabled={running}>
            {busy === "generate" ? <Spinner /> : null} 生成 / 重新生成
          </button>
        </div>
      </div>

      <div className="panel">
        <div className="panel-head">
          <h2>正文 {currentChapter ? `· ${currentChapter}` : ""}</h2>
          <div className="panel-actions">
            <button onClick={onSave} disabled={!currentChapter || running || !dirty}>
              {busy === "save" ? <Spinner /> : null} 保存正文{dirty ? " *" : ""}
            </button>
          </div>
        </div>
        <textarea
          className="chapter-body"
          value={content}
          placeholder={currentChapter ? "（本章暂无正文，点击上方生成）" : "请选择或生成一个章节"}
          onChange={(e) => {
            setContent(e.target.value);
            setDirty(true);
          }}
          disabled={!currentChapter}
        />
        <div className="wordcount">{content.length} 字</div>

        <div className="actions wrap">
          <button onClick={onCheck} disabled={!currentChapter || running}>
            {busy === "consistency" ? <Spinner /> : null} 一致性检查
          </button>
          <button onClick={onUpdateMemory} disabled={!currentChapter || running}>
            {busy === "memory" ? <Spinner /> : null} 更新记忆
          </button>
          <span className="spacer" />
          <label className="danger-check">
            <input
              type="checkbox"
              checked={includeMemory}
              onChange={(e) => setIncludeMemory(e.target.checked)}
            />
            连带记忆
          </label>
          <button className="danger" onClick={onReset} disabled={!currentChapter || running}>
            {busy === "reset" ? <Spinner /> : null} 重置本章
          </button>
        </div>
      </div>

      {banner ? <Banner kind={banner.kind}>{banner.msg}</Banner> : null}
      <LogBox log={log} />
    </div>
  );
}
