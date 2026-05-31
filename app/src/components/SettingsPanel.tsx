import { useEffect, useState } from "react";
import { api, Settings } from "../api";
import { Banner, Field, Spinner } from "./common";

export default function SettingsPanel({ root }: { root: string }) {
  const [settings, setSettings] = useState<Settings>({
    project_dir: "",
    llm_base_url: "http://localhost:18083/v1",
    llm_api_key: "local",
    llm_model: "",
  });
  const [busy, setBusy] = useState(false);
  const [banner, setBanner] = useState<{ kind: "ok" | "err" | "info"; msg: string } | null>(null);

  useEffect(() => {
    api.getSettings().then(setSettings).catch(() => {});
  }, []);

  function update<K extends keyof Settings>(key: K, value: Settings[K]) {
    setSettings((s) => ({ ...s, [key]: value }));
  }

  async function onSave() {
    setBusy(true);
    try {
      await api.saveSettings(settings);
      setBanner({ kind: "ok", msg: "设置已保存，下一次运行即生效。" });
    } catch (e) {
      setBanner({ kind: "err", msg: String(e) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="settings">
      <div className="panel">
        <h2>运行设置</h2>
        <p className="muted">
          所有设置保存在项目目录的 <code>.ui-settings.json</code> 中。
          修改后下一次运行即生效。
        </p>

        <Field label="项目目录" hint="小说项目根目录；留空则使用默认路径">
          <input
            value={settings.project_dir}
            onChange={(e) => update("project_dir", e.target.value)}
            placeholder="留空使用默认目录"
          />
        </Field>

        <Field label="LLM Base URL" hint="本地 OpenAI 兼容服务地址">
          <input
            value={settings.llm_base_url}
            onChange={(e) => update("llm_base_url", e.target.value)}
          />
        </Field>

        <Field label="LLM API Key" hint="本地服务通常填任意值，如 local">
          <input
            type="password"
            value={settings.llm_api_key}
            onChange={(e) => update("llm_api_key", e.target.value)}
          />
        </Field>

        <Field label="模型名称" hint="留空则使用 config.yaml 中的 model_name">
          <input
            value={settings.llm_model}
            onChange={(e) => update("llm_model", e.target.value)}
          />
        </Field>

        <div className="actions">
          <button className="primary" onClick={onSave} disabled={busy}>
            {busy ? <Spinner /> : null} 保存设置
          </button>
        </div>

        {banner ? <Banner kind={banner.kind}>{banner.msg}</Banner> : null}

        <div className="root-info">
          <span className="muted">流水线根目录：</span>
          <code>{root || "（定位中…）"}</code>
        </div>
      </div>
    </div>
  );
}
