import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  ReactNode,
} from "react";

export type Lang = "zh" | "en";
export type Theme = "light" | "dark" | "warm";

type Dict = Record<string, string>;

const zh: Dict = {
  "brand.subtitle": "本地大模型 · 结构化记忆",
  "nav.workbench": "章节工作台",
  "nav.reports": "报告",
  "nav.memory": "记忆库",
  "nav.settings": "设置",
  "nav.guide": "使用指南",
  "sidebar.chapters": "章节数",
  "madeBy": "Made by",
  "topbar.theme": "主题",
  "topbar.lang": "语言",
  "theme.light": "浅色",
  "theme.dark": "深色",
  "theme.warm": "暖色",

  "wb.chapter": "章节",
  "wb.selectChapter": "选择章节",
  "wb.none": "（未选择 / 新建）",
  "wb.newId": "新建章节 ID",
  "wb.newIdHint": "留空选择时用此 ID 生成",
  "wb.autoNumber": "自动编号",
  "wb.newIdPlaceholder": "如 ch001",
  "wb.idea": "本章 idea（一句话方向，最高优先级）",
  "wb.ideaPlaceholder":
    "例：第一章，主角回到故乡，发现父亲留下的一封信，决定调查多年前的旧事。",
  "wb.targetWords": "目标字数",
  "wb.useContext": "使用长期记忆",
  "wb.overwrite": "覆盖已有正文",
  "wb.narrativeSection": "叙事控制（可选）",
  "wb.pov": "视角 (POV)",
  "wb.povFollow": "跟随全书默认",
  "wb.povLimited": "第三人称限制",
  "wb.povObjective": "客观旁观（隐形观察者，只写可见）",
  "wb.povFirst": "第一人称",
  "wb.povOmniscient": "全知（不推荐）",
  "wb.povCharacter": "视角角色",
  "wb.povCharacterHint": "限制 / 第一人称时填写",
  "wb.narrative": "叙事规则（英文，逗号分隔）",
  "wb.narrativeHint": "形式指令用英文更贴合模型，例：Forensic Minimalism",
  "wb.narrativePlaceholder": "Forensic Minimalism, Diegetic Observation only",
  "wb.generate": "生成 / 重新生成",
  "wb.bodyTitle": "正文",
  "wb.save": "保存正文",
  "wb.bodyHas": "（本章暂无正文，点击上方生成）",
  "wb.bodyNone": "请选择或生成一个章节",
  "wb.words": "字",
  "wb.check": "一致性检查",
  "wb.updateMemory": "更新记忆",
  "wb.includeMemory": "连带记忆",
  "wb.reset": "重置本章",
  "wb.needChapter": "请先选择或新建一个章节 ID。",
  "wb.needIdea": "请填写本章 idea。",
  "wb.generating": "正在调用模型生成整章，请稍候……",
  "wb.genOk": "章节 {id} 已生成。",
  "wb.execFail": "执行失败，请看下方日志。",
  "wb.saved": "正文已保存。",
  "wb.checking": "正在进行一致性审查……",
  "wb.checkOk": "一致性审查完成，可到「报告」查看。",
  "wb.memRunning": "正在抽取并更新长期记忆……",
  "wb.memOk": "记忆更新完成。",
  "wb.resetConfirmMem":
    "确定重置 {id}？会删除正文、报告，并从摘要/事件/时间线中过滤该章记录。",
  "wb.resetConfirmNo": "确定重置 {id}？会删除正文和报告（不动长期记忆）。",
  "wb.resetOk": "{id} 已重置。",

  "rp.generation": "生成报告",
  "rp.consistency": "一致性报告",
  "rp.memory": "记忆更新报告",
  "rp.selectChapter": "（选择章节）",
  "rp.refresh": "刷新",
  "rp.pickChapter": "请选择一个章节查看报告。",
  "rp.empty": "暂无该报告。可在「章节工作台」运行对应操作后再来查看。",

  "me.groupCore": "核心设定",
  "me.groupAuto": "自动维护（谨慎手改）",
  "me.reload": "重新加载",
  "me.save": "保存",
  "me.hint":
    "直接编辑原始内容（YAML / JSON / JSONL）。保存后若格式有误，运行时会报错，请按提示修正。",
  "me.switchConfirm": "当前文件未保存，确定切换？",
  "me.savedSuffix": "已保存。",

  "st.title": "运行设置",
  "st.desc":
    "所有设置保存在项目目录的 .ui-settings.json 中。修改后下一次运行即生效。",
  "st.projectDir": "项目目录",
  "st.projectDirHint": "小说项目根目录；留空则使用默认路径",
  "st.projectDirPlaceholder": "留空使用默认目录",
  "st.provider": "LLM 路线",
  "st.providerHint": "选择按量计费的 API，或本机已登录的订阅 CLI（走 Pro/Max、ChatGPT 订阅额度）",
  "st.providerApi": "API（本地 / OpenAI 兼容）",
  "st.providerClaude": "Claude Code 订阅",
  "st.providerCodex": "Codex 订阅（ChatGPT）",
  "st.subModelHintClaude": "订阅模型别名，如 sonnet / opus；留空用订阅默认模型",
  "st.subModelHintCodex": "订阅模型名，如 gpt-5-codex；留空用订阅默认模型",
  "st.subNoteClaude":
    "需先安装 Claude Code 并用 Pro/Max 账号登录（终端运行 claude 登录）。本路线走订阅额度，会忽略下方 API Key。",
  "st.subNoteCodex":
    "需先安装 Codex CLI 并用 ChatGPT 订阅登录（终端运行 codex login）。本路线走订阅额度，会忽略下方 API Key。",
  "st.baseUrl": "LLM Base URL",
  "st.baseUrlHint": "须含协议头，多数本地服务在 /v1 下，例如 http://127.0.0.1:18180/v1",
  "st.apiKey": "LLM API Key",
  "st.apiKeyHint": "本地服务通常填任意值，如 local",
  "st.model": "模型名称",
  "st.modelHint": "留空则使用 config.yaml 中的 model_name",
  "st.disableThinking": "关闭模型思考（推荐）",
  "st.disableThinkingHint": "针对 Qwen3 等思考模型：勾选后直接出正文、避免空输出;需要模型推理时再取消",
  "st.save": "保存设置",
  "st.saved": "设置已保存，下一次运行即生效。",
  "st.root": "流水线根目录：",
  "st.locating": "（定位中…）",
};

