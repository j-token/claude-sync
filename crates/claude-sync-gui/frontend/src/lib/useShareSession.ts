import { useCallback, useEffect, useState } from "react";
import { invokeCommand } from "./backend";
import { errorText } from "./format";
import type { Notify } from "./notify";
import type { ShareInfo, ShareStatus } from "./types";

export interface ShareSession {
  info: ShareInfo | null;
  status: ShareStatus | null;
  isStarting: boolean;
  isActive: boolean;
  start: (ids: string[]) => Promise<void>;
  stop: () => Promise<void>;
}

const POLL_INTERVAL_MS = 1_000;

// 탭을 오가도 페어링 코드와 폴링이 끊기지 않도록 App 수준에서 보관한다.
export function useShareSession(notify: Notify): ShareSession {
  const [info, setInfo] = useState<ShareInfo | null>(null);
  const [status, setStatus] = useState<ShareStatus | null>(null);
  const [isStarting, setIsStarting] = useState(false);

  const isActive = info !== null && (status === null || status.phase === "waiting" || status.phase === "transferring");

  useEffect(() => {
    if (!isActive) return;
    const timer = window.setInterval(async () => {
      try {
        setStatus(await invokeCommand("p2p_share_status"));
      } catch (e) {
        notify("error", `공유 상태 확인 실패: ${errorText(e)}`);
      }
    }, POLL_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [isActive, notify]);

  const start = useCallback(
    async (ids: string[]) => {
      setIsStarting(true);
      try {
        const started = await invokeCommand("p2p_start_share", { ids });
        setStatus(null);
        setInfo(started);
      } catch (e) {
        notify("error", `공유 시작 실패: ${errorText(e)}`);
      } finally {
        setIsStarting(false);
      }
    },
    [notify],
  );

  const stop = useCallback(async () => {
    try {
      await invokeCommand("p2p_stop_share");
      setStatus(await invokeCommand("p2p_share_status"));
    } catch (e) {
      notify("error", `공유 중지 실패: ${errorText(e)}`);
    }
  }, [notify]);

  return { info, status, isStarting, isActive, start, stop };
}
