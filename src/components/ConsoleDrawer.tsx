import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  ChevronUpIcon,
  ChevronDownIcon,
  XIcon,
  CopyIcon,
  SaveIcon,
  TrashIcon,
} from "lucide-react";

interface GameLogEvent {
  src: string;
  line: string;
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
      className={`border-t border-border bg-card flex flex-col transition-all ${
        expanded ? "h-[70%]" : "h-56"
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

      {/* log output */}
      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto px-4 py-2 font-mono text-xs leading-5"
      >
        {filteredLogs.map((log, i) => (
          <div
            key={i}
            className={`whitespace-pre-wrap ${
              log.src === "stderr"
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
