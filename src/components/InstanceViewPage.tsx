import { useState, useEffect } from "react";
import { Profile, commands } from "../bindings";
import { Button } from "@/components/ui/button";
import {
  ArrowLeftIcon,
  FileBoxIcon,
  ImageIcon,
  PaletteIcon,
  SettingsIcon,
  PlayIcon,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

interface InstanceViewPageProps {
  profile: Profile;
  onBack: () => void;
  onLaunch: (profile: Profile) => void;
  onEdit: (profile: Profile) => void;
}

export function InstanceViewPage({
  profile,
  onBack,
  onLaunch,
  onEdit,
}: InstanceViewPageProps) {
  const [mods, setMods] = useState<string[]>([]);
  const [shaders, setShaders] = useState<string[]>([]);
  const [resourcePacks, setResourcePacks] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadDetails() {
      setLoading(true);
      try {
        const [modsRes, shadersRes, rpRes] = await Promise.all([
          commands.listMods(profile.id),
          commands.listShaders(profile.id),
          commands.listResourcePacks(profile.id),
        ]);

        if (modsRes.status === "ok") {
          setMods(modsRes.data);
        }
        if (shadersRes.status === "ok") {
          setShaders(shadersRes.data);
        }
        if (rpRes.status === "ok") {
          setResourcePacks(rpRes.data);
        }
      } catch (e) {
        console.error("Failed to load instance details", e);
      } finally {
        setLoading(false);
      }
    }

    loadDetails();
  }, [profile.id]);

  const modLabel =
    profile.modloader === "none"
      ? "Vanilla"
      : profile.modloader.charAt(0).toUpperCase() + profile.modloader.slice(1);

  return (
    <div className="flex flex-col h-full bg-background relative overflow-hidden">
      {/* Header Banner */}
      <div className="h-32 bg-primary/10 relative shrink-0">
        <div className="absolute inset-x-0 bottom-0 h-16 bg-linear-to-t from-background to-transparent" />
      </div>

      {/* Profile Header */}
      <div className="px-8 -mt-12 relative z-10 flex gap-6 pb-6 shrink-0 border-b">
        <div className="w-24 h-24 rounded-2xl bg-card border-4 border-background flex items-center justify-center shadow-md">
          <span className="text-4xl font-bold text-primary">
            {profile.name.charAt(0).toUpperCase()}
          </span>
        </div>
        <div className="flex-1 flex flex-col justify-end pb-1">
          <div className="flex items-end justify-between">
            <div>
              <div className="flex items-center gap-3">
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 -ml-2 rounded-full"
                  onClick={onBack}
                >
                  <ArrowLeftIcon className="w-4 h-4" />
                </Button>
                <h1 className="text-2xl font-bold tracking-tight">
                  {profile.name}
                </h1>
              </div>
              <div className="flex items-center gap-2 mt-2">
                <Badge variant="secondary" className="font-mono text-xs">
                  {profile.version}
                </Badge>
                <Badge variant="outline" className="text-xs">
                  {modLabel} {profile.modloader_version || ""}
                </Badge>
              </div>
            </div>
            <div className="flex gap-2">
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

      {/* Tabs Content */}
      <div className="flex-1 overflow-hidden px-8 py-6">
        <Tabs defaultValue="mods" className="h-full flex flex-col">
          <TabsList className="w-fit">
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
            <TabsTrigger value="settings" className="gap-2">
              <SettingsIcon className="w-4 h-4" /> Overview
            </TabsTrigger>
          </TabsList>

          <div className="flex-1 mt-6 overflow-hidden min-h-0 border rounded-xl bg-card relative">
            <TabsContent
              value="mods"
              className="absolute inset-0 m-0 p-0 focus-visible:outline-none data-[state=inactive]:hidden"
            >
              <ScrollArea className="h-full">
                <div className="p-4 sm:p-6 flex flex-col gap-1">
                  {loading ? (
                    <div className="text-sm text-muted-foreground flex items-center justify-center h-32">
                      Loading mods...
                    </div>
                  ) : mods.length === 0 ? (
                    <div className="flex flex-col items-center justify-center h-48 text-muted-foreground">
                      <FileBoxIcon className="w-8 h-8 opacity-20 mb-2" />
                      <p className="text-sm">No mods installed</p>
                      <p className="text-xs mt-1">
                        Place .jar files in this instance's mods folder
                      </p>
                    </div>
                  ) : (
                    mods.map((mod, i) => (
                      <div
                        key={i}
                        className="flex px-4 py-2 border rounded-lg items-center gap-3 bg-background group"
                      >
                        <FileBoxIcon className="w-4 h-4 text-muted-foreground" />
                        <span className="text-sm font-medium">{mod}</span>
                      </div>
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
                <div className="p-4 sm:p-6 flex flex-col gap-1">
                  {loading ? (
                    <div className="text-sm text-muted-foreground flex items-center justify-center h-32">
                      Loading shaders...
                    </div>
                  ) : shaders.length === 0 ? (
                    <div className="flex flex-col items-center justify-center h-48 text-muted-foreground">
                      <ImageIcon className="w-8 h-8 opacity-20 mb-2" />
                      <p className="text-sm">No shaders installed</p>
                      <p className="text-xs mt-1">
                        Place .zip files in this instance's shaderpacks folder
                      </p>
                    </div>
                  ) : (
                    shaders.map((shader, i) => (
                      <div
                        key={i}
                        className="flex px-4 py-2 border rounded-lg items-center gap-3 bg-background group"
                      >
                        <ImageIcon className="w-4 h-4 text-muted-foreground" />
                        <span className="text-sm font-medium">{shader}</span>
                      </div>
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
                <div className="p-4 sm:p-6 flex flex-col gap-1">
                  {loading ? (
                    <div className="text-sm text-muted-foreground flex items-center justify-center h-32">
                      Loading resource packs...
                    </div>
                  ) : resourcePacks.length === 0 ? (
                    <div className="flex flex-col items-center justify-center h-48 text-muted-foreground">
                      <PaletteIcon className="w-8 h-8 opacity-20 mb-2" />
                      <p className="text-sm">No resource packs installed</p>
                      <p className="text-xs mt-1">
                        Place .zip files in this instance's resourcepacks folder
                      </p>
                    </div>
                  ) : (
                    resourcePacks.map((pack, i) => (
                      <div
                        key={i}
                        className="flex px-4 py-2 border rounded-lg items-center gap-3 bg-background group"
                      >
                        <PaletteIcon className="w-4 h-4 text-muted-foreground" />
                        <span className="text-sm font-medium">{pack}</span>
                      </div>
                    ))
                  )}
                </div>
              </ScrollArea>
            </TabsContent>

            <TabsContent
              value="settings"
              className="absolute inset-0 m-0 focus-visible:outline-none data-[state=inactive]:hidden"
            >
              <ScrollArea className="h-full">
                <div className="p-6">
                  <div className="grid gap-6 md:grid-cols-2">
                    {/* Details Card */}
                    <div className="border rounded-xl p-5 bg-background/50">
                      <h3 className="text-sm font-semibold mb-4 text-muted-foreground">
                        Paths & Identifiers
                      </h3>
                      <div className="space-y-4">
                        <div>
                          <p className="text-[10px] uppercase font-semibold text-muted-foreground mb-1 tracking-wider">
                            Profile ID
                          </p>
                          <p className="text-sm font-mono bg-muted px-2 py-1 rounded w-fit">
                            {profile.id}
                          </p>
                        </div>
                        <div>
                          <p className="text-[10px] uppercase font-semibold text-muted-foreground mb-1 tracking-wider">
                            Game Directory
                          </p>
                          <p className="text-xs font-mono text-muted-foreground break-all">
                            {profile.game_dir}
                          </p>
                        </div>
                        <div>
                          <p className="text-[10px] uppercase font-semibold text-muted-foreground mb-1 tracking-wider">
                            Custom Java Path
                          </p>
                          <p className="text-xs font-mono text-muted-foreground break-all">
                            {profile.java_path || "System Default"}
                          </p>
                        </div>
                      </div>
                    </div>

                    {/* Tweaks Card */}
                    <div className="border rounded-xl p-5 bg-background/50">
                      <h3 className="text-sm font-semibold mb-4 text-muted-foreground">
                        Launch Tweaks
                      </h3>
                      <div className="space-y-4">
                        <div>
                          <p className="text-[10px] uppercase font-semibold text-muted-foreground mb-1 tracking-wider">
                            JVM Arguments
                          </p>
                          <p className="text-xs font-mono bg-muted p-2 rounded text-muted-foreground break-all">
                            {profile.jvm_args || "(None)"}
                          </p>
                        </div>
                        <div>
                          <p className="text-[10px] uppercase font-semibold text-muted-foreground mb-1 tracking-wider">
                            Window Resolution
                          </p>
                          <div className="flex gap-2">
                            <Badge variant="outline" className="font-mono">
                              {profile.resolution.width}x
                              {profile.resolution.height}
                            </Badge>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </ScrollArea>
            </TabsContent>
          </div>
        </Tabs>
      </div>
    </div>
  );
}
