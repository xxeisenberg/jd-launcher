import { useEffect, useCallback } from "react";

export type KeybindAction =
  | "navigate-instances"
  | "navigate-modpacks"
  | "navigate-settings"
  | "toggle-console"
  | "new-instance"
  | "import-instance";

export const KEYBIND_LABELS: Record<KeybindAction, string> = {
  "navigate-instances": "Go to Instances",
  "navigate-modpacks": "Go to Modpacks",
  "navigate-settings": "Go to Settings",
  "toggle-console": "Toggle Console",
  "new-instance": "New Instance",
  "import-instance": "Import Instance",
};

export const DEFAULT_KEYBINDS: Record<KeybindAction, string> = {
  "navigate-instances": "ctrl+1",
  "navigate-modpacks": "ctrl+2",
  "navigate-settings": "ctrl+,",
  "toggle-console": "ctrl+`",
  "new-instance": "ctrl+n",
  "import-instance": "ctrl+i",
};

export const KEYBIND_ACTIONS = Object.keys(DEFAULT_KEYBINDS) as KeybindAction[];

const STORAGE_KEY = "jd-launcher-keybinds";

export function getKeybinds(): Record<KeybindAction, string> {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      return { ...DEFAULT_KEYBINDS, ...JSON.parse(stored) };
    }
  } catch {
    // ignore
  }
  return { ...DEFAULT_KEYBINDS };
}

export function saveKeybinds(keybinds: Record<KeybindAction, string>) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(keybinds));
  window.dispatchEvent(new Event("keybinds-changed"));
}

export function resetKeybinds() {
  localStorage.removeItem(STORAGE_KEY);
  window.dispatchEvent(new Event("keybinds-changed"));
}

// Normalize a KeyboardEvent into a combo string
export function eventToCombo(e: KeyboardEvent): string | null {
  const key = e.key.toLowerCase();

  // Ignore bare modifier presses
  if (["control", "shift", "alt", "meta"].includes(key)) return null;

  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("ctrl");
  if (e.altKey) parts.push("alt");
  if (e.shiftKey) parts.push("shift");
  parts.push(key);
  return parts.join("+");
}

// Pretty-print a combo
export function formatKeybind(combo: string): string {
  return combo
    .split("+")
    .map((part) => {
      if (part === "ctrl") return "Ctrl";
      if (part === "alt") return "Alt";
      if (part === "shift") return "Shift";
      if (part === "`") return "`";
      if (part === ",") return ",";
      if (part.length === 1) return part.toUpperCase();
      return part.charAt(0).toUpperCase() + part.slice(1);
    })
    .join(" + ");
}

export function useKeybindListener(
  keybinds: Record<KeybindAction, string>,
  callbacks: Partial<Record<KeybindAction, () => void>>,
) {
  const handler = useCallback(
    (e: KeyboardEvent) => {
      // Skip if user is typing in an input
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      const combo = eventToCombo(e);
      if (!combo) return;

      for (const action of KEYBIND_ACTIONS) {
        if (keybinds[action] === combo && callbacks[action]) {
          e.preventDefault();
          callbacks[action]!();
          return;
        }
      }
    },
    [keybinds, callbacks],
  );

  useEffect(() => {
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [handler]);
}
