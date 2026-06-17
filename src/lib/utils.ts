import { localeTag, t, type TKey } from "./i18n.svelte";
import type { ModelInfo } from "./api";

export const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta"]);

export function keyFromEvent(e: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null;
  const code = e.code;
  if (/^F\d{1,2}$/.test(e.key)) return e.key;
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  const named: Record<string, string> = {
    Space: "Space",
    Tab: "Tab",
    Enter: "Enter",
    Backquote: "`",
    Minus: "-",
    Equal: "=",
    Comma: ",",
    Period: ".",
    Slash: "/",
    Semicolon: ";",
    Quote: "'",
    BracketLeft: "[",
    BracketRight: "]",
    Backslash: "\\",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    Insert: "Insert",
    Delete: "Delete",
  };
  return named[code] ?? null;
}

export function fmtSize(mb: number): string {
  return mb >= 1000 ? `${(mb / 1000).toFixed(1)} ${t("unit_gb")}` : `${mb} ${t("unit_mb")}`;
}

export function fmtRam(bytes: number): string {
  const mb = bytes / 1_048_576;
  return mb >= 1000 ? `${(mb / 1024).toFixed(1)} Go` : `${Math.round(mb)} Mo`;
}

export function fmtTime(tsMs: number): string {
  return new Date(tsMs).toLocaleTimeString(localeTag(), {
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function dayLabel(tsMs: number): string {
  const d = new Date(tsMs);
  const now = new Date();
  if (d.toDateString() === now.toDateString()) return t("today");
  if (d.toDateString() === new Date(now.getTime() - 86_400_000).toDateString()) {
    return t("yesterday");
  }
  return d.toLocaleDateString(localeTag(), { day: "numeric", month: "long" });
}

export function modelDesc(model: ModelInfo): string {
  const key = `model_desc.${model.id}` as TKey;
  const translated = t(key);
  return translated === key ? model.description : translated;
}
