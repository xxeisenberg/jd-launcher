import React, { useEffect, useState } from "react";
import { commands } from "../bindings";
import type { LauncherSettings } from "../bindings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { ACCENT_COLORS, FONTS, UI_STYLES } from "@/lib/themes";

interface SettingsPageProps {
  onSettingsSaved: (settings: LauncherSettings) => void;
}

type TabKey =
  | "general"
  | "appearance"
  | "game"
  | "java"
  | "directories"
  | "network"
  | "developer";

const TABS: { key: TabKey; label: string }[] = [
  { key: "general", label: "General" },
  { key: "appearance", label: "Appearance" },
  { key: "game", label: "Game" },
  { key: "java", label: "Java" },
  { key: "directories", label: "Directories" },
  { key: "network", label: "Network" },
  { key: "developer", label: "Developer" },
];

export function SettingsPage({ onSettingsSaved }: SettingsPageProps) {
  const [settings, setSettings] = useState<LauncherSettings | null>(null);
  const [activeTab, setActiveTab] = useState<TabKey>("general");
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    commands.getSettings().then((s) => setSettings(s));
  }, []);

  const handleSave = async () => {
    if (!settings) return;
    setSaving(true);
    const result = await commands.updateSettings(settings);
    setSaving(false);
    if (result.status === "ok") {
      setDirty(false);
      onSettingsSaved(settings);
    } else {
      alert("Failed to save: " + result.error);
    }
  };

  const handleReset = async () => {
    if (!confirm("Reset all settings to defaults?")) return;
    setSaving(true);
    const result = await commands.resetSettings();
    if (result.status === "ok") {
      const s = await commands.getSettings();
      setSettings(s);
      setDirty(false);
      onSettingsSaved(s);
    }
    setSaving(false);
  };

  const set = (key: keyof LauncherSettings, value: any) => {
    if (!settings) return;
    const newSettings = { ...settings, [key]: value };
    setSettings(newSettings);
    setDirty(true);
  };

  if (!settings) return null;

  return (
    <div className="flex h-full">
      {/* tabs */}
      <nav className="w-44 shrink-0 border-r border-border p-3 flex flex-col gap-0.5">
        {TABS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={cn(
              "text-left text-sm px-3 py-1.5 rounded-md transition-colors",
              activeTab === tab.key
                ? "bg-primary/10 text-primary font-medium"
                : "text-muted-foreground hover:text-foreground hover:bg-accent",
            )}
          >
            {tab.label}
          </button>
        ))}
      </nav>

      {/* content */}
      <div className="flex-1 flex flex-col overflow-hidden">
        <div className="flex-1 overflow-y-auto p-6 space-y-5">
          {activeTab === "general" && (
            <>
              <FieldGroup label="Language">
                <Select
                  value={settings.language}
                  onValueChange={(v) => set("language", v)}
                >
                  <SelectTrigger className="w-48">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="en">English</SelectItem>
                  </SelectContent>
                </Select>
              </FieldGroup>
              <Separator />
              <CheckboxField
                label="Close launcher on game launch"
                checked={settings.close_on_launch}
                onChange={(v) => set("close_on_launch", v)}
              />
            </>
          )}

          {activeTab === "appearance" && (
            <>
              <FieldGroup label="Theme">
                <Select
                  value={settings.theme}
                  onValueChange={(v) => set("theme", v)}
                >
                  <SelectTrigger className="w-48">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="dark">Dark</SelectItem>
                    <SelectItem value="light">Light</SelectItem>
                  </SelectContent>
                </Select>
              </FieldGroup>
              <FieldGroup label="Accent Color">
                <Select
                  value={settings.accent_color}
                  onValueChange={(v) => set("accent_color", v)}
                >
                  <SelectTrigger className="w-48">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {Object.entries(ACCENT_COLORS).map(([name, c]) => (
                      <SelectItem key={name} value={name}>
                        <span className="flex items-center gap-2">
                          <span
                            className="w-3 h-3 rounded-full shrink-0 border border-border"
                            style={{ background: c.swatch }}
                          />
                          {name}
                        </span>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </FieldGroup>
              <FieldGroup label="Font">
                <Select
                  value={settings.font_family}
                  onValueChange={(v) => set("font_family", v)}
                >
                  <SelectTrigger className="w-48">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {FONTS.map((f) => (
                      <SelectItem key={f.name} value={f.name}>
                        {f.name}
                        {f.mono ? " (mono)" : ""}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </FieldGroup>
              <FieldGroup label="Style">
                <Select
                  value={settings.ui_style}
                  onValueChange={(v) => set("ui_style", v)}
                >
                  <SelectTrigger className="w-48">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {UI_STYLES.map((s) => (
                      <SelectItem key={s.name} value={s.name}>
                        {s.name}
                        <span className="text-muted-foreground ml-1">
                          — {s.description}
                        </span>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </FieldGroup>
              <FieldGroup label="UI Scale">
                <div className="flex gap-3 items-center w-48">
                  <input
                    type="range"
                    min="50"
                    max="150"
                    step="5"
                    className="flex-1 accent-primary"
                    value={settings.ui_scale}
                    onChange={(e) => set("ui_scale", parseInt(e.target.value))}
                  />
                  <span className="text-sm font-medium w-12 text-right">
                    {settings.ui_scale}%
                  </span>
                </div>
              </FieldGroup>
            </>
          )}

          {activeTab === "game" && (
            <>
              <div className="flex gap-4">
                <FieldGroup label="Default Width">
                  <Input
                    type="number"
                    min={1}
                    value={settings.default_resolution_width}
                    onChange={(e) =>
                      set(
                        "default_resolution_width",
                        parseInt(e.target.value) || 854,
                      )
                    }
                    className="w-28"
                  />
                </FieldGroup>
                <FieldGroup label="Default Height">
                  <Input
                    type="number"
                    min={1}
                    value={settings.default_resolution_height}
                    onChange={(e) =>
                      set(
                        "default_resolution_height",
                        parseInt(e.target.value) || 480,
                      )
                    }
                    className="w-28"
                  />
                </FieldGroup>
              </div>
              <Separator />
              <CheckboxField
                label="Start in fullscreen"
                checked={settings.fullscreen}
                onChange={(v) => set("fullscreen", v)}
              />
            </>
          )}

          {activeTab === "java" && (
            <>
              <FieldGroup
                label="Default JVM Arguments"
                hint="Used for new profiles"
              >
                <Input
                  value={settings.default_jvm_args}
                  onChange={(e) => set("default_jvm_args", e.target.value)}
                />
              </FieldGroup>
              <FieldGroup
                label="Custom Java Path"
                hint="Leave blank to auto-detect"
              >
                <Input
                  value={settings.custom_java_path || ""}
                  placeholder="/usr/lib/jvm/java-21/bin/java"
                  onChange={(e) =>
                    set("custom_java_path", e.target.value || null)
                  }
                />
              </FieldGroup>
            </>
          )}

          {activeTab === "directories" && (
            <FieldGroup
              label="Game Root Directory"
              hint="New profiles created here by default"
            >
              <Input
                value={settings.game_root_directory}
                onChange={(e) => set("game_root_directory", e.target.value)}
              />
            </FieldGroup>
          )}

          {activeTab === "network" && (
            <FieldGroup
              label="HTTP Proxy"
              hint="Leave blank for system settings. Restart required."
            >
              <Input
                value={settings.http_proxy || ""}
                placeholder="http://127.0.0.1:8080"
                onChange={(e) => set("http_proxy", e.target.value || null)}
              />
            </FieldGroup>
          )}

          {activeTab === "developer" && (
            <>
              <CheckboxField
                label="Show snapshots"
                checked={settings.show_snapshots}
                onChange={(v) => set("show_snapshots", v)}
              />
              <CheckboxField
                label="Show old beta"
                checked={settings.show_old_beta}
                onChange={(v) => set("show_old_beta", v)}
              />
              <CheckboxField
                label="Show old alpha"
                checked={settings.show_old_alpha}
                onChange={(v) => set("show_old_alpha", v)}
              />
              <Separator />
              <CheckboxField
                label="Verbose logging"
                checked={settings.verbose_logging}
                onChange={(v) => set("verbose_logging", v)}
              />
              <p className="text-xs text-muted-foreground pl-6">
                Restart required.
              </p>
            </>
          )}
        </div>

        {/* footer */}
        <div className="flex items-center gap-2 px-6 py-3 border-t border-border">
          <Button
            variant="destructive"
            size="sm"
            onClick={handleReset}
            disabled={saving}
            className="mr-auto"
          >
            Reset
          </Button>
          <Button size="sm" onClick={handleSave} disabled={!dirty || saving}>
            {saving ? "Saving…" : "Save"}
          </Button>
        </div>
      </div>
    </div>
  );
}

function FieldGroup({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1.5">
      <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        {label}
      </Label>
      {children}
      {hint && <p className="text-xs text-muted-foreground/70">{hint}</p>}
    </div>
  );
}

function CheckboxField({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex items-center gap-2.5 cursor-pointer text-sm">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="w-4 h-4 rounded border-border accent-primary"
      />
      {label}
    </label>
  );
}
