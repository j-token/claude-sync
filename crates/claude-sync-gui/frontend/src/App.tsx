import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import Dashboard from "./components/Dashboard";
import SkillManager from "./components/SkillManager";
import SecretManager from "./components/SecretManager";
import SetupWizard from "./components/SetupWizard";
import type { SyncStatus } from "./lib/types";

type Tab = "dashboard" | "skills" | "secrets";

function App() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const [status, setStatus] = useState<SyncStatus | null>(null);
  const [showSetup, setShowSetup] = useState(false);
  const [loading, setLoading] = useState(true);

  const refreshStatus = useCallback(async () => {
    try {
      const result = await invoke<SyncStatus>("get_status");
      setStatus(result);
      // config가 없으면 셋업 위저드 표시
      if (!result.initialized) {
        setShowSetup(true);
      }
    } catch {
      setShowSetup(true);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  function handleSetupComplete() {
    setShowSetup(false);
    refreshStatus();
  }

  const tabs: { id: Tab; label: string }[] = [
    { id: "dashboard", label: "Dashboard" },
    { id: "skills", label: "Skills" },
    { id: "secrets", label: "Secrets" },
  ];

  if (loading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-gray-950 text-gray-400">
        Loading...
      </div>
    );
  }

  // 셋업이 안 되어 있으면 위저드 표시
  if (showSetup) {
    return (
      <div className="min-h-screen bg-gray-950 text-gray-100">
        <header className="border-b border-gray-800 px-6 py-4">
          <h1 className="text-xl font-bold">Claude Sync</h1>
          <p className="text-sm text-gray-400">Setup</p>
        </header>
        <main className="p-6">
          <SetupWizard
            onComplete={handleSetupComplete}
            onCancel={status?.initialized ? () => setShowSetup(false) : undefined}
          />
        </main>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-950 text-gray-100">
      {/* Header */}
      <header className="border-b border-gray-800 px-6 py-4">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-xl font-bold">Claude Sync</h1>
            <p className="text-sm text-gray-400">
              Sync Claude Code configuration across devices
            </p>
          </div>
          <div className="flex items-center gap-3">
            {status?.initialized && (
              <span className="rounded-full bg-gray-800 px-3 py-1 text-xs text-gray-400">
                {status.device_id}
              </span>
            )}
            <button
              onClick={() => setShowSetup(true)}
              className="rounded px-2 py-1 text-xs text-gray-500 hover:bg-gray-800 hover:text-gray-300 transition-colors"
              title="Settings"
            >
              Settings
            </button>
          </div>
        </div>

        {/* Tab Navigation */}
        <nav className="mt-4 flex gap-1">
          {tabs.map((t) => (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              className={`rounded-t-lg px-4 py-2 text-sm font-medium transition-colors ${
                tab === t.id
                  ? "bg-gray-900 text-white"
                  : "text-gray-500 hover:text-gray-300"
              }`}
            >
              {t.label}
            </button>
          ))}
        </nav>
      </header>

      {/* Content */}
      <main className="p-6">
        {tab === "dashboard" && (
          <Dashboard status={status} onRefresh={refreshStatus} />
        )}
        {tab === "skills" && <SkillManager />}
        {tab === "secrets" && <SecretManager />}
      </main>
    </div>
  );
}

export default App;
