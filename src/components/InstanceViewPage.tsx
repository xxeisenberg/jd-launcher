import { type ReactNode, useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  InstalledContentPack,
  InstalledMod,
  LocalScreenshot,
  LocalWorld,
  Profile,
  commands,
} from "../bindings";
import { Button } from "@/components/ui/button";
import {
  ArrowLeftIcon,
  CalendarIcon,
  ExternalLinkIcon,
  FileBoxIcon,
  FolderOpenIcon,
  HardDriveIcon,
  ImageIcon,
  MonitorIcon,
  PaletteIcon,
  PlusIcon,
  SettingsIcon,
  PlayIcon,
  Trash2Icon,
  StarIcon,
} from "lucide-react";
import * as LucideIcons from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { Input } from "@/components/ui/input";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";

interface InstanceViewPageProps {
  profile: Profile;
  onBack: () => void;
  onLaunch: (profile: Profile) => void;
  onEdit: (profile: Profile) => void;
  onBrowseModrinth?: (type: "mod" | "shader" | "resourcepack") => void;
}

type SaveState = "idle" | "saving" | "saved" | "error";

function getMemoryMb(jvmArgs: string) {
  const match = jvmArgs.match(/-Xmx(\d+)([GMmKgk])/);
  if (!match) return 2048;

  const value = Number(match[1]);
  const unit = match[2].toUpperCase();
  if (unit === "G") return value * 1024;
  if (unit === "K") return Math.max(1, Math.floor(value / 1024));
  return value;
}

function withUpdatedMemory(jvmArgs: string, mb: number) {
  if (/-Xmx\d+[GMmKgk]/.test(jvmArgs)) {
    return jvmArgs.replace(/-Xmx\d+[GMmKgk]/, `-Xmx${mb}M`);
  }
  return `-Xmx${mb}M ${jvmArgs}`.trim();
}

function overviewSignature(profile: Profile) {
  return `${profile.jvm_args}|${profile.resolution.width}x${profile.resolution.height}`;
}

function formatBytes(bytes: number) {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const index = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  const value = bytes / 1024 ** index;
  return `${value >= 10 || index === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[index]}`;
}

function formatDate(timestamp: number) {
  if (!timestamp) return "Unknown";
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(timestamp);
}

function filenameLabel(name: string) {
  return name.replace(".disabled", "");
}