const en: Dict = {
  "brand.subtitle": "Local LLM · Structured Memory",
  "nav.workbench": "Workbench",
  "nav.reports": "Reports",
  "nav.memory": "Memory",
  "nav.settings": "Settings",
  "nav.guide": "Guide",
  "sidebar.chapters": "Chapters",
  "madeBy": "Made by",
  "topbar.theme": "Theme",
  "topbar.lang": "Language",
  "theme.light": "Light",
  "theme.dark": "Dark",
  "theme.warm": "Warm",

  "wb.chapter": "Chapter",
  "wb.selectChapter": "Select chapter",
  "wb.none": "(none / new)",
  "wb.newId": "New chapter ID",
  "wb.newIdHint": "Used when no chapter is selected",
  "wb.autoNumber": "Auto number",
  "wb.newIdPlaceholder": "e.g. ch001",
  "wb.idea": "Chapter idea (one line — highest priority)",
  "wb.ideaPlaceholder":
    "e.g. Ch.1 — the hero returns home, finds a letter left by his father, and decides to dig into an old case.",
  "wb.targetWords": "Target length (chars)",
  "wb.useContext": "Use long-term memory",
  "wb.overwrite": "Overwrite existing text",
  "wb.narrativeSection": "Narrative control (optional)",
  "wb.pov": "Point of view",
  "wb.povFollow": "Follow book default",
  "wb.povLimited": "Third-person limited",
  "wb.povObjective": "Objective (invisible observer, visible-only)",
  "wb.povFirst": "First person",
  "wb.povOmniscient": "Omniscient (not recommended)",
  "wb.povCharacter": "POV character",
  "wb.povCharacterHint": "for limited / first person",
  "wb.narrative": "Narrative directives (English, comma-separated)",
  "wb.narrativeHint": "Form directives work best in English, e.g. Forensic Minimalism",
  "wb.narrativePlaceholder": "Forensic Minimalism, Diegetic Observation only",
  "wb.generate": "Generate / Regenerate",
  "wb.bodyTitle": "Chapter text",
  "wb.save": "Save text",
  "wb.bodyHas": "(No text yet — click Generate above)",
  "wb.bodyNone": "Select or generate a chapter",
  "wb.words": "chars",
  "wb.check": "Consistency check",
  "wb.updateMemory": "Update memory",
  "wb.includeMemory": "Incl. memory",
  "wb.reset": "Reset chapter",
  "wb.needChapter": "Please select or create a chapter ID first.",
  "wb.needIdea": "Please enter the chapter idea.",
  "wb.generating": "Generating the whole chapter, please wait…",
  "wb.genOk": "Chapter {id} generated.",
  "wb.execFail": "Failed — see the log below.",
  "wb.saved": "Text saved.",
  "wb.checking": "Running consistency check…",
  "wb.checkOk": "Consistency check done — see Reports.",
  "wb.memRunning": "Extracting and updating long-term memory…",
  "wb.memOk": "Memory updated.",
  "wb.resetConfirmMem":
    "Reset {id}? This deletes the text, reports, and its records in summaries/events/timeline.",
  "wb.resetConfirmNo": "Reset {id}? This deletes the text and reports (long-term memory untouched).",
  "wb.resetOk": "{id} reset.",

  "rp.generation": "Generation",
  "rp.consistency": "Consistency",
  "rp.memory": "Memory update",
  "rp.selectChapter": "(select chapter)",
  "rp.refresh": "Refresh",
  "rp.pickChapter": "Select a chapter to view its reports.",
  "rp.empty": "No report yet. Run the matching action in the Workbench first.",

  "me.groupCore": "Core setup",
  "me.groupAuto": "Auto-maintained (edit with care)",
  "me.reload": "Reload",
  "me.save": "Save",
  "me.hint":
    "Edit the raw content (YAML / JSON / JSONL). If the format is broken after saving, the run will report an error — fix as prompted.",
  "me.switchConfirm": "Unsaved changes in this file — switch anyway?",
  "me.savedSuffix": "saved.",

  "st.title": "Runtime settings",
  "st.desc":
    "All settings are saved to .ui-settings.json in the project dir. Changes take effect on the next run.",
  "st.projectDir": "Project directory",
  "st.projectDirHint": "Root of your novel project; leave empty for the default path",
  "st.projectDirPlaceholder": "Leave empty for the default directory",
  "st.provider": "LLM route",
  "st.providerHint":
    "Pick the pay-as-you-go API, or a subscription CLI you're already signed into (uses your Pro/Max or ChatGPT quota)",
  "st.providerApi": "API (local / OpenAI-compatible)",
  "st.providerClaude": "Claude Code subscription",
  "st.providerCodex": "Codex subscription (ChatGPT)",
  "st.subModelHintClaude": "Subscription model alias, e.g. sonnet / opus; empty = subscription default",
  "st.subModelHintCodex": "Subscription model name, e.g. gpt-5-codex; empty = subscription default",
  "st.subNoteClaude":
    "Requires Claude Code installed and signed in with a Pro/Max account (run claude in a terminal to log in). Uses your subscription quota; the API Key below is ignored.",
  "st.subNoteCodex":
    "Requires Codex CLI installed and signed in with ChatGPT (run codex login). Uses your subscription quota; the API Key below is ignored.",
  "st.baseUrl": "LLM Base URL",
  "st.baseUrlHint":
    "Must include the scheme; most local servers live under /v1, e.g. http://127.0.0.1:18180/v1",
  "st.apiKey": "LLM API Key",
  "st.apiKeyHint": "Local servers usually accept any value, e.g. local",
  "st.model": "Model name",
  "st.modelHint": "Leave empty to use model_name from config.yaml",
  "st.disableThinking": "Disable model thinking (recommended)",
  "st.disableThinkingHint": "For Qwen3-style thinking models: checked = direct prose, avoids empty output; uncheck to let it reason",
  "st.save": "Save settings",
  "st.saved": "Settings saved — effective on the next run.",
  "st.root": "Pipeline root:",
  "st.locating": "(locating…)",
};

