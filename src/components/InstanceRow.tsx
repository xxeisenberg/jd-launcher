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

interface InstanceRowProps {
  profile: Profile;
  isLastUsed: boolean;
  onLaunch: (profile: Profile) => void;
  onEdit: (profile: Profile) => void;
  onDuplicate: (profile: Profile) => void;
  onDelete: (profile: Profile) => void;
  onExport: (profile: Profile) => void;
  onView: (profile: Profile) => void;
}

export function InstanceRow({
  profile,
  isLastUsed,
  onLaunch,
  onEdit,
  onDuplicate,
  onDelete,
  onExport,
  onView,
}: InstanceRowProps) {
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
      className={`group flex items-center gap-4 px-4 py-3 rounded-lg border transition-colors hover:bg-accent/50 ${
        isLastUsed ? "border-primary/30 bg-primary/5" : "border-transparent"
      }`}
    >
      {/* icon */}
      <div className="flex-shrink-0 w-9 h-9 rounded-md bg-primary/10 flex items-center justify-center">
        <span className="text-sm font-bold text-primary">
          {profile.name.charAt(0).toUpperCase()}
        </span>
      </div>

      {/* info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold truncate">{profile.name}</span>
          {isLastUsed && (
            <Badge
              variant="outline"
              className="text-[10px] px-1.5 py-0 h-4 border-primary/30 text-primary"
            >
              Last
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-2 mt-0.5">
          <span className="text-xs text-muted-foreground font-mono">
            {profile.version}
          </span>
          <Badge
            variant="outline"
            className={`text-[10px] px-1.5 py-0 h-4 ${modClasses}`}
          >
            {modLabel}
          </Badge>
          {profile.java_path && (
            <span
              className={`text-xs ${hasJavaWarning ? "text-destructive" : "text-muted-foreground"}`}
            >
              Java {localJava ?? "?"}
              {hasJavaWarning && ` ⚠`}
            </span>
          )}
        </div>
      </div>

      {/* actions */}
      <div className="flex items-center gap-1.5 opacity-0 group-hover:opacity-100 transition-opacity">
        <Button
          size="sm"
          className="h-7 gap-1.5 px-3"
          onClick={() => onLaunch(profile)}
        >
          <PlayIcon className="w-3 h-3" />
          Launch
        </Button>
        <Button
          size="sm"
          variant="secondary"
          className="h-7 gap-1.5 px-3"
          onClick={() => onView(profile)}
        >
          <ImageIcon className="w-3 h-3" />
          View
        </Button>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon" className="h-7 w-7">
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
