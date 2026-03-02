import { useEffect, useState } from "react";
import { commands } from "../bindings";
import type { Profile } from "../bindings";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  PlayIcon,
  PencilIcon,
  CopyIcon,
  DownloadIcon,
  TrashIcon,
  MoreHorizontalIcon,
  ImageIcon,
} from "lucide-react";

const MODLOADER_COLORS: Record<string, string> = {
  fabric: "text-amber-600 bg-amber-500/10 border-amber-500/20",
  forge: "text-orange-500 bg-orange-500/10 border-orange-500/20",
  neoforge: "text-sky-500 bg-sky-500/10 border-sky-500/20",
  quilt: "text-purple-500 bg-purple-500/10 border-purple-500/20",
  none: "text-muted-foreground bg-muted border-border",
};

interface InstanceCardProps {
  profile: Profile;
  isLastUsed: boolean;
  onLaunch: (profile: Profile) => void;
  onEdit: (profile: Profile) => void;
  onDuplicate: (profile: Profile) => void;
  onDelete: (profile: Profile) => void;
  onExport: (profile: Profile) => void;
  onView: (profile: Profile) => void;
}

export function InstanceCard({
  profile,
  isLastUsed,
  onLaunch,
  onEdit,
  onDuplicate,
  onDelete,
  onExport,
  onView,
}: InstanceCardProps) {
  const modLabel =
    profile.modloader === "none"
      ? "Vanilla"
      : profile.modloader.charAt(0).toUpperCase() + profile.modloader.slice(1);
  const modClasses =
    MODLOADER_COLORS[profile.modloader] ?? MODLOADER_COLORS.none;

  const [requiredJava, setRequiredJava] = useState<number | null>(null);
  const [localJava, setLocalJava] = useState<number | null>(null);

  useEffect(() => {
    commands.getRequiredJavaVersion(profile.version).then(setRequiredJava);
    if (profile.java_path) {
      commands.getLocalJavaVersion(profile.java_path).then(setLocalJava);
    } else {
      setLocalJava(null);
    }
  }, [profile.version, profile.java_path]);

  const hasJavaWarning =
    profile.java_path &&
    requiredJava !== null &&
    localJava !== null &&
    localJava < requiredJava;

  return (
    <div
      className={`group relative flex flex-col gap-3 p-4 rounded-lg border bg-card transition-all hover:shadow-md ${
        isLastUsed
          ? "border-primary/30 ring-1 ring-primary/10"
          : "border-border"
      }`}
    >
      {isLastUsed && (
        <Badge className="absolute -top-2 right-3 text-[10px] px-2 py-0 h-4">
          Last Played
        </Badge>
      )}

      {/* header */}
      <div className="flex items-start gap-3">
        <div className="flex-shrink-0 w-10 h-10 rounded-md bg-primary/10 flex items-center justify-center">
          <span className="text-base font-bold text-primary">
            {profile.name.charAt(0).toUpperCase()}
          </span>
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="text-sm font-semibold truncate">{profile.name}</h3>
          <div className="flex items-center gap-1.5 mt-1 flex-wrap">
            <Badge
              variant="secondary"
              className="text-[10px] px-1.5 py-0 h-4 font-mono"
            >
              {profile.version}
            </Badge>
            <Badge
              variant="outline"
              className={`text-[10px] px-1.5 py-0 h-4 ${modClasses}`}
            >
              {modLabel}
              {profile.modloader_version ? ` ${profile.modloader_version}` : ""}
            </Badge>
          </div>
        </div>
      </div>

      {/* meta */}
      <div className="text-xs text-muted-foreground font-mono truncate px-1">
        {profile.game_dir.replace(/^.*\/instances\//, "instances/")}
      </div>
      {profile.java_path && (
        <div
          className={`text-xs px-1 ${hasJavaWarning ? "text-destructive" : "text-muted-foreground"}`}
        >
          Java {localJava ?? "?"}
          {hasJavaWarning && ` — requires ${requiredJava}`}
        </div>
      )}

      {/* actions */}
      <div className="flex items-center gap-2 pt-1">
        <Button
          size="sm"
          className="flex-1 h-8 gap-1.5"
          onClick={() => onLaunch(profile)}
        >
          <PlayIcon className="w-3.5 h-3.5" />
          Launch
        </Button>
        <Button
          size="sm"
          variant="secondary"
          className="flex-1 h-8 gap-1.5"
          onClick={() => onView(profile)}
        >
          <ImageIcon className="w-3.5 h-3.5" />
          View
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="outline"
              size="icon"
              className="h-8 w-8 px-0 shrink-0"
            >
              <MoreHorizontalIcon className="w-4 h-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-40">
            <DropdownMenuItem onClick={() => onEdit(profile)}>
              <PencilIcon className="w-3.5 h-3.5 mr-2" /> Edit
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => onDuplicate(profile)}>
              <CopyIcon className="w-3.5 h-3.5 mr-2" /> Duplicate
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => onExport(profile)}>
              <DownloadIcon className="w-3.5 h-3.5 mr-2" /> Export
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onDelete(profile)}
              className="text-destructive focus:text-destructive"
            >
              <TrashIcon className="w-3.5 h-3.5 mr-2" /> Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}
