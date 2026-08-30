import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  ChevronUpIcon,
  ChevronDownIcon,
  XIcon,
  CopyIcon,
  ExternalLinkIcon,
  SaveIcon,
  ServerIcon,
  TrashIcon,
} from "lucide-react";

interface GameLogEvent {
  src: string;
  line: string;
}

interface LanPortOpenedEvent {
  port: number;
}

interface PlayitActionUrlEvent {
  url: string;
  label: string;
}

interface PlayitTunnelReadyEvent {
  address: string;
}

interface PlayitStatusEvent {
  status: string;
  message: string;
}

interface ConsoleDrawerProps {
  visible: boolean;
  onClose: () => void;
}

export function ConsoleDrawer({ visible, onClose }: ConsoleDrawerProps) {
  const [logs, setLogs] = useState<GameLogEvent[]>([]);
  const [filter, setFilter] = useState("");
  const [expanded, setExpanded] = useState(false);
  const endRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const [lanPort, setLanPort] = useState<number | null>(null);
  const [playitAction, setPlayitAction] = useState<{
    url: string;
    label: string;
  } | null>(null);
  const [playitAddress, setPlayitAddress] = useState<string | null>(null);
  const [playitStatus, setPlayitStatus] = useState<string | null>(null);

  useEffect(() => {
    const unlisten = listen<GameLogEvent>("game-log", (event) => {
      setLogs((prev) => {
        const next = [...prev, event.payload];
        if (next.length > 5000) return next.slice(next.length - 5000);
        return next;
      });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const unlisteners = [
      listen<LanPortOpenedEvent>("lan-port-opened", (event) => {
        setLanPort(event.payload.port);
        setPlayitStatus(`LAN opened on port ${event.payload.port}`);
      }),
      listen<PlayitActionUrlEvent>("playit-action-url", (event) => {
        setPlayitAction(event.payload);
      }),
      listen<PlayitTunnelReadyEvent>("playit-tunnel-ready", (event) => {
        setPlayitAddress(event.payload.address);
        setPlayitStatus("Playit tunnel is ready");
      }),
      listen<PlayitStatusEvent>("playit-status", (event) => {
        if (event.payload.status !== "ready") {
          setPlayitAddress(null);
        }
        setPlayitStatus(event.payload.message);
      }),
    ];

    return () => {
      unlisteners.forEach((unlisten) => {
        unlisten.then((fn) => fn());
      });
    };
  }, []);

  useEffect(() => {
    if (autoScroll && endRef.current) {
      endRef.current.scrollIntoView({ behavior: "instant" });
    }
  }, [logs, autoScroll]);

  const handleScroll = () => {
    if (containerRef.current) {
      const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
      setAutoScroll(scrollHeight - scrollTop - clientHeight < 10);
    }
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(logs.map((l) => l.line).join("\n"));
  };

  const handleCopyPlayitAddress = () => {
    if (playitAddress) {
      navigator.clipboard.writeText(playitAddress);
    }
  };

  const handleSave = async () => {
    const dest = await save({
      defaultPath: "minecraft_log.txt",
      filters: [{ name: "Text File", extensions: ["txt"] }],
    });
    if (!dest) return;
    await invoke("save_log_file", {
      path: dest,
      content: logs.map((l) => l.line).join("\n"),
    });
  };

  const filteredLogs = filter
    ? logs.filter((l) => l.line.toLowerCase().includes(filter.toLowerCase()))
    : logs;

  if (!visible) return null;

  return (
    <div
      className={`absolute bottom-0 inset-x-0 z-40 border-t border-border bg-card flex flex-col shadow-2xl transition-all ${
        expanded ? "h-[70vh]" : "h-56"
      }`}
    >
      {/* toolbar */}
      <div className="flex items-center gap-2 px-4 py-2 border-b border-border shrink-0">
        <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
          Console
        </span>
        <div className="flex-1" />

        <Input
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="Filter…"
          className="h-6 text-xs w-40"
        />

        {!autoScroll && (
          <span className="text-[10px] text-muted-foreground">paused</span>
        )}

        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={() => setLogs([])}
        >
          <TrashIcon className="w-3 h-3" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={handleCopy}
        >
          <CopyIcon className="w-3 h-3" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={handleSave}
        >
          <SaveIcon className="w-3 h-3" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={() => setExpanded(!expanded)}
        >
          {expanded ? (
            <ChevronDownIcon className="w-3 h-3" />
          ) : (
            <ChevronUpIcon className="w-3 h-3" />
          )}
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={onClose}
        >
          <XIcon className="w-3 h-3" />
        </Button>
      </div>

      {(lanPort || playitAction || playitAddress || playitStatus) && (
        <div className="flex flex-wrap items-center gap-2 border-b border-border bg-muted/30 px-4 py-2 text-xs">
          <div className="flex min-w-0 flex-1 items-center gap-2">
            <ServerIcon className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
            <span className="shrink-0 font-medium text-foreground">Playit</span>
            <span className="min-w-0 truncate text-muted-foreground">
              {playitAddress ??
                playitStatus ??
                (lanPort ? `LAN opened on port ${lanPort}` : "Waiting for LAN")}
            </span>
          </div>

          {playitAction && (
            <Button
              variant="outline"
              size="sm"
              className="h-7 gap-1.5 px-2 text-xs"
              onClick={() => openUrl(playitAction.url)}
            >
              <ExternalLinkIcon className="h-3 w-3" />
              {playitAction.label}
            </Button>
          )}

          {playitAddress && (
            <Button
              variant="outline"
              size="sm"
              className="h-7 gap-1.5 px-2 text-xs"
              onClick={handleCopyPlayitAddress}
            >
              <CopyIcon className="h-3 w-3" />
              Copy
            </Button>
          )}
        </div>
      )}

      {/* log output */}
      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto min-h-0 px-4 py-2 font-mono text-xs leading-5"
      >
        {filteredLogs.map((log, i) => (
          <div
            key={i}
            className={`whitespace-pre-wrap ${
              log.line.includes("WARN")
                ? "text-yellow-600 dark:text-yellow-400"
                : log.src === "stderr" || log.line.includes("ERROR")
                  ? "text-destructive/80"
                  : "text-muted-foreground"
            }`}
          >
            {log.line}
          </div>
        ))}
        <div ref={endRef} />
      </div>
    </div>
  );
}
