import { useState, useEffect, useCallback } from "react";
import Dashboard from "./components/Dashboard";
import SkillManager from "./components/SkillManager";
import SecretManager from "./components/SecretManager";
import PluginManager from "./components/PluginManager";
import Settings from "./components/SetupWizard";
import { invokeCommand } from "./lib/backend";
import type { SyncStatus } from "./lib/types";

type Tab = "dashboard" | "skills" | "plugins" | "secrets";

const tabs: { id: Tab; label: string }[] = [
  { id: "dashboard", label: "Overview" },
  { id: "skills", label: "Skills" },
  { id: "plugins", label: "Plugins" },
  { id: "secrets", label: "Secrets" },
];

function App() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const [status, setStatus] = useState<SyncStatus | null>(null);
  const [showSetup, setShowSetup] = useState(false);
  const [loading, setLoading] = useState(true);
  const [pushing, setPushing] = useState(false);
  const [pulling, setPulling] = useState(false);
  const [actionMsg, setActionMsg] = useState<string | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      const result = await invokeCommand("get_status");
      setStatus(result);
      if (!result.initialized) setShowSetup(true);
    } catch {
      setShowSetup(true);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  async function handlePush() {
    setPushing(true);
    setActionMsg(null);
    try {
      const result = await invokeCommand("sync_push");
      setActionMsg(result);
      refreshStatus();
    } catch (e) {
      setActionMsg(`Push failed: ${e}`);
    } finally {
      setPushing(false);
    }
  }

  async function handlePull() {
    setPulling(true);
    setActionMsg(null);
    try {
      const result = await invokeCommand("sync_pull");
      setActionMsg(result);
      refreshStatus();
    } catch (e) {
      setActionMsg(`Pull failed: ${e}`);
    } finally {
      setPulling(false);
    }
  }

  if (loading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-[var(--color-canvas-subtle)]">
        <div className="h-5 w-5 animate-spin rounded-full border-2 border-[var(--color-border-default)] border-t-[var(--color-accent-fg)]" />
      </div>
    );
  }

  if (showSetup) {
    return (
      <div className="min-h-screen bg-[var(--color-canvas-subtle)]">
        <header className="border-b border-[var(--color-border-default)] bg-[var(--color-canvas-default)] px-4 py-3">
          <div className="mx-auto flex max-w-[1012px] items-center justify-between">
            <div className="flex items-center gap-2">
              <svg width="20" height="20" viewBox="0 0 16 16" fill="var(--color-fg-default)">
                <path d="M8 0a8 8 0 1 1 0 16A8 8 0 0 1 8 0ZM5.78 8.75a9.64 9.64 0 0 0 1.363 4.177c.255.426.542.832.857 1.215.245-.296.551-.705.857-1.215A9.64 9.64 0 0 0 10.22 8.75Zm4.44-1.5a9.64 9.64 0 0 0-1.363-4.177c-.307-.51-.612-.919-.857-1.215a9.927 9.927 0 0 0-.857 1.215A9.64 9.64 0 0 0 5.78 7.25Z" />
              </svg>
              <span className="text-base font-semibold text-[var(--color-fg-default)]">
                Claude Sync
              </span>
            </div>
            <span className="rounded-full border border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)] px-2.5 py-0.5 text-xs font-medium text-[var(--color-fg-muted)]">
              Setup
            </span>
          </div>
        </header>
        <main className="mx-auto max-w-[1012px] px-4 py-6">
          <Settings
            onComplete={() => {
              setShowSetup(false);
              refreshStatus();
            }}
            onCancel={status?.initialized ? () => setShowSetup(false) : undefined}
            status={status}
          />
        </main>
      </div>
    );
  }

  const syncState = getSyncState(status);

  return (
    <div className="min-h-screen bg-[var(--color-canvas-subtle)]">
      {/* GitHub-style top header */}
      <header className="border-b border-[var(--color-border-default)] bg-[var(--color-canvas-default)]">
        <div className="mx-auto max-w-[1280px] px-4 lg:px-6">
          {/* Top bar: branding + actions */}
          <div className="flex h-12 items-center justify-between gap-4">
            <div className="flex items-center gap-3">
              <svg width="20" height="20" viewBox="0 0 16 16" fill="var(--color-fg-default)">
                <path d="M8 0a8 8 0 1 1 0 16A8 8 0 0 1 8 0ZM5.78 8.75a9.64 9.64 0 0 0 1.363 4.177c.255.426.542.832.857 1.215.245-.296.551-.705.857-1.215A9.64 9.64 0 0 0 10.22 8.75Zm4.44-1.5a9.64 9.64 0 0 0-1.363-4.177c-.307-.51-.612-.919-.857-1.215a9.927 9.927 0 0 0-.857 1.215A9.64 9.64 0 0 0 5.78 7.25Z" />
              </svg>
              <span className="text-base font-semibold text-[var(--color-fg-default)]">
                Claude Sync
              </span>
              {status?.repo_url && (
                <span className="hidden text-sm text-[var(--color-fg-muted)] sm:inline">
                  {status.repo_url}
                </span>
              )}
            </div>

            <div className="flex items-center gap-2">
              {/* Sync status badge */}
              <span
                className={`rounded-full px-2.5 py-0.5 text-xs font-medium ${
                  syncState.tone === "success"
                    ? "bg-emerald-100 text-emerald-800"
                    : syncState.tone === "warning"
                      ? "bg-amber-100 text-amber-800"
                      : "bg-gray-100 text-[var(--color-fg-muted)]"
                }`}
              >
                {syncState.label}
              </span>

              {/* Push / Pull */}
              <button
                onClick={handlePull}
                disabled={pulling}
                className="rounded-md border border-[var(--color-btn-border)] bg-[var(--color-btn-bg)] px-3 py-1 text-xs font-medium text-[var(--color-fg-default)] transition hover:bg-[var(--color-btn-hover-bg)] disabled:opacity-50"
              >
                {pulling ? "Pulling..." : "Pull"}
              </button>
              <button
                onClick={handlePush}
                disabled={pushing}
                className="rounded-md bg-[var(--color-btn-primary-bg)] px-3 py-1 text-xs font-medium text-white transition hover:bg-[var(--color-btn-primary-hover-bg)] disabled:opacity-50"
              >
                {pushing ? "Pushing..." : "Push"}
              </button>

              {/* Settings */}
              <button
                onClick={() => setShowSetup(true)}
                className="rounded-md border border-[var(--color-btn-border)] bg-[var(--color-btn-bg)] p-1.5 text-[var(--color-fg-muted)] transition hover:bg-[var(--color-btn-hover-bg)] hover:text-[var(--color-fg-default)]"
                title="Settings"
              >
                <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M8 0a8.2 8.2 0 0 1 .701.031C9.444.095 9.99.645 10.16 1.29l.288 1.107c.018.066.079.158.212.224.231.114.454.243.668.386.123.082.233.09.299.071l1.1-.303c.652-.18 1.37-.042 1.822.524.626.783 1.124 1.668 1.478 2.624.227.612.005 1.28-.475 1.725l-.813.804c-.05.048-.098.147-.088.294a6.09 6.09 0 0 1 0 .772c-.01.147.038.246.088.294l.813.804c.48.445.702 1.113.475 1.725a8.084 8.084 0 0 1-1.478 2.624c-.452.566-1.17.704-1.822.524l-1.1-.303c-.066-.019-.176-.011-.299.071a5.97 5.97 0 0 1-.668.386c-.133.066-.194.158-.211.224l-.29 1.107c-.168.645-.714 1.196-1.458 1.26a8.006 8.006 0 0 1-1.402 0c-.744-.064-1.29-.614-1.458-1.26l-.29-1.107c-.017-.066-.078-.158-.211-.224a5.977 5.977 0 0 1-.668-.386c-.123-.082-.233-.09-.299-.071l-1.1.303c-.652.18-1.37.042-1.822-.524a8.084 8.084 0 0 1-1.478-2.624c-.227-.612-.005-1.28.475-1.725l.813-.804c.05-.048.098-.147.088-.294a6.1 6.1 0 0 1 0-.772c.01-.147-.038-.246-.088-.294l-.813-.804C.32 6.07.098 5.402.325 4.79c.354-.956.852-1.841 1.478-2.624.452-.566 1.17-.704 1.822-.524l1.1.303c.066.019.176.011.299-.071.214-.143.437-.272.668-.386.133-.066.194-.158.212-.224L6.14 1.29C6.308.645 6.854.095 7.598.031 7.73.01 7.864 0 8 0Zm-.571 14.97c.183.016.37.016.553 0a.6.6 0 0 0 .38-.318l.29-1.107a1.858 1.858 0 0 1 1.053-1.142c.16-.079.313-.167.46-.261a1.86 1.86 0 0 1 1.546-.159l1.1.303a.6.6 0 0 0 .472-.137 6.583 6.583 0 0 0 1.2-2.134.6.6 0 0 0-.122-.464l-.813-.804a1.854 1.854 0 0 1-.472-1.49 4.578 4.578 0 0 0 0-.598 1.854 1.854 0 0 1 .472-1.49l.813-.804a.6.6 0 0 0 .122-.464 6.583 6.583 0 0 0-1.2-2.134.6.6 0 0 0-.472-.137l-1.1.303a1.86 1.86 0 0 1-1.546-.159 4.475 4.475 0 0 0-.46-.261A1.858 1.858 0 0 1 8.65 1.455l-.29-1.107a.6.6 0 0 0-.38-.318 6.509 6.509 0 0 0-.553 0 .6.6 0 0 0-.38.318l-.29 1.107a1.858 1.858 0 0 1-1.053 1.142c-.16.079-.313.167-.46.261a1.86 1.86 0 0 1-1.546.159l-1.1-.303a.6.6 0 0 0-.472.137 6.583 6.583 0 0 0-1.2 2.134.6.6 0 0 0 .122.464l.813.804a1.854 1.854 0 0 1 .472 1.49 4.577 4.577 0 0 0 0 .598 1.854 1.854 0 0 1-.472 1.49l-.813.804a.6.6 0 0 0-.122.464c.257.79.638 1.511 1.2 2.134a.6.6 0 0 0 .472.137l1.1-.303a1.86 1.86 0 0 1 1.546.159c.147.094.3.182.46.261a1.858 1.858 0 0 1 1.053 1.142l.29 1.107a.6.6 0 0 0 .38.318ZM8 10.5a2.5 2.5 0 1 1 0-5 2.5 2.5 0 0 1 0 5ZM8 9a1 1 0 1 0 0-2 1 1 0 0 0 0 2Z" />
                </svg>
              </button>

              {/* Refresh */}
              <button
                onClick={refreshStatus}
                className="rounded-md border border-[var(--color-btn-border)] bg-[var(--color-btn-bg)] p-1.5 text-[var(--color-fg-muted)] transition hover:bg-[var(--color-btn-hover-bg)] hover:text-[var(--color-fg-default)]"
                title="Refresh"
              >
                <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M1.705 8.005a.75.75 0 0 1 .834.656 5.5 5.5 0 0 0 9.592 2.97l-1.204-1.204a.25.25 0 0 1 .177-.427h3.646a.25.25 0 0 1 .25.25v3.646a.25.25 0 0 1-.427.177l-1.071-1.071A7.002 7.002 0 0 1 1.05 8.84a.75.75 0 0 1 .656-.834ZM8 2.5a5.487 5.487 0 0 0-4.131 1.869l1.204 1.204A.25.25 0 0 1 4.896 6H1.25A.25.25 0 0 1 1 5.75V2.104a.25.25 0 0 1 .427-.177l1.071 1.071A7.002 7.002 0 0 1 14.95 7.16a.75.75 0 0 1-1.49.178A5.5 5.5 0 0 0 8 2.5Z" />
                </svg>
              </button>
            </div>
          </div>

          {/* Tab bar */}
          <nav className="-mb-px flex gap-0 overflow-x-auto">
            {tabs.map((item) => {
              const isActive = item.id === tab;
              return (
                <button
                  key={item.id}
                  onClick={() => setTab(item.id)}
                  className={`relative whitespace-nowrap border-b-2 px-4 py-2 text-sm font-medium transition ${
                    isActive
                      ? "border-[var(--color-accent-fg)] text-[var(--color-fg-default)]"
                      : "border-transparent text-[var(--color-fg-muted)] hover:border-[var(--color-border-default)] hover:text-[var(--color-fg-default)]"
                  }`}
                >
                  {item.label}
                  {item.id === "skills" && status && status.skills_count > 0 && (
                    <span className="ml-1.5 rounded-full bg-[var(--color-neutral-muted)] px-1.5 text-xs">
                      {status.skills_count}
                    </span>
                  )}
                  {item.id === "plugins" && status && status.plugins_count > 0 && (
                    <span className="ml-1.5 rounded-full bg-[var(--color-neutral-muted)] px-1.5 text-xs">
                      {status.plugins_count}
                    </span>
                  )}
                </button>
              );
            })}
          </nav>
        </div>
      </header>

      {/* Action message banner */}
      {actionMsg && (
        <div
          className={`border-b px-4 py-2 text-center text-sm ${
            actionMsg.toLowerCase().includes("fail") || actionMsg.toLowerCase().includes("error")
              ? "border-red-200 bg-red-50 text-red-800"
              : "border-emerald-200 bg-emerald-50 text-emerald-800"
          }`}
        >
          {actionMsg}
          <button
            onClick={() => setActionMsg(null)}
            className="ml-3 text-xs underline opacity-70 hover:opacity-100"
          >
            dismiss
          </button>
        </div>
      )}

      {/* Main content */}
      <main className="mx-auto max-w-[1280px] px-4 py-4 lg:px-6">
        {tab === "dashboard" && <Dashboard status={status} onRefresh={refreshStatus} />}
        {tab === "skills" && <SkillManager />}
        {tab === "plugins" && <PluginManager />}
        {tab === "secrets" && <SecretManager />}
      </main>
    </div>
  );
}

function getSyncState(status: SyncStatus | null): {
  label: string;
  tone: "success" | "warning" | "neutral";
} {
  if (!status?.initialized) {
    return { label: "Setup required", tone: "warning" };
  }
  if (status.ahead === 0 && status.behind === 0 && status.dirty_files === 0) {
    return { label: "Up to date", tone: "success" };
  }
  const parts: string[] = [];
  if (status.ahead > 0) parts.push(`${status.ahead} ahead`);
  if (status.behind > 0) parts.push(`${status.behind} behind`);
  if (status.dirty_files > 0) parts.push(`${status.dirty_files} changed`);
  return { label: parts.join(", "), tone: "warning" };
}

export default App;
