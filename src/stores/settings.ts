import { defineStore } from "pinia";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { resolve } from "@tauri-apps/api/path";
import { normalizeLocale, resolveDefaultLocale, type Locale } from "../lib/locale";

interface PersistedSettings {
  language: Locale;
}

const SETTINGS_FILE = "./settings.yml";

function parseSettings(raw: string): Partial<PersistedSettings> {
  const result: Partial<PersistedSettings> = {};
  const lines = raw.split(/\r?\n/);
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) {
      continue;
    }
    const cleaned = line.split("#")[0]?.trim() ?? "";
    if (!cleaned) {
      continue;
    }
    const match = cleaned.match(/^([A-Za-z0-9_-]+)\s*:\s*(.+)$/);
    if (!match) {
      continue;
    }
    const key = match[1];
    let value = match[2].trim();
    if (
      (value.startsWith("\"") && value.endsWith("\"")) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (key === "language" || key === "locale") {
      const normalized = normalizeLocale(value);
      if (normalized) {
        result.language = normalized;
      }
    }
  }
  return result;
}

function serializeSettings(settings: PersistedSettings) {
  return `language: ${settings.language}\n`;
}

export const useSettingsStore = defineStore("settings", {
  state: () => ({
    locale: resolveDefaultLocale() as Locale,
    initialized: false,
    settingsPath: null as string | null,
  }),
  actions: {
    async initialize() {
      if (this.initialized) {
        return;
      }
      this.initialized = true;
      await this.load();
    },
    async load() {
      let hasFile = false;
      try {
        const path = await this.ensureSettingsPath();
        const raw = await readTextFile(path);
        hasFile = true;
        const parsed = parseSettings(raw);
        if (parsed.language) {
          this.locale = parsed.language;
        }
      } catch (error) {
        console.warn("Failed to load settings", error);
      }
      if (!hasFile) {
        await this.save();
      }
      this.syncDocumentLang();
    },
    async setLocale(nextLocale: Locale) {
      if (this.locale === nextLocale) {
        return;
      }
      this.locale = nextLocale;
      this.syncDocumentLang();
      await this.save();
    },
    async save() {
      try {
        const path = await this.ensureSettingsPath();
        await writeTextFile(path, serializeSettings({ language: this.locale }));
      } catch (error) {
        console.warn("Failed to save settings", error);
      }
    },
    syncDocumentLang() {
      if (typeof document === "undefined") {
        return;
      }
      document.documentElement.lang = this.locale;
    },
    async ensureSettingsPath() {
      if (this.settingsPath) {
        return this.settingsPath;
      }
      this.settingsPath = await resolve(SETTINGS_FILE);
      return this.settingsPath;
    },
  },
});
