// Accent color definitions (oklch primary values)
export const ACCENT_COLORS: Record<
  string,
  {
    light: Record<string, string>;
    dark: Record<string, string>;
    swatch: string;
  }
> = {
  Amber: {
    swatch: "#f59e0b",
    light: {
      "--primary": "oklch(0.65 0.17 65)",
      "--primary-foreground": "oklch(0.98 0.02 65)",
      "--ring": "oklch(0.65 0.17 65)",
    },
    dark: {
      "--primary": "oklch(0.75 0.17 65)",
      "--primary-foreground": "oklch(0.15 0.04 65)",
      "--ring": "oklch(0.75 0.17 65)",
    },
  },
  Blue: {
    swatch: "#3b82f6",
    light: {
      "--primary": "oklch(0.55 0.19 255)",
      "--primary-foreground": "oklch(0.98 0.01 255)",
      "--ring": "oklch(0.55 0.19 255)",
    },
    dark: {
      "--primary": "oklch(0.65 0.19 255)",
      "--primary-foreground": "oklch(0.15 0.04 255)",
      "--ring": "oklch(0.65 0.19 255)",
    },
  },
  Cyan: {
    swatch: "#06b6d4",
    light: {
      "--primary": "oklch(0.65 0.12 200)",
      "--primary-foreground": "oklch(0.98 0.02 200)",
      "--ring": "oklch(0.65 0.12 200)",
    },
    dark: {
      "--primary": "oklch(0.72 0.12 200)",
      "--primary-foreground": "oklch(0.15 0.03 200)",
      "--ring": "oklch(0.72 0.12 200)",
    },
  },
  Emerald: {
    swatch: "#10b981",
    light: {
      "--primary": "oklch(0.60 0.15 160)",
      "--primary-foreground": "oklch(0.98 0.02 160)",
      "--ring": "oklch(0.60 0.15 160)",
    },
    dark: {
      "--primary": "oklch(0.70 0.15 160)",
      "--primary-foreground": "oklch(0.13 0.04 160)",
      "--ring": "oklch(0.70 0.15 160)",
    },
  },
  Fuchsia: {
    swatch: "#d946ef",
    light: {
      "--primary": "oklch(0.55 0.22 320)",
      "--primary-foreground": "oklch(0.98 0.02 320)",
      "--ring": "oklch(0.55 0.22 320)",
    },
    dark: {
      "--primary": "oklch(0.70 0.22 320)",
      "--primary-foreground": "oklch(0.15 0.05 320)",
      "--ring": "oklch(0.70 0.22 320)",
    },
  },
  Green: {
    swatch: "#22c55e",
    light: {
      "--primary": "oklch(0.60 0.13 163)",
      "--primary-foreground": "oklch(0.98 0.02 166)",
      "--ring": "oklch(0.60 0.13 163)",
    },
    dark: {
      "--primary": "oklch(0.70 0.15 162)",
      "--primary-foreground": "oklch(0.13 0.04 170)",
      "--ring": "oklch(0.70 0.15 162)",
    },
  },
  Indigo: {
    swatch: "#6366f1",
    light: {
      "--primary": "oklch(0.50 0.20 275)",
      "--primary-foreground": "oklch(0.98 0.01 275)",
      "--ring": "oklch(0.50 0.20 275)",
    },
    dark: {
      "--primary": "oklch(0.65 0.20 275)",
      "--primary-foreground": "oklch(0.15 0.04 275)",
      "--ring": "oklch(0.65 0.20 275)",
    },
  },
  Lime: {
    swatch: "#84cc16",
    light: {
      "--primary": "oklch(0.65 0.18 130)",
      "--primary-foreground": "oklch(0.98 0.02 130)",
      "--ring": "oklch(0.65 0.18 130)",
    },
    dark: {
      "--primary": "oklch(0.75 0.18 130)",
      "--primary-foreground": "oklch(0.15 0.04 130)",
      "--ring": "oklch(0.75 0.18 130)",
    },
  },
  Orange: {
    swatch: "#f97316",
    light: {
      "--primary": "oklch(0.60 0.18 45)",
      "--primary-foreground": "oklch(0.98 0.02 45)",
      "--ring": "oklch(0.60 0.18 45)",
    },
    dark: {
      "--primary": "oklch(0.72 0.18 45)",
      "--primary-foreground": "oklch(0.15 0.04 45)",
      "--ring": "oklch(0.72 0.18 45)",
    },
  },
  Pink: {
    swatch: "#ec4899",
    light: {
      "--primary": "oklch(0.55 0.20 350)",
      "--primary-foreground": "oklch(0.98 0.02 350)",
      "--ring": "oklch(0.55 0.20 350)",
    },
    dark: {
      "--primary": "oklch(0.70 0.20 350)",
      "--primary-foreground": "oklch(0.15 0.04 350)",
      "--ring": "oklch(0.70 0.20 350)",
    },
  },
  Purple: {
    swatch: "#a855f7",
    light: {
      "--primary": "oklch(0.52 0.22 295)",
      "--primary-foreground": "oklch(0.98 0.01 295)",
      "--ring": "oklch(0.52 0.22 295)",
    },
    dark: {
      "--primary": "oklch(0.68 0.22 295)",
      "--primary-foreground": "oklch(0.15 0.05 295)",
      "--ring": "oklch(0.68 0.22 295)",
    },
  },
  Red: {
    swatch: "#ef4444",
    light: {
      "--primary": "oklch(0.55 0.22 25)",
      "--primary-foreground": "oklch(0.98 0.02 25)",
      "--ring": "oklch(0.55 0.22 25)",
    },
    dark: {
      "--primary": "oklch(0.65 0.22 25)",
      "--primary-foreground": "oklch(0.15 0.04 25)",
      "--ring": "oklch(0.65 0.22 25)",
    },
  },
  Rose: {
    swatch: "#f43f5e",
    light: {
      "--primary": "oklch(0.55 0.20 10)",
      "--primary-foreground": "oklch(0.98 0.02 10)",
      "--ring": "oklch(0.55 0.20 10)",
    },
    dark: {
      "--primary": "oklch(0.68 0.20 10)",
      "--primary-foreground": "oklch(0.15 0.04 10)",
      "--ring": "oklch(0.68 0.20 10)",
    },
  },
  Sky: {
    swatch: "#0ea5e9",
    light: {
      "--primary": "oklch(0.58 0.15 230)",
      "--primary-foreground": "oklch(0.98 0.02 230)",
      "--ring": "oklch(0.58 0.15 230)",
    },
    dark: {
      "--primary": "oklch(0.70 0.15 230)",
      "--primary-foreground": "oklch(0.15 0.03 230)",
      "--ring": "oklch(0.70 0.15 230)",
    },
  },
  Teal: {
    swatch: "#14b8a6",
    light: {
      "--primary": "oklch(0.60 0.12 180)",
      "--primary-foreground": "oklch(0.98 0.02 180)",
      "--ring": "oklch(0.60 0.12 180)",
    },
    dark: {
      "--primary": "oklch(0.72 0.12 180)",
      "--primary-foreground": "oklch(0.15 0.03 180)",
      "--ring": "oklch(0.72 0.12 180)",
    },
  },
  Violet: {
    swatch: "#8b5cf6",
    light: {
      "--primary": "oklch(0.52 0.22 285)",
      "--primary-foreground": "oklch(0.98 0.01 285)",
      "--ring": "oklch(0.52 0.22 285)",
    },
    dark: {
      "--primary": "oklch(0.68 0.22 285)",
      "--primary-foreground": "oklch(0.15 0.05 285)",
      "--ring": "oklch(0.68 0.22 285)",
    },
  },
  Yellow: {
    swatch: "#eab308",
    light: {
      "--primary": "oklch(0.70 0.17 85)",
      "--primary-foreground": "oklch(0.20 0.04 85)",
      "--ring": "oklch(0.70 0.17 85)",
    },
    dark: {
      "--primary": "oklch(0.80 0.17 85)",
      "--primary-foreground": "oklch(0.15 0.04 85)",
      "--ring": "oklch(0.80 0.17 85)",
    },
  },
};

