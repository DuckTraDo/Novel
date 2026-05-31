import { invoke } from "@tauri-apps/api/core";

export interface CommandResult {
  success: boolean;
  log: string;
}

export interface Settings {
  project_dir: string;
  llm_base_url: string;
  llm_api_key: string;
  llm_model: string;
  disable_thinking?: boolean;
}

export type ReportKind = "generation" | "consistency" | "memory";

export const api = {
  getPipelineRoot: () => invoke<string>("get_pipeline_root"),

  listChapters: () => invoke<string[]>("list_chapters"),
  readChapter: (chapterId: string) =>
    invoke<string>("read_chapter", { chapterId }),
  saveChapter: (chapterId: string, content: string) =>
    invoke<void>("save_chapter", { chapterId, content }),

  readReport: (chapterId: string, kind: ReportKind) =>
    invoke<string>("read_report", { chapterId, kind }),

  openUrl: (url: string) => invoke<void>("open_url", { url }),

  readMemoryFile: (rel: string) =>
    invoke<string>("read_memory_file", { rel }),
  saveMemoryFile: (rel: string, content: string) =>
    invoke<void>("save_memory_file", { rel, content }),

  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) =>
    invoke<void>("save_settings", { settings }),

  generateChapter: (
    chapterId: string,
    idea: string,
    targetWords: number,
    useContext: boolean,
    overwrite: boolean,
    pov: string,
    narrative: string
  ) =>
    invoke<CommandResult>("generate_chapter", {
      chapterId,
      idea,
      targetWords,
      useContext,
      overwrite,
      pov,
      narrative,
    }),

  checkConsistency: (chapterId: string) =>
    invoke<CommandResult>("check_consistency", { chapterId }),

  updateMemory: (chapterId: string) =>
    invoke<CommandResult>("update_memory", { chapterId }),

  resetChapter: (chapterId: string, includeMemory: boolean) =>
    invoke<CommandResult>("reset_chapter", { chapterId, includeMemory }),
};
