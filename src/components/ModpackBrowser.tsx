import { useState, useRef, useCallback } from "react";
import {
  useInfiniteQuery,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { commands } from "../bindings";
import type { ModpackSearchResult, ModpackVersion } from "../bindings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  DownloadIcon,
  SearchIcon,
  PackageIcon,
  CheckCircleIcon,
  XIcon,
} from "lucide-react";

interface ModpackBrowserProps {
  onInstalled: () => void;
}

function useDebouncedValue(value: string, ms = 400) {
  const [debounced, setDebounced] = useState(value);
  const timerRef = useRef<ReturnType<typeof setTimeout>>();
  if (timerRef.current) clearTimeout(timerRef.current);
  timerRef.current = setTimeout(() => setDebounced(value), ms);
  return debounced;
}

export function ModpackBrowser({ onInstalled }: ModpackBrowserProps) {
  const qc = useQueryClient();
  const [query, setQuery] = useState("");
  const debouncedQuery = useDebouncedValue(query);

  const [selectedPack, setSelectedPack] = useState<ModpackSearchResult | null>(
    null,
  );
  const [versions, setVersions] = useState<ModpackVersion[]>([]);
  const [loadingVersions, setLoadingVersions] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { data: installedIds = new Set<string>() } = useQuery({
    queryKey: ["installedModpacks"],
    queryFn: async () => {
      const profiles = await commands.listProfiles();
      const ids = new Set<string>();
      for (const p of profiles) {
        if (p.modpack_info) ids.add(p.modpack_info.project_id);
      }
      return ids;
    },
  });

  const { data, fetchNextPage, hasNextPage, isFetching, isFetchingNextPage } =
    useInfiniteQuery({
      queryKey: ["modpackSearch", debouncedQuery],
      queryFn: async ({ pageParam = 0 }) => {
        const res = await commands.searchModpacks(debouncedQuery, pageParam);
        if (res.status === "ok") return res.data;
        throw new Error(res.error);
      },
      initialPageParam: 0,
      getNextPageParam: (lastPage, allPages) =>
        lastPage.length >= 20 ? allPages.length * 20 : undefined,
    });

  const results = data?.pages.flat() ?? [];
  const loading = isFetching;

  // infinite scroll observer
  const observer = useRef<IntersectionObserver | null>(null);
  const lastElementRef = useCallback(
    (node: HTMLElement | null) => {
      if (isFetchingNextPage) return;
      if (observer.current) observer.current.disconnect();
      observer.current = new IntersectionObserver((entries) => {
        if (entries[0].isIntersecting && hasNextPage) {
          fetchNextPage();
        }
      });
      if (node) observer.current.observe(node);
    },
    [isFetchingNextPage, hasNextPage, fetchNextPage],
  );

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
        qc.invalidateQueries({ queryKey: ["installedModpacks"] });
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
    <div className="absolute inset-0 flex flex-col overflow-hidden">
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
      {results.length === 0 && !loading && (
        <div className="flex-1 flex flex-col items-center justify-center gap-2 text-muted-foreground">
          {query.trim() === "" ? (
            <>
              <PackageIcon className="w-10 h-10 opacity-30" />
              <p className="text-sm">No modpacks found</p>
            </>
          ) : (
            <>
              <SearchIcon className="w-10 h-10 opacity-30" />
              <p className="text-sm">No results found for "{query}"</p>
            </>
          )}
        </div>
      )}

      {/* scrollable area */}
      <div className="flex-1 overflow-y-auto px-6 pb-4">
        {/* results list */}
        <div className="flex flex-col gap-3">
          {results.map((pack) => {
            const isInstalled = installedIds.has(pack.project_id);
            return (
              <div
                key={pack.project_id}
                className={`group relative flex items-start gap-4 p-4 rounded-xl border bg-card/40 hover:bg-card/80 hover:shadow-lg transition-all duration-300 ${
                  isInstalled
                    ? "border-primary/30 shadow-[0_0_15px_-3px_rgba(var(--primary),0.1)]"
                    : "border-border/60"
                }`}
              >
                <div className="w-20 h-20 rounded-2xl bg-muted/50 flex items-center justify-center shrink-0 overflow-hidden shadow-inner border border-white/5">
                  {pack.icon_url ? (
                    <img
                      src={pack.icon_url}
                      alt=""
                      className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500"
                    />
                  ) : (
                    <PackageIcon className="w-8 h-8 text-muted-foreground/30" />
                  )}
                </div>

                <div className="flex-1 min-w-0 pt-0.5 space-y-2">
                  <div className="flex items-start justify-between gap-4">
                    <div className="min-w-0">
                      <div className="flex items-baseline gap-2 flex-wrap mb-1">
                        <h3 className="text-lg font-bold text-foreground tracking-tight line-clamp-1">
                          {pack.title}
                        </h3>
                        <span className="text-sm text-muted-foreground">
                          by
                        </span>
                        <span className="text-sm font-medium text-foreground/80 line-clamp-1">
                          {pack.author}
                        </span>
                      </div>
                      <p className="text-sm text-muted-foreground line-clamp-2 leading-relaxed pr-8">
                        {pack.description}
                      </p>
                    </div>

                    <div className="flex flex-col items-end gap-2.5 shrink-0">
                      <div className="flex items-center gap-4 text-xs font-medium text-muted-foreground mb-1">
                        <span className="flex items-center gap-1.5 transition-colors group-hover:text-foreground/80">
                          <DownloadIcon className="w-3.5 h-3.5" />{" "}
                          {fmtDl(pack.downloads)}
                        </span>
                      </div>
                      {isInstalled ? (
                        <Button
                          variant="secondary"
                          size="sm"
                          className="h-8 rounded-full px-4 text-green-500 bg-green-500/10 hover:bg-green-500/20 font-semibold cursor-default"
                        >
                          <CheckCircleIcon className="w-4 h-4 mr-1.5" />{" "}
                          Installed
                        </Button>
                      ) : (
                        <Button
                          onClick={() => openVersionPicker(pack)}
                          size="sm"
                          className="h-8 rounded-full px-5 font-semibold shadow-sm transition-all hover:scale-105 active:scale-95"
                        >
                          Install
                        </Button>
                      )}
                    </div>
                  </div>

                  <div className="flex items-center gap-2 flex-wrap pt-1">
                    {pack.categories?.slice(0, 5).map((c) => (
                      <Badge
                        key={c}
                        variant="secondary"
                        className="px-2 h-5.5 text-[11px] font-medium bg-secondary/40 hover:bg-secondary text-secondary-foreground border-transparent transition-colors"
                      >
                        {c}
                      </Badge>
                    ))}
                    {pack.latest_mc_version && (
                      <Badge
                        variant="outline"
                        className="px-2 h-5.5 text-[11px] font-medium text-primary/70 border-primary/20"
                      >
                        {pack.latest_mc_version}
                      </Badge>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>

        {hasNextPage && results.length > 0 && (
          <div ref={lastElementRef} className="flex justify-center py-3 h-10">
            {isFetchingNextPage && <div className="spinner" />}
          </div>
        )}
      </div>

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
