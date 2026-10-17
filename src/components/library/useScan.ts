import { useCallback, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { listTracks, scanFolder, type ScanSummary } from "@/lib/api";
import { useLibraryStore } from "@/store/libraryStore";

/** P12: scan-folder flow (import enrichment removed at the user's request). */
export function useScan() {
  const [summary, setSummary] = useState<ScanSummary | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    useLibraryStore.getState().setTracks(await listTracks());
  }, []);

  const handleScan = useCallback(async () => {
    setError(null);
    const folder = await open({ directory: true });
    if (typeof folder !== "string") return;
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
  }, [refresh]);

  return { summary, scanning, error, refresh, handleScan };
}
