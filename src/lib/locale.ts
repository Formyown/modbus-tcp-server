export const SUPPORTED_LOCALES = ["en", "zh-CN"] as const;

export type Locale = (typeof SUPPORTED_LOCALES)[number];

export function normalizeLocale(value: string | null | undefined): Locale | null {
  if (!value) {
    return null;
  }
  const trimmed = value.trim();
  if (SUPPORTED_LOCALES.includes(trimmed as Locale)) {
    return trimmed as Locale;
  }
  const lower = trimmed.toLowerCase();
  if (lower.startsWith("zh")) {
    return "zh-CN";
  }
  if (lower.startsWith("en")) {
    return "en";
  }
  return null;
}

export function resolveDefaultLocale(): Locale {
  if (typeof navigator === "undefined") {
    return "en";
  }
  return normalizeLocale(navigator.language) ?? "en";
}
