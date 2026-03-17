import React, { useState, useEffect, useRef } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
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
import { Checkbox } from "@/components/ui/checkbox";
import { Kbd, KbdGroup } from "@/components/ui/kbd";
import { cn } from "@/lib/utils";
import { ACCENT_COLORS, FONTS, UI_STYLES } from "@/lib/themes";
import {
  KEYBIND_ACTIONS,
  KEYBIND_LABELS,
  getKeybinds,
  saveKeybinds,
  resetKeybinds,
  formatKeybind,
  eventToCombo,
  type KeybindAction,
} from "@/hooks/useKeybinds";
import * as LucideIcons from "lucide-react";

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
  | "groups"
  | "keybinds"
  | "developer";

const TABS: { key: TabKey; label: string }[] = [
  { key: "general", label: "General" },
  { key: "appearance", label: "Appearance" },
  { key: "game", label: "Game" },
  { key: "java", label: "Java" },
  { key: "directories", label: "Directories" },
  { key: "network", label: "Network" },
  { key: "groups", label: "Groups" },
  { key: "keybinds", label: "Keybinds" },
  { key: "developer", label: "Developer" },
];

const GROUP_ICONS = [
  "Folder", "FolderOpen", "Archive", "Box", "Boxes", "Package",
  "Star", "Heart", "Bookmark", "Tag", "Flame", "Zap",
  "Sword", "Shield", "Crosshair", "Compass", "Map", "Globe",
  "Gem", "Crown", "Ghost", "Skull", "Rocket", "Gamepad2",
  "Cylinder", "Hexagon", "Triangle", "Circle", "Square", "Cpu"
];

