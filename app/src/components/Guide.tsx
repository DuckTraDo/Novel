import { useEffect, useState } from "react";
import { marked } from "marked";
import { useUi } from "../i18n";
import guideZh from "../guide.md?raw";
import guideEn from "../guide.en.md?raw";

export default function Guide() {
  const { lang } = useUi();
  const [html, setHtml] = useState("");

  useEffect(() => {
    const md = lang === "en" ? guideEn : guideZh;
    Promise.resolve(marked.parse(md)).then(setHtml);
  }, [lang]);

  return (
    <div className="guide">
      <div className="markdown" dangerouslySetInnerHTML={{ __html: html }} />
    </div>
  );
}
