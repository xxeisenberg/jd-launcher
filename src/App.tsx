import { useState, useEffect, useCallback, useMemo } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { commands } from "./bindings";
import type { Profile, LauncherSettings, Group } from "./bindings";
import { listen } from "@tauri-apps/api/event";
import { save, open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";

import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarMenuSub,
  SidebarMenuSubItem,
  SidebarProvider,
  SidebarInset,
  SidebarRail,
  SidebarSeparator,
  SidebarTrigger,
} from "@/components/ui/sidebar";

import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";

import { Avatar, AvatarImage, AvatarFallback } from "@/components/ui/avatar";
import { Checkbox } from "@/components/ui/checkbox";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

import { Button } from "@/components/ui/button";

import {
  BoxesIcon,
  PackageIcon,
  SettingsIcon,
  PlusIcon,
  UploadIcon,
  ListIcon,
  LayoutGridIcon,
  ChevronRightIcon,
  ChevronDownIcon,
  LogOutIcon,
  ChevronsUpDownIcon,
  LogInIcon,
  UserIcon,
  TerminalIcon,
  StarIcon,
  MoreVerticalIcon,
  MoreHorizontalIcon,
  DownloadIcon,
  TrashIcon,
} from "lucide-react";
import * as LucideIcons from "lucide-react";

import { InstanceRow } from "./components/InstanceRow";
import { InstanceCard } from "./components/InstanceCard";
import { ProfileModal } from "./components/ProfileModal";
import { SettingsPage } from "./components/SettingsPage";
import { ConsoleDrawer } from "./components/ConsoleDrawer";
import { ModpackBrowser } from "./components/ModpackBrowser";
import { WelcomeScreen } from "./components/WelcomeScreen";
import { InstanceViewPage } from "./components/InstanceViewPage";
import { ModrinthBrowsePage } from "./components/ModrinthBrowsePage";
import { TitleBar } from "./components/TitleBar";

import "./App.css";
import { applyTheme } from "./lib/themes";
import { useKeybindListener, getKeybinds } from "./hooks/useKeybinds";

const ONBOARDING_KEY = "jd-launcher-onboarded";
const OFFLINE_USER_KEY = "jd-launcher-offline-username";

const C418_SONGS = [
  { name: "cat", url: "https://minecraft.wiki/images/Cat.ogg" },
  { name: "13", url: "https://minecraft.wiki/images/13.ogg" },
  { name: "blocks", url: "https://minecraft.wiki/images/Blocks.ogg" },
  { name: "chirp", url: "https://minecraft.wiki/images/Chirp.ogg" },
  { name: "far", url: "https://minecraft.wiki/images/Far.ogg" },
  { name: "mall", url: "https://minecraft.wiki/images/Mall.ogg" },
  { name: "mellohi", url: "https://minecraft.wiki/images/Mellohi.ogg" },
  { name: "stal", url: "https://minecraft.wiki/images/Stal.ogg" },
  { name: "strad", url: "https://minecraft.wiki/images/Strad.ogg" },
  { name: "ward", url: "https://minecraft.wiki/images/Ward.ogg" },
  { name: "11", url: "https://minecraft.wiki/images/11.ogg" },
  { name: "wait", url: "https://minecraft.wiki/images/Wait.ogg" },
];

interface DownloadProgress {
  completed: number;
  total: number;
  phase: string;
}

type ModalState =
  | { kind: "closed" }
  | { kind: "new" }
  | { kind: "edit"; profile: Profile };

type View =
  | "instances"
  | "modpacks"
  | "settings"
  | "instance-view"
  | "instance-modrinth";
type InstanceLayout = "list" | "grid";

