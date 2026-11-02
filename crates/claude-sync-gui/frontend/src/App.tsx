import { useCallback, useEffect, useState } from "react";
import BackupsTab from "./components/BackupsTab";
import ImportTab from "./components/ImportTab";
import ItemsTab from "./components/ItemsTab";
import OverviewTab from "./components/OverviewTab";
import P2PTab from "./components/P2PTab";
import SecretsTab from "./components/SecretsTab";
import SettingsTab from "./components/SettingsTab";
import TemplatesTab from "./components/TemplatesTab";
import { invokeCommand } from "./lib/backend";
import { errorText } from "./lib/format";
import { NotifyContext, type NoticeTone } from "./lib/notify";
import type { Overview } from "./lib/types";
import { useShareSession } from "./lib/useShareSession";

export type TabId = "overview" | "items" | "import" | "p2p" | "backups" | "templates" | "secrets" | "settings";

const TABS: { id: TabId; label: string }[] = [
  { id: "overview", label: "개요" },
  { id: "items", label: "항목" },
  { id: "import", label: "가져오기" },
  { id: "p2p", label: "P2P" },
  { id: "backups", label: "백업" },
  { id: "templates", label: "템플릿" },
  { id: "secrets", label: "시크릿" },
  { id: "settings", label: "설정" },
];

interface Notice {
  tone: NoticeTone;
  text: string;
}