// Font definitions
export const FONTS: { name: string; value: string; mono?: boolean }[] = [
  { name: "Geist", value: "'Geist', system-ui, sans-serif" },
  { name: "Inter", value: "'Inter', system-ui, sans-serif" },
  { name: "Noto Sans", value: "'Noto Sans', system-ui, sans-serif" },
  { name: "Nunito Sans", value: "'Nunito Sans', system-ui, sans-serif" },
  { name: "Figtree", value: "'Figtree', system-ui, sans-serif" },
  { name: "Roboto", value: "'Roboto', system-ui, sans-serif" },
  { name: "Raleway", value: "'Raleway', system-ui, sans-serif" },
  { name: "DM Sans", value: "'DM Sans', system-ui, sans-serif" },
  { name: "Public Sans", value: "'Public Sans', system-ui, sans-serif" },
  {
    name: "Outfit",
    value: "'Outfit Variable', 'Outfit', system-ui, sans-serif",
  },
  {
    name: "Geist Mono",
    value: "'Geist Mono', ui-monospace, monospace",
    mono: true,
  },
  {
    name: "JetBrains Mono",
    value: "'JetBrains Mono', ui-monospace, monospace",
    mono: true,
  },
];

// UI style presets
export const UI_STYLES: {
  name: string;
  radius: string;
  spacing: string;
  description: string;
}[] = [
  {
    name: "Vega",
    radius: "0.5rem",
    spacing: "0.25rem",
    description: "Default",
  },
  {
    name: "Nova",
    radius: "0.375rem",
    spacing: "0.225rem",
    description: "Compact",
  },
  { name: "Maia", radius: "0.75rem", spacing: "0.275rem", description: "Soft" },
  { name: "Lyra", radius: "0rem", spacing: "0.25rem", description: "Sharp" },
  { name: "Mira", radius: "0.25rem", spacing: "0.2rem", description: "Dense" },
];

