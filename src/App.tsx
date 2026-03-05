import { useState, useEffect, useCallback } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { commands } from "./bindings";
import type { Profile, LauncherSettings } from "./bindings";
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
  SidebarProvider,
  SidebarInset,
  SidebarRail,
  SidebarSeparator,
} from "@/components/ui/sidebar";

import { Avatar, AvatarImage, AvatarFallback } from "@/components/ui/avatar";
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
  LogOutIcon,
  ChevronsUpDownIcon,
  LogInIcon,
  UserIcon,
  TerminalIcon,
} from "lucide-react";

import { InstanceRow } from "./components/InstanceRow";
import { InstanceCard } from "./components/InstanceCard";
import { ProfileModal } from "./components/ProfileModal";
import { SettingsPage } from "./components/SettingsPage";
import { ConsoleDrawer } from "./components/ConsoleDrawer";
import { ModpackBrowser } from "./components/ModpackBrowser";
import { WelcomeScreen } from "./components/WelcomeScreen";
import { InstanceViewPage } from "./components/InstanceViewPage";
import { ModrinthBrowsePage } from "./components/ModrinthBrowsePage";

import "./App.css";
import { applyTheme } from "./lib/themes";

const ONBOARDING_KEY = "jd-launcher-onboarded";
const OFFLINE_USER_KEY = "jd-launcher-offline-username";

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
  const [error, setError] = useState<string | null>(null);
  const [currentView, setCurrentView] = useState<View>("instances");
  const [activeInstance, setActiveInstance] = useState<Profile | null>(null);
  const [instanceLayout, setInstanceLayout] = useState<InstanceLayout>("list");
  const [showConsole, setShowConsole] = useState(false);
  const [modrinthProjectType, setModrinthProjectType] = useState<
    "mod" | "shader" | "resourcepack"
  >("mod");
  const [deviceCodeInfo, setDeviceCodeInfo] = useState<any | null>(null);

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

  const isDownloading = progress !== null;

  const invalidateProfiles = useCallback(
    () => qc.invalidateQueries({ queryKey: ["profiles"] }),
    [qc],
  );

  const handleOnboardingComplete = useCallback(
    async (mode: "online" | "offline", offlineUser?: string) => {
      localStorage.setItem(ONBOARDING_KEY, "true");
      if (mode === "offline" && offlineUser) {
        localStorage.setItem(OFFLINE_USER_KEY, offlineUser);
        setUsername(offlineUser);
      }
      setOnboarded(true);
      qc.invalidateQueries({ queryKey: ["authMode"] });
      qc.invalidateQueries({ queryKey: ["activeAccount"] });
    },
    [qc],
  );

  useEffect(() => {
    const unlisten = listen<DownloadProgress>("download-progress", (event) => {
      if (event.payload.phase === "done") {
        setProgress(null);
        setLaunching(null);
      } else {
        setProgress(event.payload);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
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
        qc.invalidateQueries({ queryKey: ["activeAccount"] });
      } else if (account && account.status === "error") {
        setError(account.error as string);
      } else {
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
    await commands.deleteProfile(profile.id);
    setDeleteConfirm(null);
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
      case "client":
        return "Downloading client…";
      case "libraries":
        return "Downloading libraries…";
      case "assets":
        return "Downloading assets…";
      case "modloader":
        return "Downloading modloader…";
      default:
        return "Preparing…";
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
                className="cursor-default"
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
                <SidebarMenuItem>
                  <SidebarMenuButton
                    tooltip="Instances"
                    isActive={currentView === "instances"}
                    onClick={() => setCurrentView("instances")}
                  >
                    <BoxesIcon />
                    <span>Instances</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
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
                    <span className="text-xs text-muted-foreground">
                      Offline
                    </span>
                  </div>
                </SidebarMenuButton>
              )}
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarFooter>

        <SidebarRail />
      </Sidebar>

      <SidebarInset className="flex flex-col overflow-hidden">
        {/* breadcrumb bar */}
        <div
          className="flex items-center gap-2 px-6 h-12 border-b border-border shrink-0"
          data-tauri-drag-region
        >
          <span className="text-xs text-muted-foreground">Home</span>
          <ChevronRightIcon className="w-3 h-3 text-muted-foreground/50" />
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
            <div className="flex items-center gap-1.5">
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
        </div>

        {/* content */}
        <div className="flex-1 overflow-hidden flex flex-col relative">
          {/* error banner */}
          {error && (
            <div className="mx-6 mt-3 flex items-center gap-2 px-3 py-2 rounded-md bg-destructive/10 text-destructive text-sm">
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
                <div className="flex flex-col px-4 py-3 gap-0.5">
                  {profiles.map((profile) => (
                    <InstanceRow
                      key={profile.id}
                      profile={profile}
                      isLastUsed={profile.id === lastProfileId}
                      onLaunch={handleLaunch}
                      onEdit={(p) => setModal({ kind: "edit", profile: p })}
                      onDuplicate={handleDuplicate}
                      onDelete={(p) => setDeleteConfirm(p)}
                      onExport={handleExport}
                      onView={(p) => {
                        setActiveInstance(p);
                        setCurrentView("instance-view");
                      }}
                    />
                  ))}
                </div>
              ) : (
                <div className="grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-3 p-6">
                  {profiles.map((profile) => (
                    <InstanceCard
                      key={profile.id}
                      profile={profile}
                      isLastUsed={profile.id === lastProfileId}
                      onLaunch={handleLaunch}
                      onEdit={(p) => setModal({ kind: "edit", profile: p })}
                      onDuplicate={handleDuplicate}
                      onDelete={(p) => setDeleteConfirm(p)}
                      onExport={handleExport}
                      onView={(p) => {
                        setActiveInstance(p);
                        setCurrentView("instance-view");
                      }}
                    />
                  ))}
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
              onSettingsSaved={() =>
                qc.invalidateQueries({ queryKey: ["settings"] })
              }
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
          <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center">
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

      {/* delete confirm */}
      {deleteConfirm && (
        <div
          className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center"
          onClick={() => setDeleteConfirm(null)}
        >
          <div
            className="bg-card border border-border rounded-xl p-6 w-[360px] space-y-4 shadow-xl"
            onClick={(e) => e.stopPropagation()}
          >
            <h2 className="text-base font-semibold">Delete Profile?</h2>
            <p className="text-sm text-muted-foreground">
              Delete <strong>"{deleteConfirm.name}"</strong>? The game directory
              will <em>not</em> be removed.
            </p>
            <div className="flex gap-2 justify-end">
              <Button variant="outline" onClick={() => setDeleteConfirm(null)}>
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
    </SidebarProvider>
  );
}

export default App;
