import { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { MinusIcon, SquareIcon, XIcon, CopyIcon } from "lucide-react";

export function TitleBar() {
  const [maximized, setMaximized] = useState(false);
  const appWindow = getCurrentWindow();

  useEffect(() => {
    appWindow.isMaximized().then(setMaximized);
    const unlisten = appWindow.onResized(() => {
      appWindow.isMaximized().then(setMaximized);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  return (
    <div className="flex items-center gap-0.5 -mr-2">
      <button
        onClick={() => appWindow.minimize()}
        className="h-7 w-7 inline-flex items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
      >
        <MinusIcon className="w-3.5 h-3.5" />
      </button>
      <button
        onClick={() => appWindow.toggleMaximize()}
        className="h-7 w-7 inline-flex items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
      >
        {maximized
          ? <CopyIcon className="w-3 h-3" />
          : <SquareIcon className="w-3 h-3" />
        }
      </button>
      <button
        onClick={() => appWindow.close()}
        className="h-7 w-7 inline-flex items-center justify-center rounded-md text-muted-foreground hover:bg-destructive/90 hover:text-white transition-colors"
      >
        <XIcon className="w-3.5 h-3.5" />
      </button>
    </div>
  );
}