// Apply all appearance settings
export function applyTheme(
  isDark: boolean,
  accentColor: string,
  fontFamily: string,
  uiStyle: string,
  uiScale: number,
) {
  const root = document.documentElement;

  // Dark mode
  root.classList.toggle("dark", isDark);

  // Scale - set font-size on the root html to scale all `rem` units seamlessly
  root.style.fontSize = `${uiScale}%`;

  // Accent color
  const accent = ACCENT_COLORS[accentColor] ?? ACCENT_COLORS.Blue;
  const vars = isDark ? accent.dark : accent.light;
  for (const [key, val] of Object.entries(vars)) {
    root.style.setProperty(key, val);
  }
  root.style.setProperty("--sidebar-primary", vars["--primary"]);
  root.style.setProperty(
    "--sidebar-primary-foreground",
    vars["--primary-foreground"],
  );
  root.style.setProperty("--sidebar-ring", vars["--ring"]);

  // Font — set directly since @theme inline bakes values
  const font = FONTS.find((f) => f.name === fontFamily) ?? FONTS[0];
  document.body.style.fontFamily = font.value;

  // Style — set --radius and all derived radius vars
  const style = UI_STYLES.find((s) => s.name === uiStyle) ?? UI_STYLES[0];
  const rRem = parseFloat(style.radius);
  const rPx = rRem * 16;

  root.style.setProperty("--spacing", style.spacing);
  root.style.setProperty("--radius", `${rPx}px`);
  root.style.setProperty("--radius-sm", `${Math.max(0, rPx - 4)}px`);
  root.style.setProperty("--radius-md", `${Math.max(0, rPx - 2)}px`);
  root.style.setProperty("--radius-lg", `${rPx}px`);
  root.style.setProperty("--radius-xl", `${rPx + 4}px`);
  root.style.setProperty("--radius-2xl", `${rPx + 8}px`);
  root.style.setProperty("--radius-3xl", `${rPx + 12}px`);
  root.style.setProperty("--radius-4xl", `${rPx + 16}px`);
}
