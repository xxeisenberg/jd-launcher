import { useState, useEffect, useCallback } from "react";
import { commands } from "../bindings";
import type { ModpackSearchResult, ModpackVersion } from "../bindings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  DownloadIcon,
  UserIcon,
  SearchIcon,
  PackageIcon,
  CheckCircleIcon,
  XIcon,
} from "lucide-react";

interface ModpackBrowserProps {
  onInstalled: () => void;
}

export function ModpackBrowser({ onInstalled }: ModpackBrowserProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ModpackSearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [offset, setOffset] = useState(0);
  const [hasMore, setHasMore] = useState(true);

  const [selectedPack, setSelectedPack] = useState<ModpackSearchResult | null>(
    null,
  );
  const [versions, setVersions] = useState<ModpackVersion[]>([]);
  const [loadingVersions, setLoadingVersions] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [installedIds, setInstalledIds] = useState<Set<string>>(new Set());

  const loadInstalled = useCallback(async () => {
    const profiles = await commands.listProfiles();
    const ids = new Set<string>();
    for (const p of profiles) {
      if (p.modpack_info) ids.add(p.modpack_info.project_id);
    }
    setInstalledIds(ids);
  }, []);

  useEffect(() => {
    loadInstalled();
  }, [loadInstalled]);

  const doSearch = useCallback(
    async (q: string, off: number, append: boolean) => {
      if (!q.trim()) {
        if (!append) setResults([]);
        return;
      }
      setLoading(true);
      setError(null);
      try {
        const res = await commands.searchModpacks(q, off);
        if (res.status === "ok") {
          append
            ? setResults((p) => [...p, ...res.data])
            : setResults(res.data);
          setHasMore(res.data.length >= 20);
        } else {
          setError(res.error);
        }
      } catch (e: any) {
        setError(e.toString());
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  useEffect(() => {
    const t = setTimeout(() => {
      setOffset(0);
      doSearch(query, 0, false);
    }, 400);
    return () => clearTimeout(t);
  }, [query, doSearch]);

  const loadMore = () => {
    const next = offset + 20;
    setOffset(next);
    doSearch(query, next, true);
  };

  const openVersionPicker = async (pack: ModpackSearchResult) => {
    setSelectedPack(pack);
    setLoadingVersions(true);
    setVersions([]);
    try {
      const res = await commands.getModpackVersions(pack.project_id);
      if (res.status === "ok") setVersions(res.data);
      else setError(res.error);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setLoadingVersions(false);
    }
  };

  const handleInstall = async (
    pack: ModpackSearchResult,
    ver: ModpackVersion,
  ) => {
    setInstalling(true);
    setError(null);
    try {
      const res = await commands.installModpack(
        pack.project_id,
        ver.version_id,
        pack.title,
      );
      if (res.status === "ok") {
        setSelectedPack(null);
        await loadInstalled();
        onInstalled();
      } else {
        setError(res.error);
      }
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setInstalling(false);
    }
  };

  const fmtDl = (n: number) => {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
    return n.toString();
  };

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* search bar */}
      <div className="flex items-center gap-2 px-6 py-3">
        <div className="relative flex-1">
          <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search modpacks…"
            className="pl-9"
          />
        </div>
        {loading && <div className="spinner" />}
      </div>

      {error && (
        <div className="mx-6 mb-2 flex items-center gap-2 px-3 py-2 rounded-md bg-destructive/10 text-destructive text-xs">
          ⚠ {error}
          <button onClick={() => setError(null)} className="ml-auto">
            <XIcon className="w-3 h-3" />
          </button>
        </div>
      )}

      {/* empty states */}
      {results.length === 0 && !loading && query.trim() === "" && (
        <div className="flex-1 flex flex-col items-center justify-center gap-2 text-muted-foreground">
          <PackageIcon className="w-10 h-10 opacity-30" />
          <p className="text-sm">Search Modrinth to find modpacks</p>
        </div>
      )}
      {results.length === 0 && !loading && query.trim() !== "" && (
        <div className="flex-1 flex flex-col items-center justify-center gap-2 text-muted-foreground">
          <SearchIcon className="w-10 h-10 opacity-30" />
          <p className="text-sm">No results found</p>
        </div>
      )}

      {/* results grid */}
      <div className="flex-1 overflow-y-auto px-6 pb-4 grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-3 auto-rows-min">
        {results.map((pack) => (
          <button
            key={pack.project_id}
            onClick={() => openVersionPicker(pack)}
            className={`text-left flex gap-3 p-3 rounded-lg border transition-colors hover:bg-accent ${
              installedIds.has(pack.project_id)
                ? "border-primary/30"
                : "border-border"
            }`}
          >
            <div className="w-12 h-12 rounded-md bg-muted flex items-center justify-center shrink-0 overflow-hidden">
              {pack.icon_url ? (
                <img
                  src={pack.icon_url}
                  alt=""
                  className="w-full h-full object-cover"
                />
              ) : (
                <PackageIcon className="w-5 h-5 text-muted-foreground" />
              )}
            </div>
            <div className="flex-1 min-w-0 space-y-1">
              <h3 className="text-sm font-semibold truncate">{pack.title}</h3>
              <p className="text-xs text-muted-foreground line-clamp-2 leading-4">
                {pack.description}
              </p>
              <div className="flex items-center gap-3 text-[10px] text-muted-foreground">
                <span className="flex items-center gap-0.5">
                  <DownloadIcon className="w-3 h-3" /> {fmtDl(pack.downloads)}
                </span>
                <span className="flex items-center gap-0.5">
                  <UserIcon className="w-3 h-3" /> {pack.author}
                </span>
              </div>
              {installedIds.has(pack.project_id) && (
                <span className="flex items-center gap-1 text-[10px] text-primary font-medium">
                  <CheckCircleIcon className="w-3 h-3" /> Installed
                </span>
              )}
            </div>
          </button>
        ))}
      </div>

      {hasMore && results.length > 0 && (
        <div className="flex justify-center py-3">
          <Button
            variant="outline"
            size="sm"
            onClick={loadMore}
            disabled={loading}
          >
            {loading ? "Loading…" : "Load more"}
          </Button>
        </div>
      )}

      {/* version picker modal */}
      {selectedPack && (
        <div
          className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center"
          onClick={() => !installing && setSelectedPack(null)}
        >
          <div
            className="bg-card border border-border rounded-xl w-[520px] max-w-[calc(100vw-40px)] max-h-[calc(100vh-80px)] flex flex-col shadow-xl"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center gap-3 px-5 py-4 border-b border-border">
              {selectedPack.icon_url && (
                <img
                  src={selectedPack.icon_url}
                  alt=""
                  className="w-8 h-8 rounded"
                />
              )}
              <h2 className="text-base font-semibold flex-1 truncate">
                {selectedPack.title}
              </h2>
              <button
                onClick={() => !installing && setSelectedPack(null)}
                className="text-muted-foreground hover:text-foreground"
              >
                <XIcon className="w-4 h-4" />
              </button>
            </div>
            <div className="flex-1 overflow-y-auto p-5 space-y-2">
              <p className="text-xs text-muted-foreground mb-3">
                {selectedPack.description}
              </p>
              {loadingVersions ? (
                <div className="flex justify-center py-8">
                  <div className="spinner" />
                </div>
              ) : (
                versions.map((ver) => (
                  <div
                    key={ver.version_id}
                    className="flex items-center gap-3 p-3 rounded-md border border-border hover:border-border/80 transition-colors"
                  >
                    <div className="flex-1 min-w-0 space-y-1">
                      <span className="text-sm font-medium">{ver.name}</span>
                      <div className="flex gap-1 flex-wrap">
                        {ver.mc_versions.slice(0, 3).map((mc) => (
                          <Badge
                            key={mc}
                            variant="secondary"
                            className="text-[10px] px-1.5 py-0 h-4 font-mono"
                          >
                            {mc}
                          </Badge>
                        ))}
                        {ver.loaders
                          .filter((l) => l !== "modpack")
                          .map((l) => (
                            <Badge
                              key={l}
                              variant="outline"
                              className="text-[10px] px-1.5 py-0 h-4 text-purple-500 border-purple-500/20"
                            >
                              {l}
                            </Badge>
                          ))}
                      </div>
                    </div>
                    <Button
                      size="sm"
                      className="h-7"
                      onClick={() => handleInstall(selectedPack, ver)}
                      disabled={installing}
                    >
                      {installing ? "…" : "Install"}
                    </Button>
                  </div>
                ))
              )}
              {versions.length === 0 && !loadingVersions && (
                <p className="text-xs text-muted-foreground text-center py-4">
                  No versions available.
                </p>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