export function InstanceViewPage({
  profile,
  onBack,
  onLaunch,
  onEdit,
  onBrowseModrinth,
}: InstanceViewPageProps) {
  const qc = useQueryClient();
  const [activeTab, setActiveTab] = useState("mods");
  const [overviewDraft, setOverviewDraft] = useState(profile);
  const [lastSavedOverview, setLastSavedOverview] = useState(
    overviewSignature(profile),
  );
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [actionError, setActionError] = useState<string | null>(null);
  const [deleteDialog, setDeleteDialog] = useState<{
    isOpen: boolean;
    subfolder: string;
    name: string;
  }>({ isOpen: false, subfolder: "", name: "" });

  const invalidateContent = () => {
    qc.invalidateQueries({ queryKey: ["instanceDetails", profile.id] });
  };

  useEffect(() => {
    setOverviewDraft(profile);
    setLastSavedOverview(overviewSignature(profile));
    setSaveState("idle");
  }, [profile]);

  useEffect(() => {
    const signature = overviewSignature(overviewDraft);
    if (signature === lastSavedOverview) return;

    const timeout = window.setTimeout(async () => {
      setSaveState("saving");
      const result = await commands.saveProfile(overviewDraft);
      if (result.status === "ok") {
        setLastSavedOverview(signature);
        setSaveState("saved");
        qc.invalidateQueries({ queryKey: ["profiles"] });
        window.setTimeout(() => {
          setSaveState((current) => (current === "saved" ? "idle" : current));
        }, 1200);
      } else {
        setSaveState("error");
        setActionError(result.error);
      }
    }, 350);

    return () => window.clearTimeout(timeout);
  }, [lastSavedOverview, overviewDraft, qc]);

  const { data: systemMemoryMb = 16384 } = useQuery({
    queryKey: ["systemMemoryMb"],
    queryFn: async () => {
      const mb = await commands.getSystemMemoryMb();
      return Math.max(1024, Math.floor(mb / 512) * 512);
    },
  });

  const { data: settings } = useQuery({
    queryKey: ["settings"],
    queryFn: commands.getSettings,
  });

  const { data: details, isLoading: loading } = useQuery({
    queryKey: ["instanceDetails", profile.id],
    staleTime: 0,
    queryFn: async () => {
      const [modsRes, shadersRes, rpRes, worldsRes, screenshotsRes] =
        await Promise.all([
          commands.listMods(profile.id),
          commands.listShaders(profile.id),
          commands.listResourcePacks(profile.id),
          commands.listWorlds(profile.id),
          commands.listScreenshots(profile.id),
        ]);
      return {
        mods: modsRes.status === "ok" ? modsRes.data : [],
        shaders: shadersRes.status === "ok" ? shadersRes.data : [],
        resourcePacks: rpRes.status === "ok" ? rpRes.data : [],
        worlds: worldsRes.status === "ok" ? worldsRes.data : [],
        screenshots: screenshotsRes.status === "ok" ? screenshotsRes.data : [],
      };
    },
  });

  const mods = details?.mods ?? [];
  const shaders = details?.shaders ?? [];
  const resourcePacks = details?.resourcePacks ?? [];
  const worlds = details?.worlds ?? [];
  const screenshots = details?.screenshots ?? [];

  const modLabel =
    profile.modloader === "none"
      ? "Vanilla"
      : profile.modloader.charAt(0).toUpperCase() + profile.modloader.slice(1);

  const handleToggle = async (subfolder: string, name: string) => {
    const result = await commands.toggleContent(profile.id, subfolder, name);
    if (result.status === "error") {
      setActionError(result.error);
      return;
    }
    invalidateContent();
  };

  const handleOpenFolder = async (subfolder: string) => {
    const result = await commands.openContentFolder(profile.id, subfolder);
    if (result.status === "error") setActionError(result.error);
  };

  const handleOpenEntry = async (subfolder: string, name: string) => {
    const result = await commands.openContentEntry(profile.id, subfolder, name);
    if (result.status === "error") setActionError(result.error);
  };

  const handleRevealEntry = async (subfolder: string, name: string) => {
    const result = await commands.revealContentEntry(
      profile.id,
      subfolder,
      name,
    );
    if (result.status === "error") setActionError(result.error);
  };

  const confirmDelete = async () => {
    const { subfolder, name } = deleteDialog;
    if (!subfolder || !name) return;

    const result = await commands.deleteContent(profile.id, subfolder, name);
    if (result.status === "error") {
      setActionError(result.error);
      return;
    }

    invalidateContent();
    setDeleteDialog({ isOpen: false, subfolder: "", name: "" });
  };

  const toggleFavorite = async () => {
    const updated = { ...overviewDraft, favorite: !overviewDraft.favorite };
    setOverviewDraft(updated);
    const result = await commands.saveProfile(updated);
    if (result.status === "ok") {
      setLastSavedOverview(overviewSignature(updated));
      qc.invalidateQueries({ queryKey: ["profiles"] });
    } else {
      setActionError(result.error);
      setOverviewDraft(overviewDraft); // revert on error
    }
  };

  let toolbar: ReactNode = null;
  if (
    (activeTab === "mods" ||
      activeTab === "shaders" ||
      activeTab === "resourcepacks") &&
    onBrowseModrinth
  ) {
    toolbar = (
      <Button
        size="sm"
        className="gap-2"
        onClick={() =>
          onBrowseModrinth(
            activeTab === "resourcepacks"
              ? "resourcepack"
              : (activeTab.replace(/s$/, "") as "mod" | "shader"),
          )
        }
      >
        <PlusIcon className="w-4 h-4" /> Add{" "}
        {activeTab === "mods"
          ? "Mod"
          : activeTab === "shaders"
            ? "Shader"
            : "Resource Pack"}
      </Button>
    );
  } else if (activeTab === "worlds") {
    toolbar = (
      <Button
        size="sm"
        variant="outline"
        className="gap-2"
        onClick={() => handleOpenFolder("saves")}
      >
        <FolderOpenIcon className="w-4 h-4" /> Open Saves Folder
      </Button>
    );
  } else if (activeTab === "screenshots") {
    toolbar = (
      <Button
        size="sm"
        variant="outline"
        className="gap-2"
        onClick={() => handleOpenFolder("screenshots")}
      >
        <FolderOpenIcon className="w-4 h-4" /> Open Screenshots Folder
      </Button>
    );
  }

  const memoryMb = getMemoryMb(overviewDraft.jvm_args);

  return (
    <div className="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background">
      <div className="relative h-24 shrink-0 bg-[radial-gradient(circle_at_top_left,hsl(var(--primary)/0.22),transparent_45%),linear-gradient(135deg,hsl(var(--primary)/0.12),transparent_65%)] min-[900px]:h-28 min-[1180px]:h-32">
        <div className="absolute inset-x-0 bottom-0 h-16 bg-linear-to-t from-background to-transparent" />
      </div>

      <div className="relative z-10 -mt-10 flex shrink-0 gap-4 border-b px-4 pb-4 min-[900px]:-mt-11 min-[900px]:gap-5 min-[900px]:px-6 min-[900px]:pb-5 min-[1180px]:-mt-12 min-[1180px]:gap-6 min-[1180px]:px-8 min-[1180px]:pb-6">
        <div className="flex h-20 w-20 shrink-0 items-center justify-center rounded-2xl border-4 border-background bg-card shadow-md shadow-primary/10 min-[1180px]:h-24 min-[1180px]:w-24 min-[1180px]:rounded-3xl">
          <span className="text-4xl font-bold text-primary">
            {profile.name.charAt(0).toUpperCase()}
          </span>
        </div>
        <div className="flex min-w-0 flex-1 flex-col justify-end pb-1">
          <div className="flex flex-wrap items-end justify-between gap-3">
            <div className="min-w-0 flex-1">
              <div className="flex min-w-0 items-center gap-2 min-[1180px]:gap-3">
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 -ml-2 rounded-full"
                  onClick={onBack}
                >
                  <ArrowLeftIcon className="w-4 h-4" />
                </Button>
                <h1 className="min-w-0 truncate text-xl font-bold tracking-tight min-[1180px]:text-2xl">
                  {profile.name}
                </h1>
                <button
                  className="p-1 rounded-md hover:bg-accent transition-colors"
                  onClick={toggleFavorite}
                  title={overviewDraft.favorite ? "Remove from favorites" : "Add to favorites"}
                >
                  <StarIcon className={`w-5 h-5 ${overviewDraft.favorite ? "fill-yellow-400 text-yellow-400" : "text-muted-foreground/30 hover:text-yellow-400"}`} />
                </button>
                {(() => {
                  if (!profile.group) return null;
                  const customGroup = settings?.groups?.find((g) => g.id === profile.group || g.name === profile.group);
                  if (customGroup) {
                    const IconComponent = (LucideIcons as any)[customGroup.icon] || LucideIcons.FolderIcon;
                    return (
                      <Badge variant="outline" className="text-xs flex items-center gap-1.5 px-2 py-0.5" style={{ borderColor: customGroup.color + "40", backgroundColor: customGroup.color + "10" }}>
                        <IconComponent className="w-3 h-3" style={{ color: customGroup.color }} />
                        <span style={{ color: customGroup.color }}>{customGroup.name}</span>
                      </Badge>
                    );
                  }
                  return (
                    <Badge variant="outline" className="text-xs">
                      {profile.group}
                    </Badge>
                  );
                })()}
              </div>
              <div className="flex items-center gap-2 mt-2 flex-wrap">
                <Badge variant="secondary" className="font-mono text-xs">
                  {profile.version}
                </Badge>
                <Badge variant="outline" className="text-xs">
                  {modLabel} {profile.modloader_version || ""}
                </Badge>
                {saveState !== "idle" && (
                  <span
                    className={cn(
                      "text-xs",
                      saveState === "error"
                        ? "text-destructive"
                        : "text-muted-foreground",
                    )}
                  >
                    {saveState === "saving"
                      ? "Saving overview..."
                      : saveState === "saved"
                        ? "Overview saved"
                        : "Save failed"}
                  </span>
                )}
              </div>
            </div>
            <div className="flex shrink-0 gap-2">
              <Button
                variant="outline"
                className="gap-2"
                onClick={() => onEdit(profile)}
              >
                <SettingsIcon className="w-4 h-4" /> Edit Settings
              </Button>
              <Button className="gap-2" onClick={() => onLaunch(profile)}>
                <PlayIcon className="w-4 h-4" /> Play
              </Button>
            </div>
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-hidden px-4 py-4 min-[900px]:px-6 min-[900px]:py-5 min-[1180px]:px-8 min-[1180px]:py-6">
        <Tabs
          value={activeTab}
          onValueChange={setActiveTab}
          className="h-full flex flex-col"
        >
          <div className="flex min-w-0 items-center gap-3">
            <TabsList className="min-w-0 flex-1 w-auto justify-start overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
              <TabsTrigger value="mods" className="gap-2">
                <FileBoxIcon className="w-4 h-4" /> Mods
                <Badge
                  variant="secondary"
                  className="ml-1 opacity-60 px-1 py-0 text-[10px] h-4 min-w-[1rem] flex items-center justify-center"
                >
                  {mods.length}
                </Badge>
              </TabsTrigger>
              <TabsTrigger value="shaders" className="gap-2">
                <ImageIcon className="w-4 h-4" /> Shaders
                <Badge
                  variant="secondary"
                  className="ml-1 opacity-60 px-1 py-0 text-[10px] h-4 min-w-[1rem] flex items-center justify-center"
                >
                  {shaders.length}
                </Badge>
              </TabsTrigger>
              <TabsTrigger value="resourcepacks" className="gap-2">
                <PaletteIcon className="w-4 h-4" /> Resource Packs
                <Badge
                  variant="secondary"
                  className="ml-1 opacity-60 px-1 py-0 text-[10px] h-4 min-w-[1rem] flex items-center justify-center"
                >
                  {resourcePacks.length}
                </Badge>
              </TabsTrigger>
              <TabsTrigger value="worlds" className="gap-2">
                <FolderOpenIcon className="w-4 h-4" /> Worlds
                <Badge
                  variant="secondary"
                  className="ml-1 opacity-60 px-1 py-0 text-[10px] h-4 min-w-[1rem] flex items-center justify-center"
                >
                  {worlds.length}
                </Badge>
              </TabsTrigger>
              <TabsTrigger value="screenshots" className="gap-2">
                <ImageIcon className="w-4 h-4" /> Screenshots
                <Badge
                  variant="secondary"
                  className="ml-1 opacity-60 px-1 py-0 text-[10px] h-4 min-w-[1rem] flex items-center justify-center"
                >
                  {screenshots.length}
                </Badge>
              </TabsTrigger>
              <TabsTrigger value="settings" className="gap-2">
                <SettingsIcon className="w-4 h-4" /> Overview
              </TabsTrigger>
            </TabsList>
            <div className="shrink-0">{toolbar}</div>
          </div>

          {actionError && (
            <div className="mt-4 rounded-lg border border-destructive/20 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {actionError}
            </div>
          )}

          <div className="relative mt-4 min-h-0 flex-1 overflow-hidden rounded-xl border bg-card min-[1180px]:mt-6 min-[1180px]:rounded-2xl">
            <TabsContent
              value="mods"
              className="absolute inset-0 m-0 p-0 focus-visible:outline-none data-[state=inactive]:hidden"
            >
              <ScrollArea className="h-full">
                <div className="p-4 min-[1180px]:p-6 flex flex-col gap-3">
                  {loading ? (
                    <EmptyState icon={FileBoxIcon} title="Loading mods..." />
                  ) : mods.length === 0 ? (
                    <EmptyState
                      icon={FileBoxIcon}
                      title="No mods installed"
                      subtitle="Place .jar files in this instance's mods folder."
                    />
                  ) : (
                    mods.map((mod) => (
                      <ModRow
                        key={mod.file_name}
                        mod={mod}
                        onToggle={() => handleToggle("mods", mod.file_name)}
                        onDelete={() =>
                          setDeleteDialog({
                            isOpen: true,
                            subfolder: "mods",
                            name: mod.file_name,
                          })
                        }
                        onReveal={() =>
                          handleRevealEntry("mods", mod.file_name)
                        }
                      />
                    ))
                  )}
                </div>
              </ScrollArea>
            </TabsContent>

            <TabsContent
              value="shaders"
              className="absolute inset-0 m-0 p-0 focus-visible:outline-none data-[state=inactive]:hidden"
            >
              <ScrollArea className="h-full">
                <div className="p-4 min-[1180px]:p-6 flex flex-col gap-2">
                  {loading ? (
                    <EmptyState icon={ImageIcon} title="Loading shaders..." />
                  ) : shaders.length === 0 ? (
                    <EmptyState
                      icon={ImageIcon}
                      title="No shaders installed"
                      subtitle="Place files or folders in this instance's shaderpacks folder."
                    />
                  ) : (
                    shaders.map((shader) => (
                      <ContentPackRow
                        key={shader.file_name}
                        pack={shader}
                        icon={<ImageIcon className="w-5 h-5 text-primary/70" />}
                        onToggle={() =>
                          handleToggle("shaderpacks", shader.file_name)
                        }
                        onDelete={() =>
                          setDeleteDialog({
                            isOpen: true,
                            subfolder: "shaderpacks",
                            name: shader.file_name,
                          })
                        }
                        onReveal={() =>
                          handleRevealEntry("shaderpacks", shader.file_name)
                        }
                      />
                    ))
                  )}
                </div>
              </ScrollArea>
            </TabsContent>

            <TabsContent
              value="resourcepacks"
              className="absolute inset-0 m-0 p-0 focus-visible:outline-none data-[state=inactive]:hidden"
            >
              <ScrollArea className="h-full">
                <div className="p-4 min-[1180px]:p-6 flex flex-col gap-2">
                  {loading ? (
                    <EmptyState
                      icon={PaletteIcon}
                      title="Loading resource packs..."
                    />
                  ) : resourcePacks.length === 0 ? (
                    <EmptyState
                      icon={PaletteIcon}
                      title="No resource packs installed"
                      subtitle="Place files or folders in this instance's resourcepacks folder."
                    />
                  ) : (
                    resourcePacks.map((pack) => (
                      <ContentPackRow
                        key={pack.file_name}
                        pack={pack}
                        icon={
                          <PaletteIcon className="w-5 h-5 text-primary/70" />
                        }
                        onToggle={() =>
                          handleToggle("resourcepacks", pack.file_name)
                        }
                        onDelete={() =>
                          setDeleteDialog({
                            isOpen: true,
                            subfolder: "resourcepacks",
                            name: pack.file_name,
                          })
                        }
                        onReveal={() =>
                          handleRevealEntry("resourcepacks", pack.file_name)
                        }
                      />
                    ))
                  )}
                </div>
              </ScrollArea>
            </TabsContent>

            <TabsContent
              value="worlds"
              className="absolute inset-0 m-0 p-0 focus-visible:outline-none data-[state=inactive]:hidden"
            >
              <ScrollArea className="h-full">
                <div className="p-4 min-[1180px]:p-6 flex flex-col gap-3">
                  {loading ? (
                    <EmptyState
                      icon={FolderOpenIcon}
                      title="Scanning saved worlds..."
                    />
                  ) : worlds.length === 0 ? (
                    <EmptyState
                      icon={FolderOpenIcon}
                      title="No saved worlds found"
                      subtitle="Single-player saves for this instance will appear here."
                    />
                  ) : (
                    worlds.map((world) => (
                      <WorldRow
                        key={world.folder_name}
                        world={world}
                        onOpen={() =>
                          handleOpenEntry("saves", world.folder_name)
                        }
                        onReveal={() =>
                          handleRevealEntry("saves", world.folder_name)
                        }
                        onDelete={() =>
                          setDeleteDialog({
                            isOpen: true,
                            subfolder: "saves",
                            name: world.folder_name,
                          })
                        }
                      />
                    ))
                  )}
                </div>
              </ScrollArea>
            </TabsContent>

            <TabsContent
              value="screenshots"
              className="absolute inset-0 m-0 p-0 focus-visible:outline-none data-[state=inactive]:hidden"
            >
              <ScrollArea className="h-full">
                <div className="p-4 min-[1180px]:p-6">
                  {loading ? (
                    <EmptyState
                      icon={ImageIcon}
                      title="Loading screenshots..."
                    />
                  ) : screenshots.length === 0 ? (
                    <EmptyState
                      icon={ImageIcon}
                      title="No screenshots captured yet"
                      subtitle="Screenshots taken in this instance will show up here."
                    />
                  ) : (
                    <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-4">
                      {screenshots.map((shot) => (
                        <ScreenshotCard
                          key={shot.path}
                          screenshot={shot}
                          onOpen={() =>
                            handleOpenEntry("screenshots", shot.file_name)
                          }
                          onReveal={() =>
                            handleRevealEntry("screenshots", shot.file_name)
                          }
                          onDelete={() =>
                            setDeleteDialog({
                              isOpen: true,
                              subfolder: "screenshots",
                              name: shot.file_name,
                            })
                          }
                        />
                      ))}
                    </div>
                  )}
                </div>
              </ScrollArea>
            </TabsContent>

            <TabsContent
              value="settings"
              className="absolute inset-0 m-0 focus-visible:outline-none data-[state=inactive]:hidden"
            >
              <ScrollArea className="h-full">
                <div className="grid gap-4 p-4 min-[1180px]:gap-6 min-[1180px]:p-6 xl:grid-cols-[1.1fr_0.9fr]">
                  <section className="border rounded-2xl p-5 bg-background/60">
                    <div className="flex items-center justify-between gap-4 mb-5">
                      <div>
                        <p className="text-xs uppercase tracking-[0.24em] text-muted-foreground">
                          Launch Profile
                        </p>
                        <h3 className="text-lg font-semibold">Quick Tweaks</h3>
                      </div>
                      <Badge variant="outline" className="font-mono">
                        {overviewDraft.resolution.width} x{" "}
                        {overviewDraft.resolution.height}
                      </Badge>
                    </div>

                    <div className="space-y-6">
                      <div className="rounded-2xl border border-border/70 bg-card/60 p-4">
                        <div className="flex items-center justify-between gap-4 mb-3">
                          <div>
                            <p className="text-sm font-semibold">Memory</p>
                            <p className="text-xs text-muted-foreground">
                              Adjust the maximum heap passed to Java.
                            </p>
                          </div>
                          <Badge className="font-mono">{memoryMb} MB</Badge>
                        </div>
                        <input
                          type="range"
                          min={512}
                          max={systemMemoryMb}
                          step={512}
                          value={Math.min(memoryMb, systemMemoryMb)}
                          onChange={(e) =>
                            setOverviewDraft((current) => ({
                              ...current,
                              jvm_args: withUpdatedMemory(
                                current.jvm_args,
                                Number(e.target.value),
                              ),
                            }))
                          }
                          className="w-full accent-primary"
                        />
                        <div className="mt-2 flex items-center justify-between text-xs text-muted-foreground">
                          <span>512 MB</span>
                          <span>System ceiling {systemMemoryMb} MB</span>
                        </div>
                      </div>

                      <div className="rounded-2xl border border-border/70 bg-card/60 p-4">
                        <div className="flex items-center gap-2 mb-4">
                          <MonitorIcon className="w-4 h-4 text-primary" />
                          <div>
                            <p className="text-sm font-semibold">
                              Window Resolution
                            </p>
                            <p className="text-xs text-muted-foreground">
                              Saved automatically as you edit.
                            </p>
                          </div>
                        </div>
                        <div className="grid gap-3 sm:grid-cols-2">
                          <label className="space-y-1.5">
                            <span className="text-xs uppercase tracking-[0.18em] text-muted-foreground">
                              Width
                            </span>
                            <Input
                              type="number"
                              min={320}
                              value={overviewDraft.resolution.width}
                              onChange={(e) =>
                                setOverviewDraft((current) => ({
                                  ...current,
                                  resolution: {
                                    ...current.resolution,
                                    width: Math.max(
                                      320,
                                      Number(e.target.value) || 320,
                                    ),
                                  },
                                }))
                              }
                            />
                          </label>
                          <label className="space-y-1.5">
                            <span className="text-xs uppercase tracking-[0.18em] text-muted-foreground">
                              Height
                            </span>
                            <Input
                              type="number"
                              min={240}
                              value={overviewDraft.resolution.height}
                              onChange={(e) =>
                                setOverviewDraft((current) => ({
                                  ...current,
                                  resolution: {
                                    ...current.resolution,
                                    height: Math.max(
                                      240,
                                      Number(e.target.value) || 240,
                                    ),
                                  },
                                }))
                              }
                            />
                          </label>
                        </div>
                      </div>
                    </div>
                  </section>

                  <section className="border rounded-2xl p-5 bg-background/60">
                    <h3 className="text-sm font-semibold mb-4 text-muted-foreground">
                      Paths & Identifiers
                    </h3>
                    <div className="space-y-4">
                      <InfoBlock label="Profile ID" value={profile.id} mono />
                      <InfoBlock
                        label="Game Directory"
                        value={profile.game_dir}
                        mono
                      />
                      <InfoBlock
                        label="Custom Java Path"
                        value={profile.java_path || "System Default"}
                        mono
                      />
                      <InfoBlock
                        label="JVM Arguments"
                        value={overviewDraft.jvm_args || "(None)"}
                        mono
                      />
                      <div className="flex gap-2 pt-2">
                        <Button
                          variant="outline"
                          className="gap-2"
                          onClick={() => handleOpenFolder(".")}
                        >
                          <FolderOpenIcon className="w-4 h-4" /> Open Instance
                          Folder
                        </Button>
                      </div>
                    </div>
                  </section>
                </div>
              </ScrollArea>
            </TabsContent>
          </div>
        </Tabs>
      </div>

      <AlertDialog
        open={deleteDialog.isOpen}
        onOpenChange={(isOpen) =>
          setDeleteDialog((current) => ({ ...current, isOpen }))
        }
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              Delete {filenameLabel(deleteDialog.name)}?
            </AlertDialogTitle>
            <AlertDialogDescription>
              This permanently removes {filenameLabel(deleteDialog.name)} from
              the instance.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={confirmDelete}>
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

function EmptyState({
  icon: Icon,
  title,
  subtitle,
}: {
  icon: typeof FileBoxIcon;
  title: string;
  subtitle?: string;
}) {
  return (
    <div className="flex flex-col items-center justify-center h-52 text-muted-foreground text-center">
      <Icon className="w-9 h-9 opacity-20 mb-3" />
      <p className="text-sm font-medium">{title}</p>
      {subtitle && <p className="text-xs mt-1 max-w-sm">{subtitle}</p>}
    </div>
  );
}

function ContentPackRow({
  pack,
  icon,
  onToggle,
  onDelete,
  onReveal,
}: {
  pack: InstalledContentPack;
  icon: ReactNode;
  onToggle: () => void;
  onDelete: () => void;
  onReveal: () => void;
}) {
  const iconSrc = pack.icon_data_url || pack.icon_url;
  return (
    <div className="group rounded-2xl border bg-background px-4 py-3 hover:border-primary/40 transition-colors">
      <div className="flex items-start gap-4">
        <div
          className={cn(
            "mt-0.5 flex h-12 w-12 items-center justify-center overflow-hidden rounded-2xl border bg-card",
            !pack.enabled && "opacity-50",
          )}
        >
          {iconSrc ? (
            <img
              src={iconSrc}
              alt={pack.display_name}
              className="h-full w-full object-cover"
            />
          ) : (
            icon
          )}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <p
                className={cn(
                  "text-sm font-semibold truncate",
                  !pack.enabled &&
                    "text-muted-foreground line-through opacity-60",
                )}
              >
                {pack.display_name}
              </p>
              <p className="text-xs text-muted-foreground font-mono truncate mt-1">
                {pack.file_name}
              </p>
            </div>
            <div className="adaptive-actions flex shrink-0 items-center gap-2 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8"
                onClick={onReveal}
              >
                <ExternalLinkIcon className="w-3.5 h-3.5" />
              </Button>
              <Switch
                checked={pack.enabled}
                onCheckedChange={onToggle}
                className="scale-75"
              />
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8 text-destructive hover:text-destructive hover:bg-destructive/10"
                onClick={onDelete}
              >
                <Trash2Icon className="w-3.5 h-3.5" />
              </Button>
            </div>
          </div>
          <div className="mt-3 flex flex-wrap gap-2">
            {pack.version && (
              <Badge variant="secondary" className="font-mono text-xs">
                v{pack.version}
              </Badge>
            )}
            {pack.author && (
              <Badge variant="outline" className="text-xs">
                {pack.author}
              </Badge>
            )}
            {!pack.enabled && (
              <Badge
                variant="outline"
                className="text-xs text-muted-foreground"
              >
                Disabled
              </Badge>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function ModRow({
  mod,
  onToggle,
  onDelete,
  onReveal,
}: {
  mod: InstalledMod;
  onToggle: () => void;
  onDelete: () => void;
  onReveal: () => void;
}) {
  return (
    <div className="group rounded-2xl border bg-background px-4 py-3 hover:border-primary/40 transition-colors">
      <div className="flex items-start gap-4">
        <div
          className={cn(
            "mt-0.5 flex h-12 w-12 items-center justify-center overflow-hidden rounded-2xl border bg-card",
            !mod.enabled && "opacity-50",
          )}
        >
          {mod.icon_data_url ? (
            <img
              src={mod.icon_data_url}
              alt={mod.display_name}
              className="h-full w-full object-cover"
            />
          ) : (
            <FileBoxIcon className="w-5 h-5 text-primary/70" />
          )}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <p
                className={cn(
                  "text-sm font-semibold truncate",
                  !mod.enabled &&
                    "text-muted-foreground line-through opacity-60",
                )}
              >
                {mod.display_name}
              </p>
              <p className="text-xs text-muted-foreground font-mono truncate mt-1">
                {mod.file_name}
              </p>
            </div>
            <div className="adaptive-actions flex shrink-0 items-center gap-2 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8"
                onClick={onReveal}
              >
                <ExternalLinkIcon className="w-3.5 h-3.5" />
              </Button>
              <Switch
                checked={mod.enabled}
                onCheckedChange={onToggle}
                className="scale-75"
              />
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8 text-destructive hover:text-destructive hover:bg-destructive/10"
                onClick={onDelete}
              >
                <Trash2Icon className="w-3.5 h-3.5" />
              </Button>
            </div>
          </div>
          <div className="mt-3 flex flex-wrap gap-2">
            {mod.version && (
              <Badge variant="secondary" className="font-mono text-xs">
                v{mod.version}
              </Badge>
            )}
            {mod.author && (
              <Badge variant="outline" className="text-xs">
                {mod.author}
              </Badge>
            )}
            {!mod.enabled && (
              <Badge
                variant="outline"
                className="text-xs text-muted-foreground"
              >
                Disabled
              </Badge>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function WorldRow({
  world,
  onOpen,
  onDelete,
}: {
  world: LocalWorld;
  onOpen: () => void;
  onReveal: () => void;
  onDelete: () => void;
}) {
  return (
    <div className="rounded-2xl border bg-background px-4 py-4 flex items-center gap-4">
      <div className="h-11 w-11 rounded-2xl bg-primary/10 flex items-center justify-center shrink-0 overflow-hidden">
        {world.icon_path ? (
          <img
            src={convertFileSrc(world.icon_path)}
            alt={`${world.folder_name} icon`}
            className="w-full h-full object-cover"
            onError={(e) => {
              e.currentTarget.style.display = "none";
              if (e.currentTarget.nextElementSibling) {
                e.currentTarget.nextElementSibling.classList.remove("hidden");
              }
            }}
          />
        ) : null}
        <FolderOpenIcon
          className={cn("w-5 h-5 text-primary", world.icon_path && "hidden")}
        />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-semibold truncate">{world.folder_name}</p>
        <div className="mt-1 flex flex-wrap gap-3 text-xs text-muted-foreground">
          <span className="inline-flex items-center gap-1">
            <CalendarIcon className="w-3.5 h-3.5" />
            {formatDate(world.modified_ms)}
          </span>
          <span className="inline-flex items-center gap-1">
            <HardDriveIcon className="w-3.5 h-3.5" />
            {formatBytes(world.size_bytes)}
          </span>
        </div>
      </div>
      <div className="flex items-center gap-2">
        <Button variant="outline" size="sm" className="gap-2" onClick={onOpen}>
          <FolderOpenIcon className="w-4 h-4" /> Open
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 text-destructive hover:text-destructive hover:bg-destructive/10"
          onClick={onDelete}
        >
          <Trash2Icon className="w-4 h-4" />
        </Button>
      </div>
    </div>
  );
}

function ScreenshotCard({
  screenshot,
  onOpen,
  onReveal,
  onDelete,
}: {
  screenshot: LocalScreenshot;
  onOpen: () => void;
  onReveal: () => void;
  onDelete: () => void;
}) {
  const src = convertFileSrc(screenshot.path);

  return (
    <div className="group overflow-hidden rounded-2xl border bg-background">
      <button
        type="button"
        className="aspect-video w-full flex items-center justify-center overflow-hidden bg-muted"
        onClick={onOpen}
      >
        <img
          src={src}
          alt={screenshot.file_name}
          className="h-full w-full object-contain transition-transform duration-300 group-hover:scale-[1.03]"
          loading="lazy"
        />
      </button>
      <div className="p-3 space-y-3">
        <div>
          <p className="text-sm font-semibold truncate">
            {screenshot.file_name}
          </p>
          <div className="mt-1 flex flex-wrap gap-3 text-xs text-muted-foreground">
            <span>{formatDate(screenshot.modified_ms)}</span>
            <span>{formatBytes(screenshot.size_bytes)}</span>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button size="sm" className="flex-1 gap-2" onClick={onOpen}>
            <ImageIcon className="w-4 h-4" /> Open
          </Button>
          <Button
            size="icon"
            variant="outline"
            className="h-9 w-9"
            onClick={onReveal}
          >
            <ExternalLinkIcon className="w-4 h-4" />
          </Button>
          <Button
            size="icon"
            variant="ghost"
            className="h-9 w-9 text-destructive hover:text-destructive hover:bg-destructive/10"
            onClick={onDelete}
          >
            <Trash2Icon className="w-4 h-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}

function InfoBlock({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div>
      <p className="text-[10px] uppercase font-semibold text-muted-foreground mb-1 tracking-wider">
        {label}
      </p>
      <p
        className={cn(
          "text-xs break-all rounded-xl bg-muted/70 px-3 py-2 text-muted-foreground",
          mono && "font-mono",
        )}
      >
        {value}
      </p>
    </div>
  );
}
