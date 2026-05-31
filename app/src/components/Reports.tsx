import { useEffect, useState } from "react";
import { marked } from "marked";
import { api, ReportKind } from "../api";
import { Banner } from "./common";

interface Props {
  chapters: string[];
  currentChapter: string;
  setCurrentChapter: (id: string) => void;
}

const KINDS: { key: ReportKind; label: string }[] = [
  { key: "generation", label: "生成报告" },
  { key: "consistency", label: "一致性报告" },
  { key: "memory", label: "记忆更新报告" },
];

export default function Reports({
  chapters,
  currentChapter,
  setCurrentChapter,
}: Props) {
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
    } catch (e) {
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
        <select
          value={currentChapter}
          onChange={(e) => setCurrentChapter(e.target.value)}
        >
          <option value="">（选择章节）</option>
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
              {k.label}
            </button>
          ))}
        </div>
        <button className="ghost" onClick={load}>
          刷新
        </button>
      </div>

      {!currentChapter ? (
        <Banner kind="info">请选择一个章节查看报告。</Banner>
      ) : empty ? (
        <Banner kind="info">
          暂无该报告。可在「章节工作台」运行对应操作后再来查看。
        </Banner>
      ) : (
        <div className="markdown" dangerouslySetInnerHTML={{ __html: html }} />
      )}
    </div>
  );
}
