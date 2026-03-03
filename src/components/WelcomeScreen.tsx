import { useState } from "react";
import { commands } from "../bindings";
import type { DeviceCodeInfo } from "../bindings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { UserIcon, LogInIcon, WifiOffIcon, ArrowRightIcon } from "lucide-react";

interface WelcomeScreenProps {
  onComplete: (mode: "online" | "offline", username?: string) => void;
}

export function WelcomeScreen({ onComplete }: WelcomeScreenProps) {
  const [step, setStep] = useState<"choose" | "offline" | "online">("choose");
  const [username, setUsername] = useState("");
  const [error, setError] = useState<string | null>(null);

  // ms login state
  const [deviceCode, setDeviceCode] = useState<DeviceCodeInfo | null>(null);
  const [polling, setPolling] = useState(false);

  const handleMsLogin = async () => {
    setStep("online");
    setError(null);
    try {
      const info = await commands.startMsLogin();
      if (info.status === "ok") {
        setDeviceCode(info.data);
        setPolling(true);
        const account = await commands.pollMsLogin();
        if (account.status === "ok") {
          onComplete("online");
        } else {
          setError(account.error as string);
          setPolling(false);
        }
      } else {
        setError(info.error as string);
        setStep("choose");
      }
    } catch (e: any) {
      setError(e.toString());
      setStep("choose");
    }
  };

  const handleOfflineSubmit = () => {
    const trimmed = username.trim();
    if (!trimmed) return;
    onComplete("offline", trimmed);
  };

  return (
    <div className="h-full w-full flex items-center justify-center bg-background">
      <div className="w-[400px] flex flex-col items-center gap-6">
        {/* logo */}
        <div className="flex flex-col items-center gap-3">
          <div className="flex items-center justify-center">
            <img
              src="/logo.svg"
              alt="Logo"
              className="w-16 h-16 drop-shadow-sm"
            />
          </div>
          <h1 className="text-xl font-bold tracking-tight">JD Launcher</h1>
          <p className="text-sm text-muted-foreground text-center">
            {step === "choose" && "How would you like to play?"}
            {step === "offline" && "Enter a username for offline play"}
            {step === "online" && "Sign in with Microsoft"}
          </p>
        </div>

        {error && (
          <div className="w-full px-3 py-2 rounded-md bg-destructive/10 text-destructive text-xs">
            {error}
          </div>
        )}

        {/* choose */}
        {step === "choose" && (
          <div className="w-full flex flex-col gap-3">
            <button
              onClick={handleMsLogin}
              className="w-full flex items-center gap-4 p-4 rounded-lg border border-border bg-card hover:bg-accent transition-colors text-left"
            >
              <div className="w-10 h-10 rounded-md bg-primary/10 flex items-center justify-center shrink-0">
                <LogInIcon className="w-5 h-5 text-primary" />
              </div>
              <div className="flex-1 min-w-0">
                <span className="text-sm font-semibold block">
                  Sign in with Microsoft
                </span>
                <span className="text-xs text-muted-foreground">
                  Play on online servers and access your skins
                </span>
              </div>
              <ArrowRightIcon className="w-4 h-4 text-muted-foreground shrink-0" />
            </button>

            <button
              onClick={() => setStep("offline")}
              className="w-full flex items-center gap-4 p-4 rounded-lg border border-border bg-card hover:bg-accent transition-colors text-left"
            >
              <div className="w-10 h-10 rounded-md bg-muted flex items-center justify-center shrink-0">
                <WifiOffIcon className="w-5 h-5 text-muted-foreground" />
              </div>
              <div className="flex-1 min-w-0">
                <span className="text-sm font-semibold block">
                  Continue Offline
                </span>
                <span className="text-xs text-muted-foreground">
                  Play singleplayer with a custom username
                </span>
              </div>
              <ArrowRightIcon className="w-4 h-4 text-muted-foreground shrink-0" />
            </button>
          </div>
        )}

        {/* offline username */}
        {step === "offline" && (
          <div className="w-full flex flex-col gap-3">
            <div className="relative">
              <UserIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
              <Input
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleOfflineSubmit()}
                placeholder="Steve"
                maxLength={16}
                autoFocus
                className="pl-9"
              />
            </div>
            <div className="flex gap-2">
              <Button
                variant="outline"
                className="flex-1"
                onClick={() => setStep("choose")}
              >
                Back
              </Button>
              <Button
                className="flex-1"
                onClick={handleOfflineSubmit}
                disabled={!username.trim()}
              >
                Continue
              </Button>
            </div>
          </div>
        )}

        {/* ms login flow */}
        {step === "online" && deviceCode && (
          <div className="w-full flex flex-col items-center gap-4 p-5 rounded-lg border border-border bg-card">
            <p className="text-xs text-muted-foreground text-center">
              Open the link below and enter the code
            </p>
            <a
              href={deviceCode.verification_uri}
              target="_blank"
              rel="noreferrer"
              className="text-primary text-sm font-medium hover:underline"
            >
              {deviceCode.verification_uri}
            </a>
            <div className="text-2xl font-bold font-mono tracking-[0.2em]">
              {deviceCode.user_code}
            </div>
            {polling && (
              <>
                <div className="spinner" />
                <p className="text-xs text-muted-foreground">
                  Waiting for authorization…
                </p>
              </>
            )}
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setStep("choose");
                setDeviceCode(null);
                setPolling(false);
              }}
            >
              Cancel
            </Button>
          </div>
        )}
        {step === "online" && !deviceCode && !error && (
          <div className="flex flex-col items-center gap-3">
            <div className="spinner" />
            <p className="text-xs text-muted-foreground">Connecting…</p>
          </div>
        )}
      </div>
    </div>
  );
}