export function SettingsPage({ onSettingsSaved }: SettingsPageProps) {
  const qc = useQueryClient();
  const { data: loadedSettings } = useQuery({
    queryKey: ["settings"],
    queryFn: () => commands.getSettings(),
  });

  const [localSettings, setLocalSettings] = useState<LauncherSettings | null>(
    null,
  );
  const [activeTab, setActiveTab] = useState<TabKey>("general");
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);

  const settings = dirty ? localSettings : (loadedSettings ?? null);

  const handleSave = async () => {
    if (!settings) return;
    setSaving(true);
    const result = await commands.updateSettings(settings);
    setSaving(false);
    if (result.status === "ok") {
      setDirty(false);
      setLocalSettings(null);
      qc.invalidateQueries({ queryKey: ["settings"] });
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
      setDirty(false);
      setLocalSettings(null);
      await qc.invalidateQueries({ queryKey: ["settings"] });
      const s = await commands.getSettings();
      onSettingsSaved(s);
    }
    setSaving(false);
  };

  const set = (key: keyof LauncherSettings, value: any) => {
    if (!settings) return;
    const newSettings = { ...settings, [key]: value };
    setLocalSettings(newSettings);
    setDirty(true);
  };

  const handleUpdateGroup = (groupId: string, updates: any) => {
    if (!settings) return;
    const currentGroups = settings.groups || [];
    const newGroups = currentGroups.map((g) => (g.id === groupId ? { ...g, ...updates } : g));
    set("groups", newGroups);
  };

  const handleDeleteGroupOption = (groupId: string) => {
    if (!settings) return;
    const currentGroups = settings.groups || [];
    set("groups", currentGroups.filter((g) => g.id !== groupId));
  };

  const handleAddGroup = () => {
    if (!settings) return;
    const currentGroups = settings.groups || [];
    set("groups", [
      ...currentGroups,
      {
        id: crypto.randomUUID(),
        name: "New Group",
        color: "#6b7280",
        icon: "Folder",
      },
    ]);
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
              <CheckboxField
                label="Use native title bar"
                checked={settings.use_native_titlebar ?? false}
                onChange={(v) => set("use_native_titlebar", v)}
              />
              <p className="text-xs text-muted-foreground pl-6">
                Restart required.
              </p>
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
                  value={settings.accent_color ?? "Jade"}
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
                  value={settings.font_family ?? "Outfit"}
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
                  value={settings.ui_style ?? "Modern"}
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
                    value={settings.ui_scale ?? 100}
                    onChange={(e) => set("ui_scale", parseInt(e.target.value))}
                  />
                  <span className="text-sm font-medium w-12 text-right">
                    {settings.ui_scale ?? 100}%
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
              <Separator />
              <div className="space-y-1.5">
                <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
                  Maintenance
                </Label>
                <div className="flex flex-col gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    className="w-fit text-xs"
                    onClick={() => {
                      if (
                        confirm(
                          "This will clear onboarding state and reload the app. Continue?",
                        )
                      ) {
                        localStorage.removeItem("jd-launcher-onboarded");
                        localStorage.removeItem("jd-launcher-offline-username");
                        window.location.reload();
                      }
                    }}
                  >
                    Reset Onboarding
                  </Button>
                  <p className="text-xs text-muted-foreground/70">
                    Run the setup process again.
                  </p>
                </div>
              </div>
            </>
          )}

          {activeTab === "groups" && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-sm font-semibold">Custom Groups</h3>
                  <p className="text-xs text-muted-foreground mt-0.5">
                    Create groups to organize your instances.
                  </p>
                </div>
                <Button size="sm" onClick={handleAddGroup}>
                  <LucideIcons.PlusIcon className="w-4 h-4 mr-2" /> Add Group
                </Button>
              </div>
              <Separator />
              <div className="space-y-3">
                {(!settings.groups || settings.groups.length === 0) ? (
                  <p className="text-xs text-muted-foreground italic text-center py-4">
                    No custom groups yet.
                  </p>
                ) : (
                  settings.groups.map((group) => {
                    const IconComponent = (LucideIcons as any)[group.icon] || LucideIcons.FolderIcon;
                    return (
                      <div
                        key={group.id}
                        className="flex flex-col gap-3 p-3 rounded-lg border bg-card/50"
                      >
                        <div className="flex items-center gap-3">
                          <div className="flex items-center justify-center w-10 h-10 rounded-md bg-background border border-border shadow-sm shrink-0">
                            <IconComponent className="w-5 h-5" style={{ color: group.color }} />
                          </div>
                          <div className="flex-1 space-y-1">
                            <Label className="text-xs">Group Name</Label>
                            <Input
                              value={group.name}
                              onChange={(e) =>
                                handleUpdateGroup(group.id, { name: e.target.value })
                              }
                              className="h-8"
                            />
                          </div>
                          <div className="space-y-1">
                            <Label className="text-xs">Color</Label>
                            <div className="flex items-center gap-2 mt-1">
                              {Object.values(ACCENT_COLORS).slice(0, 10).map((c, i) => (
                                <button
                                  key={i}
                                  onClick={() => handleUpdateGroup(group.id, { color: c.swatch })}
                                  className={`w-6 h-6 rounded-full border border-border transition-transform hover:scale-110 ${
                                    group.color === c.swatch ? "ring-2 ring-primary ring-offset-1 ring-offset-background" : ""
                                  }`}
                                  style={{ background: c.swatch }}
                                />
                              ))}
                            </div>
                          </div>
                          <div className="space-y-1">
                            <Label className="text-xs">Icon</Label>
                            <Select
                              value={group.icon}
                              onValueChange={(v) => handleUpdateGroup(group.id, { icon: v })}
                            >
                              <SelectTrigger className="w-[120px] h-8">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                <div className="grid grid-cols-5 gap-1 p-1">
                                  {GROUP_ICONS.map((iconName) => {
                                    const I = (LucideIcons as any)[iconName];
                                    if (!I) return null;
                                    return (
                                      <SelectItem
                                        key={iconName}
                                        value={iconName}
                                        hideIndicator
                                        className="h-8 w-8 p-0 flex items-center justify-center cursor-pointer data-[state=checked]:bg-primary/20"
                                      >
                                        <div className="flex items-center justify-center w-full h-full">
                                          <I className="w-4 h-4" />
                                        </div>
                                      </SelectItem>
                                    );
                                  })}
                                </div>
                              </SelectContent>
                            </Select>
                          </div>
                          <div className="pt-5 -mt-1 ml-2">
                            <Button
                              variant="ghost"
                              size="icon"
                              className="text-destructive hover:text-destructive shrink-0"
                              onClick={() => handleDeleteGroupOption(group.id)}
                            >
                              <LucideIcons.TrashIcon className="w-4 h-4" />
                            </Button>
                          </div>
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            </div>
          )}

          {activeTab === "keybinds" && (
            <KeybindsPanel />
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
  const id = React.useId();
  return (
    <div className="flex items-center gap-2.5">
      <Checkbox
        id={id}
        checked={checked}
        onCheckedChange={(v) => onChange(!!v)}
      />
      <Label
        htmlFor={id}
        className="text-sm font-normal leading-none cursor-pointer"
      >
        {label}
      </Label>
    </div>
  );
}

function KeybindsPanel() {
  const [keybinds, setKeybinds] = useState(getKeybinds);
  const [recording, setRecording] = useState<KeybindAction | null>(null);
  const recorderRef = useRef<HTMLButtonElement | null>(null);

  // Listen for keypress while recording
  useEffect(() => {
    if (!recording) return;
    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const combo = eventToCombo(e);
      if (!combo) return;
      const next = { ...keybinds, [recording]: combo };
      setKeybinds(next);
      saveKeybinds(next);
      setRecording(null);
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [recording, keybinds]);

  // Focus the recorder button
  useEffect(() => {
    if (recording && recorderRef.current) recorderRef.current.focus();
  }, [recording]);

  const conflicts = findConflicts(keybinds);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-semibold">Keyboard Shortcuts</h3>
          <p className="text-xs text-muted-foreground mt-0.5">
            Click a shortcut to rebind it.
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            resetKeybinds();
            setKeybinds(getKeybinds());
          }}
        >
          Reset to Defaults
        </Button>
      </div>
      <Separator />
      <div className="space-y-1">
        {KEYBIND_ACTIONS.map((action) => {
          const isRecording = recording === action;
          const hasConflict = conflicts.has(action);
          return (
            <div
              key={action}
              className="flex items-center justify-between px-3 py-2 rounded-md hover:bg-accent/50 transition-colors"
            >
              <span className="text-sm">{KEYBIND_LABELS[action]}</span>
              <div className="flex items-center gap-2">
                {hasConflict && !isRecording && (
                  <span className="text-[10px] text-amber-500 font-medium">
                    Conflict
                  </span>
                )}
                <button
                  ref={isRecording ? recorderRef : undefined}
                  onClick={() => setRecording(isRecording ? null : action)}
                  className="cursor-pointer"
                >
                  {isRecording ? (
                    <Kbd className="min-w-[100px] animate-pulse border-primary bg-primary/10 text-primary">
                      Press keys…
                    </Kbd>
                  ) : (
                    <KbdGroup
                      className={cn(
                        hasConflict && "text-amber-500",
                      )}
                    >
                      {formatKeybind(keybinds[action]).split(" + ").map((key, i) => (
                        <Kbd
                          key={i}
                          className={cn(
                            hasConflict && "bg-amber-500/10 text-amber-500",
                          )}
                        >
                          {key}
                        </Kbd>
                      ))}
                    </KbdGroup>
                  )}
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function findConflicts(
  keybinds: Record<KeybindAction, string>,
): Set<KeybindAction> {
  const seen = new Map<string, KeybindAction>();
  const conflicts = new Set<KeybindAction>();
  for (const action of KEYBIND_ACTIONS) {
    const combo = keybinds[action];
    if (seen.has(combo)) {
      conflicts.add(action);
      conflicts.add(seen.get(combo)!);
    } else {
      seen.set(combo, action);
    }
  }
  return conflicts;
}
