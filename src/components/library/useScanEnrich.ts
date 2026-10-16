import { useCallback, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  enrichTracks,
  inTauri,
  listTracks,
  scanFolder,
  type EnrichSummary,
  type ScanSummary,
} from "@/lib/api";
import { useLibraryStore } from "@/store/libraryStore";

/** P10: scan-folder + batched-enrich flow, extracted from Library so the
    page only wires buttons and notes. */
export function useScanEnrich() {
  const [summary, setSummary] = useState<ScanSummary | null>(null);
  const [enrichNote, setEnrichNote] = useState<EnrichSummary | null>(null);
  const [scanning, setScanning] = useState(false);
  const [enriching, setEnriching] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    useLibraryStore.getState().setTracks(await listTracks());
  }, []);

  /** Enrich unmatched rows — batches of ≤ 50 until the well runs dry. */
  const runEnrich = useCallback(async (): Promise<EnrichSummary | null> => {
    if (!inTauri) return null;
    setEnriching(true);
    try {
      let total: EnrichSummary = { matched: 0, noMatch: 0, failed: 0, covers: 0, elapsedMs: 0 };
      for (;;) {
        const batch = await enrichTracks();
        total = {
          matched: total.matched + batch.matched,
          noMatch: total.noMatch + batch.noMatch,
          failed: total.failed + batch.failed,
          covers: total.covers + batch.covers,
          elapsedMs: total.elapsedMs + batch.elapsedMs,
        };
        if (batch.matched + batch.noMatch + batch.failed < 50) break;
      }
      setEnrichNote(total);
      await refresh();
      return total;
    } catch (enrichError) {
      setError(String(enrichError));
      return null;
    } finally {
      setEnriching(false);
    }
  }, [refresh]);

  const handleScan = useCallback(async () => {
    setError(null);
    const folder = await open({ directory: true });
    if (typeof folder !== "string") return;
    setScanning(true);
    try {
      const result = await scanFolder(folder);
      if (result) setSummary(result);
      await refresh();
      // Enrich on import (P5.4): only when the scan actually added rows.
      if (result && result.added > 0) await runEnrich();
    } catch (scanError) {
      setError(String(scanError));
    } finally {
      setScanning(false);
    }
  }, [refresh, runEnrich]);

  return { summary, enrichNote, scanning, enriching, error, refresh, runEnrich, handleScan };
}
