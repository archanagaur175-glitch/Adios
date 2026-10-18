import { useCallback, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { listTracks, scanFolder, type ScanSummary } from "@/lib/api";
import { useLibraryStore } from "@/store/libraryStore";

const ANDROID_ROOT = "/storage/emulated/0";

/** P12: scan-folder flow (import enrichment removed at the user's request).
    P18.1c: Android's plugin-dialog rejects `open({directory:true})` with
    "Folder picker is not implemented on mobile" — that rejection opens the
    in-app FolderPicker instead; desktop keeps the native dialog. */
export function useScan() {
  const [summary, setSummary] = useState<ScanSummary | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);
  const [pickStart, setPickStart] = useState(ANDROID_ROOT);

  const refresh = useCallback(async () => {
    useLibraryStore.getState().setTracks(await listTracks());
  }, []);

  const runScan = useCallback(
    async (folder: string) => {
      setScanning(true);
      try {
        const result = await scanFolder(folder);
        if (result) setSummary(result);
        await refresh();
      } catch (scanError) {
        setError(String(scanError));
      } finally {
        setScanning(false);
      }
    },
    [refresh],
  );

  const handleScan = useCallback(async () => {
    setError(null);
    try {
      const picked = await open({ directory: true });
      if (typeof picked === "string") await runScan(picked);
    } catch {
      setPickStart(
        navigator.userAgent.includes("Android") ? ANDROID_ROOT : "/",
      );
      setPicking(true);
    }
  }, [runScan]);

  const pickFolder = useCallback(
    (folder: string) => {
      setPicking(false);
      void runScan(folder);
    },
    [runScan],
  );

  const cancelPick = useCallback(() => setPicking(false), []);

  return {
    summary,
    scanning,
    error,
    refresh,
    handleScan,
    picking,
    pickStart,
    pickFolder,
    cancelPick,
  };
}
