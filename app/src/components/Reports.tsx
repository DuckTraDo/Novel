import { useEffect, useState } from "react";
import { marked } from "marked";
import { api, ReportKind } from "../api";
import { useT } from "../i18n";
import { Banner } from "./common";

interface Props {
  chapters: string[];
  currentChapter: string;
  setCurrentChapter: (id: string) => void;
}

const KINDS: { key: ReportKind; labelKey: string }[] = [
  { key: "generation", labelKey: "rp.generation" },
  { key: "consistency", labelKey: "rp.consistency" },
  { key: "memory", labelKey: "rp.memory" },
];

export default function Reports({ chapters, currentChapter, setCurrentChapter }: Props) {
  const t = useT();
  const [kind, setKind] = useState<ReportKind>("consistency");
  const [html, setHtml] = useState("");
  const [empty, setEmpty] = useState(false);

  async function load() {
    if (!currentChapter) {
      setHtml("");
      setEmpty(false);
      return;
    }
    try {
      const md = await api.readReport(currentChapter, kind);
      if (!md.trim()) {
        setHtml("");
        setEmpty(true);
        return;
      }
      setEmpty(false);
      setHtml(await marked.parse(md));
    } catch {
      setHtml("");
      setEmpty(true);
    }
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentChapter, kind]);

  return (
    <div className="reports">
      <div className="reports-bar">
        <select value={currentChapter} onChange={(e) => setCurrentChapter(e.target.value)}>
          <option value="">{t("rp.selectChapter")}</option>
          {chapters.map((c) => (
            <option key={c} value={c}>
              {c}
            </option>
          ))}
        </select>
        <div className="tabs">
          {KINDS.map((k) => (
            <button
              key={k.key}
              className={kind === k.key ? "tab active" : "tab"}
              onClick={() => setKind(k.key)}
            >
              {t(k.labelKey)}
            </button>
          ))}
        </div>
        <button className="ghost" onClick={load}>
          {t("rp.refresh")}
        </button>
      </div>

      {!currentChapter ? (
        <Banner kind="info">{t("rp.pickChapter")}</Banner>
      ) : empty ? (
        <Banner kind="info">{t("rp.empty")}</Banner>
      ) : (
        <div className="markdown" dangerouslySetInnerHTML={{ __html: html }} />
      )}
    </div>
  );
}
