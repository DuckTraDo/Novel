import { useEffect, useState } from "react";
import { api, Settings } from "../api";
import { useT } from "../i18n";
import { Banner, Field, Spinner } from "./common";

export default function SettingsPanel({ root }: { root: string }) {
  const t = useT();
  const [settings, setSettings] = useState<Settings>({
    project_dir: "",
    llm_base_url: "http://127.0.0.1:18180/v1",
    llm_api_key: "local",
    llm_model: "",
    disable_thinking: true,
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
      setBanner({ kind: "ok", msg: t("st.saved") });
    } catch (e) {
      setBanner({ kind: "err", msg: String(e) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="settings">
      <div className="panel">
        <h2>{t("st.title")}</h2>
        <p className="muted">{t("st.desc")}</p>

        <Field label={t("st.projectDir")} hint={t("st.projectDirHint")}>
          <input
            value={settings.project_dir}
            placeholder={t("st.projectDirPlaceholder")}
            onChange={(e) => update("project_dir", e.target.value)}
          />
        </Field>

        <Field label={t("st.baseUrl")} hint={t("st.baseUrlHint")}>
          <input
            value={settings.llm_base_url}
            placeholder="http://127.0.0.1:18180/v1"
            onChange={(e) => update("llm_base_url", e.target.value)}
          />
        </Field>

        <Field label={t("st.apiKey")} hint={t("st.apiKeyHint")}>
          <input
            type="password"
            value={settings.llm_api_key}
            onChange={(e) => update("llm_api_key", e.target.value)}
          />
        </Field>

        <Field label={t("st.model")} hint={t("st.modelHint")}>
          <input value={settings.llm_model} onChange={(e) => update("llm_model", e.target.value)} />
        </Field>

        <label className="check-row">
          <input
            type="checkbox"
            checked={settings.disable_thinking !== false}
            onChange={(e) => update("disable_thinking", e.target.checked)}
          />
          <span className="check-text">
            <span className="check-title">{t("st.disableThinking")}</span>
            <span className="field-hint">{t("st.disableThinkingHint")}</span>
          </span>
        </label>

        <div className="actions">
          <button className="primary" onClick={onSave} disabled={busy}>
            {busy ? <Spinner /> : null} {t("st.save")}
          </button>
        </div>

        {banner ? <Banner kind={banner.kind}>{banner.msg}</Banner> : null}

        <div className="root-info">
          <span className="muted">{t("st.root")}</span>
          <code>{root || t("st.locating")}</code>
        </div>
      </div>
    </div>
  );
}