function App() {
  const qc = useQueryClient();

  const [onboarded, setOnboarded] = useState(
    () => localStorage.getItem(ONBOARDING_KEY) === "true",
  );
  const [username, setUsername] = useState(
    () => localStorage.getItem(OFFLINE_USER_KEY) || "",
  );
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [launching, setLaunching] = useState<string | null>(null);
  const [modal, setModal] = useState<ModalState>({ kind: "closed" });
  const [deleteConfirm, setDeleteConfirm] = useState<Profile | null>(null);
  const [deleteFolder, setDeleteFolder] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [currentView, setCurrentView] = useState<View>("instances");
  const [activeInstance, setActiveInstance] = useState<Profile | null>(null);
  const [instanceLayout, setInstanceLayout] = useState<InstanceLayout>("list");
  const [showConsole, setShowConsole] = useState(false);
  const [modrinthProjectType, setModrinthProjectType] = useState<
    "mod" | "shader" | "resourcepack"
  >("mod");
  const [deviceCodeInfo, setDeviceCodeInfo] = useState<any | null>(null);
  const [sidebarGroupFilter, setSidebarGroupFilter] = useState<string | null>(
    null,
  );
  const [sidebarGroupsOpen, setSidebarGroupsOpen] = useState(true);
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(() => {
    try {
      const stored = localStorage.getItem("jd-collapsed-groups");
      return stored ? new Set(JSON.parse(stored)) : new Set();
    } catch {
      return new Set();
    }
  });
  const [dragProfileId, setDragProfileId] = useState<string | null>(null);
  const [deleteGroupConfirm, setDeleteGroupConfirm] = useState<string | null>(
    null,
  );
  const [deleteGroupFolders, setDeleteGroupFolders] = useState(false);

  // Keybinds
  const [keybinds, setKeybinds] = useState(getKeybinds);
  useEffect(() => {
    const refresh = () => setKeybinds(getKeybinds());
    window.addEventListener("keybinds-changed", refresh);
    return () => window.removeEventListener("keybinds-changed", refresh);
  }, []);

  useKeybindListener(keybinds, {
    "navigate-instances": () => { setSidebarGroupFilter(null); setCurrentView("instances"); },
    "navigate-modpacks": () => setCurrentView("modpacks"),
    "navigate-settings": () => setCurrentView("settings"),
    "toggle-console": () => setShowConsole((v) => !v),
    "new-instance": () => setModal({ kind: "new" }),
    "import-instance": () => handleImport(),
  });

  const [logoClicks, setLogoClicks] = useState(0);
  const [showJukebox, setShowJukebox] = useState(false);
  const [playingSong, setPlayingSong] = useState(C418_SONGS[0]);

  useEffect(() => {
    if (logoClicks > 0 && logoClicks < 10) {
      const timer = setTimeout(() => setLogoClicks(0), 1000);
      return () => clearTimeout(timer);
    }
  }, [logoClicks]);

  // queries
  const { data: profiles = [] } = useQuery({
    queryKey: ["profiles"],
    queryFn: () => commands.listProfiles(),
  });

  const { data: lastProfileId = null } = useQuery({
    queryKey: ["lastProfileId"],
    queryFn: async () => (await commands.getLastProfileId()) ?? null,
  });

  const { data: settings = null } = useQuery<LauncherSettings | null>({
    queryKey: ["settings"],
    queryFn: () => commands.getSettings(),
  });

  const { data: authMode = false } = useQuery({
    queryKey: ["authMode"],
    queryFn: () => commands.getAuthMode(),
  });

  const { data: activeAccount = null } = useQuery({
    queryKey: ["activeAccount"],
    queryFn: async () => (await commands.getActiveAccount()) ?? null,
    enabled: authMode,
  });

  const customGroups: Group[] = settings?.groups ?? [];

  // Filter profiles by sidebar selection
  const filteredProfiles = useMemo(() => {
    if (!sidebarGroupFilter) return profiles;
    if (sidebarGroupFilter === "__favorites__")
      return profiles.filter((p) => p.favorite);
    return profiles.filter(
      (p) => (p.group?.trim() || "Ungrouped") === sidebarGroupFilter,
    );
  }, [profiles, sidebarGroupFilter]);

  const groupedProfiles = useMemo(() => {
    return Array.from(
      filteredProfiles.reduce((groups, profile) => {
        const id = profile.group?.trim() || "Ungrouped";
        const current = groups.get(id) ?? [];
        current.push(profile);
        groups.set(id, current);
        return groups;
      }, new Map<string, Profile[]>()),
    )
      .sort(([left], [right]) => {
        if (left === "Ungrouped") return 1;
        if (right === "Ungrouped") return -1;
        const li = customGroups.findIndex(
          (g) => g.id === left || g.name === left,
        );
        const ri = customGroups.findIndex(
          (g) => g.id === right || g.name === right,
        );
        if (li >= 0 && ri >= 0) return li - ri;
        if (li >= 0) return -1;
        if (ri >= 0) return 1;
        return left.localeCompare(right);
      })
      .map(([id, items]) => {
        const custom = customGroups.find((g) => g.id === id || g.name === id);
        return { id, name: custom ? custom.name : id, items };
      });
  }, [filteredProfiles, customGroups]);

  const groupMeta = useCallback(
    (id: string) => customGroups.find((g) => g.id === id || g.name === id),
    [customGroups],
  );

  const toggleGroupCollapse = useCallback((label: string) => {
    setCollapsedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(label)) next.delete(label);
      else next.add(label);
      localStorage.setItem("jd-collapsed-groups", JSON.stringify([...next]));
      return next;
    });
  }, []);

  const handleDragStart = useCallback(
    (e: React.DragEvent, profileId: string) => {
      setDragProfileId(profileId);
      if (e.dataTransfer) {
        e.dataTransfer.setData("text/plain", profileId);
        e.dataTransfer.effectAllowed = "move";
      }
    },
    [],
  );

  const invalidateProfiles = useCallback(
    () => qc.invalidateQueries({ queryKey: ["profiles"] }),
    [qc],
  );

  const handleDrop = useCallback(
    async (e: React.DragEvent, targetGroup: string) => {
      e.preventDefault();
      const profileId = e.dataTransfer.getData("text/plain") || dragProfileId;
      if (!profileId) return;
      const profile = profiles.find((p) => p.id === profileId);
      if (!profile) return;

      let groupValue: string | null = targetGroup;
      if (targetGroup === "Ungrouped") {
        groupValue = null;
      }

      // Don't update if already in the group
      const currentGroup = profile.group?.trim() || "Ungrouped";
      if (currentGroup === targetGroup) return;

      await commands.saveProfile({ ...profile, group: groupValue });
      invalidateProfiles();
      setDragProfileId(null);
    },
    [dragProfileId, profiles, invalidateProfiles],
  );

  async function handleToggleFavorite(profile: Profile) {
    await commands.saveProfile({ ...profile, favorite: !profile.favorite });
    invalidateProfiles();
  }

  async function handleExportGroup(groupName: string) {
    const dest = await open({ directory: true, multiple: false });
    if (!dest || typeof dest !== "string") return;
    const result = await commands.exportGroup(groupName, dest);
    if (result.status === "error") setError(`Export failed: ${result.error}`);
  }

  async function handleDeleteGroup() {
    if (!deleteGroupConfirm) return;
    const result = await commands.deleteGroup(
      deleteGroupConfirm,
      deleteGroupFolders,
    );
    if (result.status === "error") setError(`Delete failed: ${result.error}`);
    setDeleteGroupConfirm(null);
    setDeleteGroupFolders(false);
    invalidateProfiles();
    qc.invalidateQueries({ queryKey: ["settings"] });
  }

  const isDownloading = progress !== null;

  const handleOnboardingComplete = useCallback(
    async (mode: "online" | "offline", offlineUser?: string) => {
      localStorage.setItem(ONBOARDING_KEY, "true");
      if (mode === "offline" && offlineUser) {
        localStorage.setItem(OFFLINE_USER_KEY, offlineUser);
        setUsername(offlineUser);
      }

      // Persist auth mode to backend
      if (settings) {
        await commands.updateSettings({
          ...settings,
          online_mode: mode === "online",
        });
        qc.invalidateQueries({ queryKey: ["settings"] });
      }

      setOnboarded(true);
      qc.invalidateQueries({ queryKey: ["authMode"] });
      qc.invalidateQueries({ queryKey: ["activeAccount"] });
    },
    [qc, settings],
  );

  useEffect(() => {
    const unlistenDownload = listen<DownloadProgress>(
      "download-progress",
      (event) => {
        if (event.payload.phase === "done") {
          setProgress(null);
          setLaunching(null);
        } else {
          setProgress(event.payload);
        }
      },
    );

    const unlistenJava = listen<DownloadProgress>(
      "java-download-progress",
      (event) => {
        if (event.payload.phase === "done") {
          setProgress(null);
        } else {
          setProgress(event.payload);
        }
      },
    );

    return () => {
      unlistenDownload.then((fn) => fn());
      unlistenJava.then((fn) => fn());
    };
  }, []);

  // appearance
  useEffect(() => {
    if (settings) {
      applyTheme(
        settings.theme === "dark",
        settings.accent_color || "",
        settings.font_family || "",
        settings.ui_style || "",
        settings.ui_scale ?? 100,
      );
    }
  }, [
    settings?.theme,
    settings?.accent_color,
    settings?.font_family,
    settings?.ui_style,
    settings?.ui_scale,
  ]);

  // title bar decorations
  useEffect(() => {
    if (settings) {
      const w = getCurrentWindow();
      w.setDecorations(!!settings.use_native_titlebar);
    }
  }, [settings?.use_native_titlebar]);

  useEffect(() => {
    if (!activeInstance) return;

    const nextProfile = profiles.find(
      (profile) => profile.id === activeInstance.id,
    );
    if (!nextProfile) {
      setActiveInstance(null);
      if (
        currentView === "instance-view" ||
        currentView === "instance-modrinth"
      ) {
        setCurrentView("instances");
      }
      return;
    }

    if (nextProfile !== activeInstance) {
      setActiveInstance(nextProfile);
    }
  }, [activeInstance, currentView, profiles]);

  async function handleLaunch(profile: Profile) {
    if (!authMode && !username.trim()) {
      setError("Enter a username first.");
      return;
    }
    if (authMode && !activeAccount) {
      setError("Sign in with Microsoft to play.");
      return;
    }
    setError(null);
    setLaunching(profile.id);
    const nameArg = authMode ? "" : username;
    try {
      setShowConsole(true);
      const result = await commands.downloadVersionAndRun(profile.id, nameArg);
      if (result.status === "error") {
        setError(`Launch failed: ${result.error}`);
        setLaunching(null);
      } else {
        if (settings?.close_on_launch) await getCurrentWindow().close();
      }
    } catch (e: any) {
      setError(`Launch failed: ${e}`);
      setLaunching(null);
    }
  }

  async function handleLogin() {
    try {
      setError(null);
      const info = await commands.startMsLogin();
      if (info && "data" in info && info.status === "ok")
        setDeviceCodeInfo(info.data);
      else setDeviceCodeInfo(info);
      const account = await commands.pollMsLogin();
      if (account && "data" in account && account.status === "ok") {
        if (settings) {
          await commands.updateSettings({ ...settings, online_mode: true });
          qc.invalidateQueries({ queryKey: ["settings"] });
          qc.invalidateQueries({ queryKey: ["authMode"] });
        }
        qc.invalidateQueries({ queryKey: ["activeAccount"] });
      } else if (account && account.status === "error") {
        setError(account.error as string);
      } else {
        if (settings) {
          await commands.updateSettings({ ...settings, online_mode: true });
          qc.invalidateQueries({ queryKey: ["settings"] });
          qc.invalidateQueries({ queryKey: ["authMode"] });
        }
        qc.invalidateQueries({ queryKey: ["activeAccount"] });
      }
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setDeviceCodeInfo(null);
    }
  }

  async function handleLogout() {
    if (!activeAccount) return;
    try {
      await commands.logoutAccount(activeAccount.uuid);
      if (settings) {
        await commands.updateSettings({ ...settings, online_mode: false });
        qc.invalidateQueries({ queryKey: ["settings"] });
        qc.invalidateQueries({ queryKey: ["authMode"] });
      }
      qc.invalidateQueries({ queryKey: ["activeAccount"] });
    } catch (e) {
      console.error("Logout failed", e);
    }
  }

  async function handleSaveProfile(profile: Profile) {
    await commands.saveProfile(profile);
    setModal({ kind: "closed" });
    invalidateProfiles();
  }

  async function handleDelete(profile: Profile) {
    await commands.deleteProfile(profile.id, deleteFolder);
    setDeleteConfirm(null);
    setDeleteFolder(false);
    invalidateProfiles();
  }

  async function handleDuplicate(profile: Profile) {
    const result = await commands.duplicateProfile(profile.id);
    if (result.status === "ok") invalidateProfiles();
  }

  async function handleExport(profile: Profile) {
    const dest = await save({
      defaultPath: `${profile.name.replace(/\s+/g, "_")}.zip`,
      filters: [{ name: "Profile Archive", extensions: ["zip"] }],
    });
    if (!dest) return;
    const result = await commands.exportProfile(profile.id, dest);
    if (result.status === "error") setError(`Export failed: ${result.error}`);
  }

  async function handleImport() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Profile Archive", extensions: ["zip"] }],
    });
    if (!selected) return;
    const zipPath = typeof selected === "string" ? selected : selected[0];
    const result = await commands.importProfile(zipPath);
    if (result.status === "ok") invalidateProfiles();
    else setError(`Import failed: ${result.error}`);
  }

  const phaseLabel = (phase: string) => {
    switch (phase) {
      case "java":
        return "Downloading Java Environment...";
      case "java-verifying":
        return "Verifying Java Environment...";
      case "java-extracting":
        return "Extracting Java Environment...";
      case "assets":
        return "Downloading Assets...";
      case "libraries":
        return "Downloading Libraries...";
      case "client":
        return "Downloading Minecraft Client...";
      case "modloader":
        return "Downloading Modloader...";
      case "verifying-libraries":
        return "Verifying libraries…";
      case "verifying-assets":
        return "Verifying assets…";
      default:
        return "Downloading...";
    }
  };

  const VIEW_LABELS: Record<View, string> = {
    instances: "Instances",
    modpacks: "Modpacks",
    settings: "Settings",
    "instance-view": "Instance Details",
    "instance-modrinth": "Browse Modrinth",
  };

  if (!onboarded) {
    return <WelcomeScreen onComplete={handleOnboardingComplete} />;
  }

  return (
    <SidebarProvider>
      <Sidebar collapsible="icon">
        <SidebarHeader>
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton
                size="lg"
                tooltip="JD Launcher"
                className="cursor-pointer"
                onClick={() => {
                  const currentClicks = logoClicks + 1;
                  if (currentClicks === 10) {
                    const randomSong = C418_SONGS[Math.floor(Math.random() * C418_SONGS.length)];
                    setPlayingSong(randomSong);
                    setShowJukebox(true);
                    setLogoClicks(0);
                    try {
                      const audio = new Audio(randomSong.url);
                      audio.volume = 0.5;
                      audio.play().catch((e) => console.error("Playing audio failed", e));
                    } catch (e) {
                      // ignore
                    }
                    setTimeout(() => setShowJukebox(false), 8000);
                  } else {
                    setLogoClicks(currentClicks);
                  }
                }}
              >
                <div className="flex aspect-square size-8 items-center justify-center">
                  <img
                    src="/logo.svg"
                    alt="Logo"
                    className="w-8 h-8 drop-shadow-sm"
                  />
                </div>
                <div className="grid flex-1 text-left text-sm leading-tight group-data-[collapsible=icon]:hidden">
                  <span className="truncate font-bold">JD Launcher</span>
                  <span className="truncate text-xs text-muted-foreground">
                    Minecraft
                  </span>
                </div>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarHeader>

        <SidebarContent>
          <SidebarGroup>
            <SidebarGroupLabel>Navigate</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                <Collapsible
                  open={sidebarGroupsOpen}
                  onOpenChange={setSidebarGroupsOpen}
                >
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      tooltip="Instances"
                      isActive={
                        currentView === "instances" && !sidebarGroupFilter
                      }
                      onClick={() => {
                        setSidebarGroupFilter(null);
                        setCurrentView("instances");
                      }}
                    >
                      <BoxesIcon />
                      <span>Instances</span>
                    </SidebarMenuButton>
                    <CollapsibleTrigger asChild>
                      <button className="absolute right-1 top-1/2 -translate-y-1/2 p-1 rounded-md hover:bg-accent text-muted-foreground group-data-[collapsible=icon]:hidden">
                        <ChevronRightIcon
                          className={`w-3.5 h-3.5 transition-transform ${sidebarGroupsOpen ? "rotate-90" : ""}`}
                        />
                      </button>
                    </CollapsibleTrigger>
                  </SidebarMenuItem>

                  <CollapsibleContent>
                    <SidebarMenuSub>
                      <SidebarMenuSubItem>
                        <SidebarMenuButton
                          size="sm"
                          isActive={sidebarGroupFilter === "__favorites__"}
                          onClick={() => {
                            setSidebarGroupFilter("__favorites__");
                            setCurrentView("instances");
                          }}
                        >
                          <StarIcon className="w-3.5 h-3.5" />
                          <span>Favorites</span>
                        </SidebarMenuButton>
                      </SidebarMenuSubItem>
                      {customGroups.map((g) => {
                        const IconComponent =
                          (LucideIcons as any)[g.icon] ||
                          LucideIcons.FolderIcon;
                        return (
                          <SidebarMenuSubItem key={g.id}>
                            <SidebarMenuButton
                              size="sm"
                              isActive={sidebarGroupFilter === g.id}
                              onClick={() => {
                                setSidebarGroupFilter(g.id);
                                setCurrentView("instances");
                              }}
                            >
                              <IconComponent
                                className="w-3.5 h-3.5 shrink-0"
                                style={{ color: g.color }}
                              />
                              <span className="truncate">{g.name}</span>
                            </SidebarMenuButton>
                          </SidebarMenuSubItem>
                        );
                      })}
                    </SidebarMenuSub>
                  </CollapsibleContent>
                </Collapsible>

                <SidebarMenuItem>
                  <SidebarMenuButton
                    tooltip="Modpacks"
                    isActive={currentView === "modpacks"}
                    onClick={() => setCurrentView("modpacks")}
                  >
                    <PackageIcon />
                    <span>Modpacks</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
                <SidebarMenuItem>
                  <SidebarMenuButton
                    tooltip="Console"
                    onClick={() => setShowConsole((v) => !v)}
                  >
                    <TerminalIcon />
                    <span>Console</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        </SidebarContent>

        <SidebarSeparator />

        <SidebarFooter>
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton
                tooltip="Settings"
                isActive={currentView === "settings"}
                onClick={() => setCurrentView("settings")}
              >
                <SettingsIcon />
                <span>Settings</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>

          {/* account */}
          <SidebarMenu>
            <SidebarMenuItem>
              {authMode ? (
                activeAccount ? (
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <SidebarMenuButton
                        size="lg"
                        tooltip={activeAccount.username}
                        className="data-[state=open]:bg-sidebar-accent"
                      >
                        <Avatar className="h-8 w-8 rounded-lg">
                          <AvatarImage
                            src={`https://mc-heads.net/avatar/${activeAccount.uuid}/32`}
                            alt={activeAccount.username}
                            style={{ imageRendering: "pixelated" }}
                          />
                          <AvatarFallback className="rounded-lg text-xs">
                            {activeAccount.username.slice(0, 2).toUpperCase()}
                          </AvatarFallback>
                        </Avatar>
                        <div className="grid flex-1 text-left text-sm leading-tight">
                          <span className="truncate font-medium">
                            {activeAccount.username}
                          </span>
                          <span className="truncate text-xs text-muted-foreground">
                            Microsoft
                          </span>
                        </div>
                        <ChevronsUpDownIcon className="ml-auto size-4" />
                      </SidebarMenuButton>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent
                      className="w-(--radix-dropdown-menu-trigger-width) min-w-56 rounded-lg"
                      side="right"
                      align="end"
                      sideOffset={4}
                    >
                      <DropdownMenuItem onClick={handleLogout}>
                        <LogOutIcon className="mr-2" /> Sign out
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                ) : (
                  <SidebarMenuButton
                    size="lg"
                    tooltip="Sign In"
                    onClick={handleLogin}
                  >
                    <div className="flex items-center justify-center w-8 h-8 rounded-lg bg-muted">
                      <LogInIcon className="w-4 h-4" />
                    </div>
                    <div className="grid flex-1 text-left text-sm leading-tight">
                      <span className="truncate font-medium">Sign In</span>
                      <span className="truncate text-xs text-muted-foreground">
                        Microsoft
                      </span>
                    </div>
                  </SidebarMenuButton>
                )
              ) : (
                <SidebarMenuButton
                  size="lg"
                  tooltip={username || "Offline"}
                  className="cursor-default"
                >
                  <div className="flex items-center justify-center w-8 h-8 rounded-lg bg-muted">
                    <UserIcon className="w-4 h-4" />
                  </div>
                  <div className="grid flex-1 text-left leading-tight group-data-[collapsible=icon]:hidden">
                    <input
                      type="text"
                      value={username}
                      onChange={(e) => {
                        setUsername(e.target.value);
                        localStorage.setItem(OFFLINE_USER_KEY, e.target.value);
                      }}
                      placeholder="Username"
                      maxLength={16}
                      className="bg-transparent text-sm font-medium border-none outline-none placeholder:text-muted-foreground w-full"
                    />
                    <div className="flex items-center justify-between">
                      <span className="text-xs text-muted-foreground">
                        Offline
                      </span>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          handleLogin();
                        }}
                        className="text-[10px] font-bold uppercase tracking-wider text-primary hover:text-primary/80 transition-colors"
                      >
                        Sign In
                      </button>
                    </div>
                  </div>
                </SidebarMenuButton>
              )}
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarFooter>

        <SidebarRail />
      </Sidebar>

      <SidebarInset className="flex min-w-0 flex-col overflow-hidden">
        {/* breadcrumb bar */}
        <div
          className="flex min-w-0 items-center gap-1.5 px-3 h-12 border-b border-border shrink-0 min-[1024px]:gap-2 min-[1024px]:px-6"
          data-tauri-drag-region
        >
          <SidebarTrigger className="-ml-1 h-7 w-7 shrink-0 min-[1024px]:hidden" />
          <div className="h-4 w-px shrink-0 bg-border min-[1024px]:hidden" />
          <span className="hidden text-xs text-muted-foreground min-[1024px]:inline">Home</span>
          <ChevronRightIcon className="hidden w-3 h-3 shrink-0 text-muted-foreground/50 min-[1024px]:block" />
          {(currentView === "instance-view" ||
            currentView === "instance-modrinth") &&
          activeInstance ? (
            <>
              <button
                className="text-xs text-muted-foreground hover:text-foreground transition-colors"
                onClick={() => setCurrentView("instances")}
              >
                Instances
              </button>
              <ChevronRightIcon className="w-3 h-3 text-muted-foreground/50" />
              <button
                className={
                  currentView === "instance-modrinth"
                    ? "text-xs text-muted-foreground hover:text-foreground transition-colors truncate max-w-[150px]"
                    : "text-xs font-medium truncate max-w-[200px]"
                }
                onClick={() => setCurrentView("instance-view")}
              >
                {activeInstance.name}
              </button>
              {currentView === "instance-modrinth" && (
                <>
                  <ChevronRightIcon className="w-3 h-3 text-muted-foreground/50" />
                  <span className="text-xs font-medium">
                    Add{" "}
                    {modrinthProjectType === "mod"
                      ? "Mod"
                      : modrinthProjectType === "shader"
                        ? "Shader"
                        : "Resource Pack"}
                  </span>
                </>
              )}
            </>
          ) : (
            <span className="text-xs font-medium">
              {VIEW_LABELS[currentView]}
            </span>
          )}
          <div className="flex-1" />

          {/* instances toolbar */}
          {currentView === "instances" && (
            <div className="flex shrink-0 items-center gap-1.5">
              <Button
                variant={instanceLayout === "list" ? "secondary" : "ghost"}
                size="icon"
                className="h-7 w-7"
                onClick={() => setInstanceLayout("list")}
              >
                <ListIcon className="w-3.5 h-3.5" />
              </Button>
              <Button
                variant={instanceLayout === "grid" ? "secondary" : "ghost"}
                size="icon"
                className="h-7 w-7"
                onClick={() => setInstanceLayout("grid")}
              >
                <LayoutGridIcon className="w-3.5 h-3.5" />
              </Button>
              <div className="w-px h-4 bg-border mx-1" />
              <Button
                variant="ghost"
                size="sm"
                className="h-7 text-xs gap-1"
                onClick={handleImport}
              >
                <UploadIcon className="w-3 h-3" /> Import
              </Button>
              <Button
                size="sm"
                className="h-7 text-xs gap-1"
                onClick={() => setModal({ kind: "new" })}
              >
                <PlusIcon className="w-3 h-3" /> New
              </Button>
            </div>
          )}
          {!settings?.use_native_titlebar && <TitleBar />}
        </div>

        {/* content */}
        <div className="flex-1 overflow-hidden flex flex-col relative">
          {/* error banner */}
          {error && (
            <div className="mx-3 mt-3 flex items-center gap-2 px-3 py-2 rounded-md bg-destructive/10 text-destructive text-sm min-[1024px]:mx-6">
              ⚠ {error}
              <button
                onClick={() => setError(null)}
                className="ml-auto text-destructive/60 hover:text-destructive"
              >
                ✕
              </button>
            </div>
          )}

          {/* instances view */}
          {currentView === "instances" && (
            <div className="flex-1 overflow-y-auto">
              {profiles.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full gap-3 text-muted-foreground">
                  <BoxesIcon className="w-12 h-12 opacity-20" />
                  <p className="text-sm font-medium">No instances yet</p>
                  <p className="text-xs">
                    Create your first Minecraft installation
                  </p>
                  <Button
                    size="sm"
                    onClick={() => setModal({ kind: "new" })}
                    className="mt-2"
                  >
                    <PlusIcon className="w-3 h-3 mr-1" /> Create
                  </Button>
                </div>
              ) : instanceLayout === "list" ? (
                <div className="flex flex-col px-3 py-3 gap-4 min-[1024px]:px-4">
                  {groupedProfiles.map((group) => {
                    const meta = groupMeta(group.id);
                    const isCollapsed = collapsedGroups.has(group.id);
                    return (
                      <section
                        key={group.id}
                        className="rounded-xl border border-border/70 bg-card/30 overflow-hidden"
                        onDragOver={(e) => {
                          e.preventDefault();
                          e.dataTransfer.dropEffect = "move";
                        }}
                        onDrop={(e) => handleDrop(e, group.id)}
                      >
                        <div
                          className="flex items-center justify-between px-4 py-3 border-b border-border/60 cursor-pointer select-none"
                          onClick={() => toggleGroupCollapse(group.id)}
                          style={
                            meta
                              ? { borderLeft: `1px solid ${meta.color}` }
                              : undefined
                          }
                        >
                          <div className="flex items-center gap-2.5">
                            {isCollapsed ? (
                              <ChevronRightIcon className="w-4 h-4 text-muted-foreground shrink-0" />
                            ) : (
                              <ChevronDownIcon className="w-4 h-4 text-muted-foreground shrink-0" />
                            )}
                            {meta &&
                              (() => {
                                const IconComponent =
                                  (LucideIcons as any)[meta.icon] ||
                                  LucideIcons.FolderIcon;
                                return (
                                  <IconComponent
                                    className="w-[20px] h-[20px] shrink-0"
                                    style={{ color: meta.color }}
                                  />
                                );
                              })()}
                            <div>
                              <p className="text-xs uppercase tracking-[0.22em] text-muted-foreground">
                                Group
                              </p>
                              <h2 className="text-sm font-semibold">
                                {group.name}
                              </h2>
                            </div>
                          </div>
                          <div className="flex items-center gap-2">
                            <span className="text-xs text-muted-foreground">
                              {group.items.length} instance
                              {group.items.length === 1 ? "" : "s"}
                            </span>
                            {group.id !== "Ungrouped" && (
                              <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                  <button
                                    onClick={(e) => e.stopPropagation()}
                                    className="p-1 rounded-md hover:bg-accent text-muted-foreground"
                                  >
                                    <MoreVerticalIcon className="w-4 h-4" />
                                  </button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end">
                                  <DropdownMenuItem
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      handleExportGroup(group.id);
                                    }}
                                  >
                                    <DownloadIcon className="w-4 h-4 mr-2" />{" "}
                                    Export Group
                                  </DropdownMenuItem>
                                  <DropdownMenuItem
                                    className="text-destructive focus:text-destructive"
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      setDeleteGroupConfirm(group.id);
                                    }}
                                  >
                                    <TrashIcon className="w-4 h-4 mr-2" />{" "}
                                    Delete Group
                                  </DropdownMenuItem>
                                </DropdownMenuContent>
                              </DropdownMenu>
                            )}
                          </div>
                        </div>
                        {!isCollapsed && (
                          <div className="flex flex-col px-2 py-2 gap-0.5">
                            {group.items.map((profile) => (
                              <InstanceRow
                                key={profile.id}
                                profile={profile}
                                isLastUsed={profile.id === lastProfileId}
                                onLaunch={handleLaunch}
                                onEdit={(p) =>
                                  setModal({ kind: "edit", profile: p })
                                }
                                onDuplicate={handleDuplicate}
                                onDelete={(p) => {
                                  setDeleteConfirm(p);
                                  setDeleteFolder(false);
                                }}
                                onExport={handleExport}
                                onView={(p) => {
                                  setActiveInstance(p);
                                  setCurrentView("instance-view");
                                }}
                                onFavorite={handleToggleFavorite}
                                onDragStart={handleDragStart}
                              />
                            ))}
                          </div>
                        )}
                      </section>
                    );
                  })}
                </div>
              ) : (
                <div className="p-4 space-y-5 min-[1024px]:p-6 min-[1024px]:space-y-6">
                  {groupedProfiles.map((group) => {
                    const meta = groupMeta(group.id);
                    const isCollapsed = collapsedGroups.has(group.id);
                    return (
                      <section
                        key={group.id}
                        className="space-y-3"
                        onDragOver={(e) => {
                          e.preventDefault();
                          e.dataTransfer.dropEffect = "move";
                        }}
                        onDrop={(e) => handleDrop(e, group.id)}
                      >
                        <div
                          className="flex items-end justify-between gap-3 cursor-pointer select-none"
                          onClick={() => toggleGroupCollapse(group.id)}
                          style={
                            meta
                              ? {
                                  borderLeft: `1px solid ${meta.color}`,
                                  paddingLeft: 12,
                                }
                              : undefined
                          }
                        >
                          <div className="flex items-center gap-2.5">
                            {isCollapsed ? (
                              <ChevronRightIcon className="w-4 h-4 text-muted-foreground shrink-0" />
                            ) : (
                              <ChevronDownIcon className="w-4 h-4 text-muted-foreground shrink-0" />
                            )}
                            {meta &&
                              (() => {
                                const IconComponent =
                                  (LucideIcons as any)[meta.icon] ||
                                  LucideIcons.FolderIcon;
                                return (
                                  <IconComponent
                                    className="w-[18px] h-[18px] shrink-0"
                                    style={{ color: meta.color }}
                                  />
                                );
                              })()}
                            <div>
                              <p className="text-xs uppercase tracking-[0.24em] text-muted-foreground">
                                Group
                              </p>
                              <h2 className="text-lg font-semibold tracking-tight">
                                {group.name}
                              </h2>
                            </div>
                          </div>
                          <div className="flex items-center gap-2 opacity-80 mb-1">
                            <span className="text-xs text-muted-foreground">
                              {group.items.length} instance
                              {group.items.length === 1 ? "" : "s"}
                            </span>
                            {group.id !== "Ungrouped" && (
                              <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                  <button
                                    onClick={(e) => e.stopPropagation()}
                                    className="p-1 rounded-md hover:bg-accent text-muted-foreground"
                                  >
                                    <MoreHorizontalIcon className="w-4 h-4" />
                                  </button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end">
                                  <DropdownMenuItem
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      handleExportGroup(group.id);
                                    }}
                                  >
                                    <DownloadIcon className="w-4 h-4 mr-2" />{" "}
                                    Export Group
                                  </DropdownMenuItem>
                                  <DropdownMenuItem
                                    className="text-destructive focus:text-destructive"
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      setDeleteGroupConfirm(group.id);
                                    }}
                                  >
                                    <TrashIcon className="w-4 h-4 mr-2" />{" "}
                                    Delete Group
                                  </DropdownMenuItem>
                                </DropdownMenuContent>
                              </DropdownMenu>
                            )}
                          </div>
                        </div>
                        {!isCollapsed && (
                          <div className="grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-3">
                            {group.items.map((profile) => (
                              <InstanceCard
                                key={profile.id}
                                profile={profile}
                                isLastUsed={profile.id === lastProfileId}
                                onLaunch={handleLaunch}
                                onEdit={(p) =>
                                  setModal({ kind: "edit", profile: p })
                                }
                                onDuplicate={handleDuplicate}
                                onDelete={(p) => {
                                  setDeleteConfirm(p);
                                  setDeleteFolder(false);
                                }}
                                onExport={handleExport}
                                onView={(p) => {
                                  setActiveInstance(p);
                                  setCurrentView("instance-view");
                                }}
                                onFavorite={handleToggleFavorite}
                                onDragStart={handleDragStart}
                              />
                            ))}
                          </div>
                        )}
                      </section>
                    );
                  })}
                </div>
              )}
            </div>
          )}

          {/* modpacks view */}
          {currentView === "modpacks" && (
            <ModpackBrowser onInstalled={invalidateProfiles} />
          )}

          {/* settings view */}
          {currentView === "settings" && (
            <SettingsPage
              onSettingsSaved={() => {
                qc.invalidateQueries({ queryKey: ["settings"] });
                qc.invalidateQueries({ queryKey: ["authMode"] });
                qc.invalidateQueries({ queryKey: ["activeAccount"] });
              }}
            />
          )}

          {/* instance view */}
          {currentView === "instance-view" && activeInstance && (
            <InstanceViewPage
              profile={activeInstance}
              onBack={() => setCurrentView("instances")}
              onLaunch={handleLaunch}
              onEdit={(p) => setModal({ kind: "edit", profile: p })}
              onBrowseModrinth={(type) => {
                setModrinthProjectType(type);
                setCurrentView("instance-modrinth");
              }}
            />
          )}

          {/* instance modrinth browse view */}
          {currentView === "instance-modrinth" && activeInstance && (
            <ModrinthBrowsePage
              profileId={activeInstance.id}
              gameDir={activeInstance.game_dir}
              gameVersion={activeInstance.version}
              modloader={activeInstance.modloader}
              projectType={modrinthProjectType}
              onBack={() => setCurrentView("instance-view")}
            />
          )}

          {/* console drawer */}
          <ConsoleDrawer
            visible={showConsole}
            onClose={() => setShowConsole(false)}
          />
        </div>

        {/* progress overlay */}
        {(isDownloading || launching) && (
          <div className="fixed inset-0 z-50 bg-black/60 flex items-center justify-center">
            <div className="bg-card border border-border rounded-xl p-8 flex flex-col items-center gap-4 min-w-[300px] shadow-xl">
              <div className="spinner w-8! h-8!" />
              {isDownloading && progress ? (
                <>
                  <p className="text-sm font-medium">
                    {phaseLabel(progress.phase)}
                  </p>
                  <div className="w-full h-1.5 bg-muted rounded-full overflow-hidden">
                    <div
                      className="h-full bg-primary rounded-full transition-all"
                      style={{
                        width: `${progress.total > 0 ? (progress.completed / progress.total) * 100 : 0}%`,
                      }}
                    />
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {progress.completed} / {progress.total}
                  </p>
                </>
              ) : (
                <p className="text-sm font-medium">Starting game…</p>
              )}
            </div>
          </div>
        )}
      </SidebarInset>

      {/* profile modal */}
      {modal.kind !== "closed" && (
        <ProfileModal
          initial={modal.kind === "edit" ? modal.profile : null}
          onSave={handleSaveProfile}
          onClose={() => setModal({ kind: "closed" })}
        />
      )}

      {deleteConfirm && (
        <div
          className="fixed inset-0 z-50 bg-black/60 flex items-center justify-center"
          onClick={() => {
            setDeleteConfirm(null);
            setDeleteFolder(false);
          }}
        >
          <div
            className="bg-card border border-border rounded-xl p-6 w-[360px] space-y-4 shadow-xl"
            onClick={(e) => e.stopPropagation()}
          >
            <h2 className="text-base font-semibold">Delete Profile?</h2>
            <p className="text-sm text-muted-foreground mb-4">
              Delete <strong>"{deleteConfirm.name}"</strong>?
            </p>
            <div className="flex items-center space-x-2">
              <Checkbox
                id="delete-folder"
                checked={deleteFolder}
                onCheckedChange={(c) => setDeleteFolder(c === true)}
              />
              <label
                htmlFor="delete-folder"
                className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
              >
                Delete instance folder
              </label>
            </div>
            <div className="flex gap-2 justify-end mt-4">
              <Button
                variant="outline"
                onClick={() => {
                  setDeleteConfirm(null);
                  setDeleteFolder(false);
                }}
              >
                Cancel
              </Button>
              <Button
                variant="destructive"
                onClick={() => handleDelete(deleteConfirm)}
              >
                Delete
              </Button>
            </div>
          </div>
        </div>
      )}

      {deleteGroupConfirm && (
        <div
          className="fixed inset-0 z-50 bg-black/60 flex items-center justify-center"
          onClick={() => {
            setDeleteGroupConfirm(null);
            setDeleteGroupFolders(false);
          }}
        >
          <div
            className="bg-card border border-border rounded-xl p-6 w-[360px] space-y-4 shadow-xl"
            onClick={(e) => e.stopPropagation()}
          >
            <h2 className="text-base font-semibold">Delete Group?</h2>
            <p className="text-sm text-muted-foreground mb-4">
              Delete the group <strong>"{deleteGroupConfirm}"</strong>? This
              will delete all instances in the group.
            </p>
            <div className="flex items-center space-x-2">
              <Checkbox
                id="delete-group-folders"
                checked={deleteGroupFolders}
                onCheckedChange={(c) => setDeleteGroupFolders(c === true)}
              />
              <label
                htmlFor="delete-group-folders"
                className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
              >
                Delete instance folders for all profiles in the group
              </label>
            </div>
            <div className="flex gap-2 justify-end mt-4">
              <Button
                variant="outline"
                onClick={() => {
                  setDeleteGroupConfirm(null);
                  setDeleteGroupFolders(false);
                }}
              >
                Cancel
              </Button>
              <Button variant="destructive" onClick={handleDeleteGroup}>
                Delete Group
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* device code modal */}
      {deviceCodeInfo && (
        <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center">
          <div className="bg-card border border-border rounded-xl p-6 w-[360px] space-y-4 shadow-xl text-center">
            <h2 className="text-base font-semibold">Microsoft Sign In</h2>
            <p className="text-sm text-muted-foreground">
              {deviceCodeInfo.message || "Go to the link and enter the code."}
            </p>
            <div className="bg-muted rounded-lg p-4 space-y-2">
              <a
                href={deviceCodeInfo.verification_uri}
                target="_blank"
                rel="noreferrer"
                className="text-primary font-medium text-sm hover:underline block"
              >
                {deviceCodeInfo.verification_uri}
              </a>
              <div className="text-2xl font-bold font-mono tracking-widest">
                {deviceCodeInfo.user_code}
              </div>
            </div>
            <div className="spinner mx-auto w-5! h-5!" />
            <p className="text-xs text-muted-foreground">
              Waiting for authorization…
            </p>
            <Button
              variant="outline"
              onClick={() => {
                setDeviceCodeInfo(null);
              }}
            >
              Cancel
            </Button>
          </div>
        </div>
      )}

      {showJukebox && (
        <div className="fixed bottom-6 right-6 z-50 animate-in slide-in-from-right-8 fade-in duration-500 flex items-center gap-4 bg-zinc-950/90 backdrop-blur-md border border-zinc-800 p-4 rounded-xl shadow-2xl">
          <div className="w-14 h-14 bg-[#5E3F32] rounded border-2 border-[#38261E] flex flex-col items-center justify-center shadow-inner relative overflow-hidden">
            <div className="w-10 h-1.5 bg-black/80 rounded-full mb-1 border-b border-white/10" />
          </div>
          <div>
            <p className="text-[10px] font-bold text-emerald-500 mb-0.5 uppercase tracking-widest">
              Now Playing
            </p>
            <p className="text-sm font-medium text-white">
              C418 - {playingSong.name}
            </p>
          </div>
        </div>
      )}
    </SidebarProvider>
  );
}

export default App;