const translations: Record<Lang, Dict> = { zh, en };

interface UiCtx {
  lang: Lang;
  setLang: (l: Lang) => void;
  theme: Theme;
  setTheme: (t: Theme) => void;
  t: (key: string, params?: Record<string, string | number>) => string;
}

const Ctx = createContext<UiCtx | null>(null);

export function UiProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(
    () => (localStorage.getItem("ui.lang") as Lang) || "zh"
  );
  const [theme, setThemeState] = useState<Theme>(
    () => (localStorage.getItem("ui.theme") as Theme) || "light"
  );

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  useEffect(() => {
    document.documentElement.setAttribute("lang", lang);
  }, [lang]);

  const setLang = useCallback((l: Lang) => {
    localStorage.setItem("ui.lang", l);
    setLangState(l);
  }, []);

  const setTheme = useCallback((t: Theme) => {
    localStorage.setItem("ui.theme", t);
    setThemeState(t);
  }, []);

  const t = useCallback(
    (key: string, params?: Record<string, string | number>) => {
      let s = translations[lang][key] ?? translations.zh[key] ?? key;
      if (params) {
        for (const [k, v] of Object.entries(params)) {
          s = s.replace(new RegExp(`\\{${k}\\}`, "g"), String(v));
        }
      }
      return s;
    },
    [lang]
  );

  const value = useMemo(
    () => ({ lang, setLang, theme, setTheme, t }),
    [lang, setLang, theme, setTheme, t]
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useUi(): UiCtx {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useUi must be used within UiProvider");
  return ctx;
}

export function useT() {
  return useUi().t;
}
