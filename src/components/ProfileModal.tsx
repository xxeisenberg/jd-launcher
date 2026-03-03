import React, { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type {
  Profile,
  Version,
  ModloaderVersion,
  JavaInstall,
  LauncherSettings,
} from "../bindings";
import { commands } from "../bindings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { XIcon, DownloadIcon } from "lucide-react";

interface ProfileModalProps {
  initial: Partial<Profile> | null;
  onSave: (profile: Profile) => void;
  onClose: () => void;
}

function generateId() {
  return crypto.randomUUID();
}

const DEFAULT_GAME_DIR_PREFIX = "~/.mc/instances/";

export function ProfileModal({ initial, onSave, onClose }: ProfileModalProps) {
  const [versions, setVersions] = useState<Version[]>([]);
  const [settings, setSettings] = useState<LauncherSettings | null>(null);
  const [loaderVersions, setLoaderVersions] = useState<ModloaderVersion[]>([]);
  const [loadingLoaders, setLoadingLoaders] = useState(false);
  const [systemJavas, setSystemJavas] = useState<JavaInstall[]>([]);
  const [downloadingJava, setDownloadingJava] = useState(false);
  const [downloadPhase, setDownloadPhase] = useState("");
  const [systemRamMb, setSystemRamMb] = useState(16384);
  const [form, setForm] = useState<Profile>(() => {
    const id = initial?.id ?? generateId();
    return {
      id,
      name: initial?.name ?? "New Profile",
      version: initial?.version ?? "",
      version_url: initial?.version_url ?? "",
      modloader: initial?.modloader ?? "none",
      modloader_version: initial?.modloader_version ?? null,
      game_dir: initial?.game_dir ?? `${DEFAULT_GAME_DIR_PREFIX}${id}`,
      java_path: initial?.java_path ?? null,
      jvm_args: initial?.jvm_args ?? "-Xmx2G -Xms512M",
      resolution: initial?.resolution ?? { width: 854, height: 480 },
    };
  });

  useEffect(() => {
    commands.getSettings().then(setSettings);
    commands.getSystemMemoryMb().then((mb) => {
      // Round down to nearest 512MB
      const rounded = Math.floor(mb / 512) * 512;
      setSystemRamMb(Math.max(1024, rounded));
    });
  }, []);

  const filterAndSortVersions = (
    allVersions: Version[],
    s: LauncherSettings | null,
  ) =>
    allVersions
      .filter((v) => {
        if (v.type === "release") return true;
        if (v.type === "snapshot" && s?.show_snapshots) return true;
        if (v.type === "old_beta" && s?.show_old_beta) return true;
        if (v.type === "old_alpha" && s?.show_old_alpha) return true;
        return false;
      })
      .sort(
        (a, b) =>
          new Date(b.releaseTime).getTime() - new Date(a.releaseTime).getTime(),
      );

  useEffect(() => {
    commands.getAvailableVersions().then((res) => {
      if (res.status === "ok") {
        const filtered = filterAndSortVersions(res.data, settings);
        setVersions(filtered);
        if (!form.version && filtered.length > 0) {
          const first = filtered[0];
          setForm((f) => ({ ...f, version: first.id, version_url: first.url }));
        }
      }
    });

    let unlisten: (() => void) | undefined;
    listen<{ versions: Version[] }>("minecraft-versions-updated", (event) => {
      setVersions(filterAndSortVersions(event.payload.versions, settings));
    }).then((un) => {
      unlisten = un;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, [settings]);

  useEffect(() => {
    if (form.modloader === "none" || !form.version) {
      setLoaderVersions([]);
      return;
    }
    setLoadingLoaders(true);
    commands
      .getModloaderVersions(form.modloader, form.version)
      .then((res) => {
        if (res.status === "ok") {
          setLoaderVersions(res.data);
          if (!form.modloader_version) {
            const stable = res.data.find((v) => v.stable);
            const first = stable ?? res.data[0];
            if (first)
              setForm((f) => ({ ...f, modloader_version: first.version }));
          }
        } else {
          setLoaderVersions([]);
        }
      })
      .finally(() => setLoadingLoaders(false));
  }, [form.modloader, form.version]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<{
      modloader: string;
      mc_version: string;
      versions: ModloaderVersion[];
    }>("modloader-versions-updated", (event) => {
      const { modloader, mc_version, versions: vs } = event.payload;
      if (form.modloader === modloader && form.version === mc_version) {
        setLoaderVersions(vs);
        setForm((f) => {
          if (!f.modloader_version) {
            const stable = vs.find((v) => v.stable);
            if (stable) return { ...f, modloader_version: stable.version };
          }
          return f;
        });
      }
    }).then((un) => {
      unlisten = un;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, [form.modloader, form.version]);

  const set = <K extends keyof Profile>(key: K, value: Profile[K]) =>
    setForm((f) => ({ ...f, [key]: value }));

  const handleVersionChange = (id: string) => {
    const v = versions.find((v) => v.id === id);
    if (v) setForm((f) => ({ ...f, version: v.id, version_url: v.url }));
  };

  const handleModloaderChange = (loader: string) => {
    setForm((f) => ({ ...f, modloader: loader, modloader_version: null }));
  };

  useEffect(() => {
    commands.detectSystemJavas().then(setSystemJavas);
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<{ completed: number; total: number; phase: string }>(
      "java-download-progress",
      (event) => {
        setDownloadPhase(
          `${event.payload.phase} (${event.payload.completed}%)`,
        );
      },
    ).then((un) => {
      unlisten = un;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const handleDownloadJava = async () => {
    if (!form.version) return;
    setDownloadingJava(true);
    setDownloadPhase("Detecting version…");
    try {
      const req = await commands.getRequiredJavaVersion(form.version);
      const res = await commands.downloadJava(req);
      if (res.status === "ok") set("java_path", res.data);
      else alert("Failed: " + res.error);
    } catch (e) {
      alert("Error: " + e);
    } finally {
      setDownloadingJava(false);
      setDownloadPhase("");
    }
  };

  const updateMemory = (mb: number) => {
    let args = form.jvm_args;
    if (/-Xmx\d+[GMmKgk]/.test(args)) {
      args = args.replace(/-Xmx\d+[GMmKgk]/, `-Xmx${mb}M`);
    } else {
      args = `-Xmx${mb}M ` + args;
    }
    set("jvm_args", args);
  };

  const getMemoryMb = () => {
    const match = form.jvm_args.match(/-Xmx(\d+)([GMmKgk])/);
    if (match) {
      const val = parseInt(match[1]);
      const unit = match[2].toUpperCase();
      if (unit === "G") return val * 1024;
      if (unit === "M") return val;
      if (unit === "K") return Math.max(1, Math.floor(val / 1024));
    }
    return 2048;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSave(form);
  };

  return (
    <div
      className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center"
      onClick={onClose}
    >
      <div
        className="bg-card border border-border rounded-xl w-[520px] max-w-[calc(100vw-40px)] max-h-[calc(100vh-80px)] flex flex-col shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        {/* header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-border">
          <h2 className="text-base font-semibold">
            {initial?.id ? "Edit Profile" : "New Profile"}
          </h2>
          <button
            onClick={onClose}
            className="text-muted-foreground hover:text-foreground"
          >
            <XIcon className="w-4 h-4" />
          </button>
        </div>

        {/* form */}
        <form
          onSubmit={handleSubmit}
          className="flex-1 overflow-y-auto p-5 space-y-4"
        >
          <Field label="Name">
            <Input
              value={form.name}
              onChange={(e) => set("name", e.target.value)}
              required
              placeholder="My Fabric 1.21.4"
            />
          </Field>

          <Field label="Minecraft Version">
            <Select value={form.version} onValueChange={handleVersionChange}>
              <SelectTrigger className="w-full h-9 text-sm">
                <SelectValue placeholder="Select version" />
              </SelectTrigger>
              <SelectContent>
                {versions.map((v) => {
                  const badge =
                    v.type === "release"
                      ? ""
                      : ` [${v.type === "snapshot" ? "Snapshot" : v.type === "old_beta" ? "Beta" : v.type === "old_alpha" ? "Alpha" : v.type}]`;
                  return (
                    <SelectItem key={v.id} value={v.id}>
                      {v.id}
                      {badge}
                    </SelectItem>
                  );
                })}
              </SelectContent>
            </Select>
          </Field>

          <div className="flex gap-3">
            <Field label="Modloader" className="flex-1">
              <Select
                value={form.modloader}
                onValueChange={handleModloaderChange}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">Vanilla</SelectItem>
                  <SelectItem value="fabric">Fabric</SelectItem>
                  <SelectItem value="forge">Forge</SelectItem>
                  <SelectItem value="neoforge">NeoForge</SelectItem>
                  <SelectItem value="quilt">Quilt</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            {form.modloader !== "none" && (
              <Field label="Loader Version" className="flex-1">
                {loadingLoaders ? (
                  <div className="h-9 flex items-center text-xs text-muted-foreground">
                    Loading…
                  </div>
                ) : loaderVersions.length > 0 ? (
                  <Select
                    value={form.modloader_version ?? ""}
                    onValueChange={(value) =>
                      set("modloader_version", value || null)
                    }
                  >
                    <SelectTrigger className="w-full h-9 text-sm">
                      <SelectValue placeholder="Select loader version" />
                    </SelectTrigger>
                    <SelectContent>
                      {loaderVersions.map((lv) => (
                        <SelectItem key={lv.version} value={lv.version}>
                          {lv.version}
                          {lv.stable ? " (stable)" : ""}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                ) : (
                  <div className="h-9 flex items-center text-xs text-muted-foreground">
                    No versions
                  </div>
                )}
              </Field>
            )}
          </div>

          <Field label="Game Directory">
            <Input
              value={form.game_dir}
              onChange={(e) => set("game_dir", e.target.value)}
              className="font-mono text-xs"
            />
          </Field>

          <Field label="Java Path" hint="Leave blank to auto-detect">
            <div className="flex gap-2">
              <Select
                value={form.java_path ?? "auto"}
                onValueChange={(value) =>
                  set("java_path", value === "auto" ? null : value)
                }
              >
                <SelectTrigger className="flex-1 h-9 text-sm w-full">
                  <SelectValue placeholder="Select Java path" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="auto">Auto-detect</SelectItem>
                  {systemJavas.map((j) => (
                    <SelectItem key={j.path} value={j.path}>
                      Java {j.version} ({j.path})
                    </SelectItem>
                  ))}
                  {form.java_path &&
                    !systemJavas.find((j) => j.path === form.java_path) && (
                      <SelectItem value={form.java_path}>
                        Custom ({form.java_path})
                      </SelectItem>
                    )}
                </SelectContent>
              </Select>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={handleDownloadJava}
                disabled={downloadingJava || !form.version}
              >
                <DownloadIcon className="w-3.5 h-3.5 mr-1.5" />
                {downloadingJava ? "…" : "Auto"}
              </Button>
            </div>
            {downloadPhase && (
              <p className="text-xs text-muted-foreground mt-1">
                {downloadPhase}
              </p>
            )}
            <Input
              value={form.java_path ?? ""}
              onChange={(e) => set("java_path", e.target.value || null)}
              placeholder="Or custom path…"
              className="mt-2 font-mono text-xs"
            />
          </Field>

          <Field label={`Memory: ${getMemoryMb()} MB`}>
            <input
              type="range"
              min={512}
              max={systemRamMb}
              step={512}
              value={getMemoryMb()}
              onChange={(e) => updateMemory(Number(e.target.value))}
              className="w-full accent-primary"
            />
          </Field>

          <Field label="JVM Arguments">
            <Input
              value={form.jvm_args}
              onChange={(e) => set("jvm_args", e.target.value)}
              className="font-mono text-xs"
            />
          </Field>

          <div className="flex gap-3">
            <Field label="Width" className="flex-1">
              <Input
                type="number"
                min={320}
                value={form.resolution.width}
                onChange={(e) =>
                  set("resolution", {
                    ...form.resolution,
                    width: Number(e.target.value),
                  })
                }
              />
            </Field>
            <Field label="Height" className="flex-1">
              <Input
                type="number"
                min={240}
                value={form.resolution.height}
                onChange={(e) =>
                  set("resolution", {
                    ...form.resolution,
                    height: Number(e.target.value),
                  })
                }
              />
            </Field>
          </div>

          {/* footer */}
          <div className="flex gap-2 justify-end pt-2 border-t border-border">
            <Button type="button" variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit">Save</Button>
          </div>
        </form>
      </div>
    </div>
  );
}

function Field({
  label,
  hint,
  className,
  children,
}: {
  label: string;
  hint?: string;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <div className={`space-y-1.5 ${className ?? ""}`}>
      <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        {label}
      </Label>
      {children}
      {hint && <p className="text-[11px] text-muted-foreground/70">{hint}</p>}
    </div>
  );
}