export default function App() {
  const [tab, setTab] = useState<TabId>("overview");
  const [overview, setOverview] = useState<Overview | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [shareIds, setShareIds] = useState<string[]>([]);

  const notify = useCallback((tone: NoticeTone, text: string) => setNotice({ tone, text }), []);
  const shareSession = useShareSession(notify);

  useEffect(() => {
    invokeCommand("get_overview")
      .then(setOverview)
      .catch((e) => notify("error", `개요를 불러오지 못했습니다: ${errorText(e)}`));
  }, [refreshKey, notify]);

  useEffect(() => {
    if (notice?.tone !== "success") return;
    const timer = window.setTimeout(() => setNotice(null), 5_000);
    return () => window.clearTimeout(timer);
  }, [notice]);

  const reloadAll = useCallback(() => setRefreshKey((key) => key + 1), []);

  async function shareFromItems(ids: string[]) {
    setShareIds(ids);
    setTab("p2p");
    await shareSession.start(ids);
  }

  return (
    <NotifyContext.Provider value={notify}>
      <div className="min-h-screen bg-[var(--color-canvas-subtle)]">
        <header className="border-b border-[var(--color-border-default)] bg-[var(--color-canvas-default)]">
          <div className="mx-auto max-w-[1280px] px-4 lg:px-6">
            <div className="flex h-12 items-center justify-between gap-4">
              <div className="flex min-w-0 items-center gap-3">
                <span className="text-base font-semibold">Claude Sync</span>
                {overview && (
                  <span className="truncate text-sm text-[var(--color-fg-muted)]">
                    {overview.device_name} · {overview.platform}
                  </span>
                )}
              </div>
              <div className="flex items-center gap-2">
                {shareSession.isActive && (
                  <button
                    type="button"
                    onClick={() => setTab("p2p")}
                    className="rounded-full border border-[#0969da66] px-2.5 py-0.5 text-xs font-medium text-[var(--color-accent-fg)]"
                  >
                    P2P 공유 중 {shareSession.info?.code}
                  </button>
                )}
                <button
                  type="button"
                  onClick={reloadAll}
                  title="새로고침"
                  aria-label="새로고침"
                  className="rounded-md border border-[var(--color-btn-border)] bg-[var(--color-btn-bg)] p-1.5 text-[var(--color-fg-muted)] transition hover:bg-[var(--color-btn-hover-bg)] hover:text-[var(--color-fg-default)]"
                >
                  <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M1.705 8.005a.75.75 0 0 1 .834.656 5.5 5.5 0 0 0 9.592 2.97l-1.204-1.204a.25.25 0 0 1 .177-.427h3.646a.25.25 0 0 1 .25.25v3.646a.25.25 0 0 1-.427.177l-1.071-1.071A7.002 7.002 0 0 1 1.05 8.84a.75.75 0 0 1 .656-.834ZM8 2.5a5.487 5.487 0 0 0-4.131 1.869l1.204 1.204A.25.25 0 0 1 4.896 6H1.25A.25.25 0 0 1 1 5.75V2.104a.25.25 0 0 1 .427-.177l1.071 1.071A7.002 7.002 0 0 1 14.95 7.16a.75.75 0 0 1-1.49.178A5.5 5.5 0 0 0 8 2.5Z" />
                  </svg>
                </button>
              </div>
            </div>

            <nav className="-mb-px flex overflow-x-auto">
              {TABS.map((item) => {
                const isActive = item.id === tab;
                return (
                  <button
                    type="button"
                    key={item.id}
                    onClick={() => setTab(item.id)}
                    className={`border-b-2 px-4 py-2 text-sm font-medium whitespace-nowrap transition ${
                      isActive
                        ? "border-[#fd8c73] text-[var(--color-fg-default)]"
                        : "border-transparent text-[var(--color-fg-muted)] hover:border-[var(--color-border-default)] hover:text-[var(--color-fg-default)]"
                    }`}
                  >
                    {item.label}
                    {item.id === "items" && overview && (
                      <span className="ml-1.5 rounded-full bg-[var(--color-neutral-muted)] px-1.5 text-xs">
                        {overview.total_items}
                      </span>
                    )}
                  </button>
                );
              })}
            </nav>
          </div>
        </header>

        {notice && (
          <div
            role="status"
            className={`border-b px-4 py-2 text-sm ${
              notice.tone === "error"
                ? "border-[#ff818266] bg-[#ffebe9] text-[var(--color-danger-fg)]"
                : "border-[#4ac26b66] bg-[#dafbe1] text-[var(--color-success-fg)]"
            }`}
          >
            <div className="mx-auto flex max-w-[1280px] items-start justify-between gap-3 lg:px-2">
              <span className="break-all">{notice.text}</span>
              <button
                type="button"
                onClick={() => setNotice(null)}
                className="shrink-0 text-xs underline opacity-70 hover:opacity-100"
              >
                닫기
              </button>
            </div>
          </div>
        )}

        <main className="mx-auto max-w-[1280px] px-4 py-4 lg:px-6">
          {/* 탭 전환 시 미리보기·입력 상태가 사라지지 않도록 모든 탭을 마운트해 두고 숨긴다. */}
          <div hidden={tab !== "overview"}>
            <OverviewTab
              overview={overview}
              onNavigate={setTab}
              onChanged={reloadAll}
            />
          </div>
          <div hidden={tab !== "items"}>
            <ItemsTab refreshKey={refreshKey} onChanged={reloadAll} onShare={shareFromItems} />
          </div>
          <div hidden={tab !== "import"}>
            <ImportTab onChanged={reloadAll} />
          </div>
          <div hidden={tab !== "p2p"}>
            <P2PTab
              refreshKey={refreshKey}
              session={shareSession}
              pendingIds={shareIds}
              onClearPending={() => setShareIds([])}
              onChanged={reloadAll}
            />
          </div>
          <div hidden={tab !== "backups"}>
            <BackupsTab refreshKey={refreshKey} onChanged={reloadAll} />
          </div>
          <div hidden={tab !== "templates"}>
            <TemplatesTab refreshKey={refreshKey} onChanged={reloadAll} />
          </div>
          <div hidden={tab !== "secrets"}>
            <SecretsTab refreshKey={refreshKey} />
          </div>
          <div hidden={tab !== "settings"}>
            <SettingsTab refreshKey={refreshKey} onChanged={reloadAll} />
          </div>
        </main>
      </div>
    </NotifyContext.Provider>
  );
}
