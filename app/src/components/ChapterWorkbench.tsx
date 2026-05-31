import { useEffect, useState } from "react";
import { api, CommandResult } from "../api";
import { useT } from "../i18n";
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
  const t = useT();
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
    } catch {
      setContent("");
    }
  }

  useEffect(() => {
    loadContent(currentChapter);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentChapter]);

  function handleResult(res: CommandResult, okMsg: string) {
    setLog(res.log);
    setBanner(res.success ? { kind: "ok", msg: okMsg } : { kind: "err", msg: t("wb.execFail") });
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
      setBanner({ kind: "err", msg: t("wb.needChapter") });
      return;
    }
    if (!idea.trim()) {
      setBanner({ kind: "err", msg: t("wb.needIdea") });
      return;
    }
    setBusy("generate");
    setBanner({ kind: "info", msg: t("wb.generating") });
    try {
      const res = await api.generateChapter(id, idea.trim(), targetWords, useContext, overwrite);
      handleResult(res, t("wb.genOk", { id }));
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
      setBanner({ kind: "ok", msg: t("wb.saved") });
    } catch (e) {
      setBanner({ kind: "err", msg: String(e) });
    } finally {
      setBusy("");
    }
  }

  async function onCheck() {
    if (!currentChapter) return;
    setBusy("consistency");
    setBanner({ kind: "info", msg: t("wb.checking") });
    try {
      const res = await api.checkConsistency(currentChapter);
      handleResult(res, t("wb.checkOk"));
    } catch (e) {
      setBanner({ kind: "err", msg: String(e) });
    } finally {
      setBusy("");
    }
  }

  async function onUpdateMemory() {
    if (!currentChapter) return;
    setBusy("memory");
    setBanner({ kind: "info", msg: t("wb.memRunning") });
    try {
      const res = await api.updateMemory(currentChapter);
      handleResult(res, t("wb.memOk"));
    } catch (e) {
      setBanner({ kind: "err", msg: String(e) });
    } finally {
      setBusy("");
    }
  }

  async function onReset() {
    if (!currentChapter) return;
    const warn = includeMemory
      ? t("wb.resetConfirmMem", { id: currentChapter })
      : t("wb.resetConfirmNo", { id: currentChapter });
    if (!window.confirm(warn)) return;
    setBusy("reset");
    try {
      const res = await api.resetChapter(currentChapter, includeMemory);
      handleResult(res, t("wb.resetOk", { id: currentChapter }));
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
        <h2>{t("wb.chapter")}</h2>
        <div className="row">
          <Field label={t("wb.selectChapter")}>
            <select value={currentChapter} onChange={(e) => setCurrentChapter(e.target.value)}>
              <option value="">{t("wb.none")}</option>
              {chapters.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </Field>
          <Field label={t("wb.newId")} hint={t("wb.newIdHint")}>
            <div className="inline">
              <input
                value={newId}
                placeholder={t("wb.newIdPlaceholder")}
                onChange={(e) => setNewId(e.target.value)}
              />
              <button type="button" className="ghost" onClick={() => setNewId(nextChapterId())}>
                {t("wb.autoNumber")}
              </button>
            </div>
          </Field>
        </div>

        <Field label={t("wb.idea")}>
          <textarea
            className="idea"
            value={idea}
            placeholder={t("wb.ideaPlaceholder")}
            onChange={(e) => setIdea(e.target.value)}
          />
        </Field>

        <div className="row">
          <Field label={t("wb.targetWords")}>
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
              <input type="checkbox" checked={useContext} onChange={(e) => setUseContext(e.target.checked)} />
              {t("wb.useContext")}
            </label>
            <label>
              <input type="checkbox" checked={overwrite} onChange={(e) => setOverwrite(e.target.checked)} />
              {t("wb.overwrite")}
            </label>
          </div>
        </div>

        <div className="actions">
          <button className="primary" onClick={onGenerate} disabled={running}>
            {busy === "generate" ? <Spinner /> : null} {t("wb.generate")}
          </button>
        </div>
      </div>

      <div className="panel">
        <div className="panel-head">
          <h2>
            {t("wb.bodyTitle")} {currentChapter ? `· ${currentChapter}` : ""}
          </h2>
          <div className="panel-actions">
            <button onClick={onSave} disabled={!currentChapter || running || !dirty}>
              {busy === "save" ? <Spinner /> : null} {t("wb.save")}
              {dirty ? " *" : ""}
            </button>
          </div>
        </div>
        <textarea
          className="chapter-body"
          value={content}
          placeholder={currentChapter ? t("wb.bodyHas") : t("wb.bodyNone")}
          onChange={(e) => {
            setContent(e.target.value);
            setDirty(true);
          }}
          disabled={!currentChapter}
        />
        <div className="wordcount">
          {content.length} {t("wb.words")}
        </div>

        <div className="actions wrap">
          <button onClick={onCheck} disabled={!currentChapter || running}>
            {busy === "consistency" ? <Spinner /> : null} {t("wb.check")}
          </button>
          <button onClick={onUpdateMemory} disabled={!currentChapter || running}>
            {busy === "memory" ? <Spinner /> : null} {t("wb.updateMemory")}
          </button>
          <span className="spacer" />
          <label className="danger-check">
            <input type="checkbox" checked={includeMemory} onChange={(e) => setIncludeMemory(e.target.checked)} />
            {t("wb.includeMemory")}
          </label>
          <button className="danger" onClick={onReset} disabled={!currentChapter || running}>
            {busy === "reset" ? <Spinner /> : null} {t("wb.reset")}
          </button>
        </div>
      </div>

      {banner ? <Banner kind={banner.kind}>{banner.msg}</Banner> : null}
      <LogBox log={log} />
    </div>
  );
}
