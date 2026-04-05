import { useState, useEffect } from "react";
import { invokeCommand } from "../lib/backend";
import type { PluginEntry } from "../lib/types";

export default function PluginManager() {
  const [plugins, setPlugins] = useState<PluginEntry[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    loadPlugins();
  }, []);

  async function loadPlugins() {
    setLoading(true);
    setError(null);
    try {
      const result = await invokeCommand("list_plugins");
      setPlugins(result);
      setSelected(new Set(result.map((p) => p.id)));
    } catch (e) {
      setError(`${e}`);
    } finally {
      setLoading(false);
    }
  }

  async function handlePush() {
    setSyncing(true);
    setMessage(null);
    try {
      const result = await invokeCommand("push_selected_plugins", {
        ids: Array.from(selected),
      });
      setMessage(result);
      await loadPlugins();
    } catch (e) {
      setMessage(`Push failed: ${e}`);
    } finally {
      setSyncing(false);
    }
  }

  async function handlePull() {
    setSyncing(true);
    setMessage(null);
    try {
      const result = await invokeCommand("pull_selected_plugins", {
        ids: Array.from(selected),
      });
      setMessage(result);
      await loadPlugins();
    } catch (e) {
      setMessage(`Pull failed: ${e}`);
    } finally {
      setSyncing(false);
    }
  }

  function togglePlugin(id: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  }

  function toggleAll() {
    setSelected(
      selected.size === plugins.length ? new Set() : new Set(plugins.map((p) => p.id))
    );
  }

  if (loading) {
    return (
      <div className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)] p-4 text-sm text-[var(--color-fg-muted)]">
        Loading plugins...
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {/* Error / message banners */}
      {error && (
        <div className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">
          {error}
        </div>
      )}
      {message && (
        <div
          className={`rounded-md border px-3 py-2 text-sm ${
            message.toLowerCase().includes("fail")
              ? "border-red-200 bg-red-50 text-red-800"
              : "border-emerald-200 bg-emerald-50 text-emerald-800"
          }`}
        >
          {message}
        </div>
      )}

      {/* Table */}
      <div className="overflow-hidden rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)]">
        {/* Table header */}
        <div className="flex items-center justify-between border-b border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)] px-4 py-2.5">
          <div className="flex items-center gap-3">
            <input
              type="checkbox"
              checked={selected.size === plugins.length && plugins.length > 0}
              onChange={toggleAll}
              className="h-3.5 w-3.5 rounded accent-[var(--color-accent-fg)]"
            />
            <span className="text-sm font-semibold text-[var(--color-fg-default)]">
              {selected.size > 0 ? `${selected.size} selected` : `${plugins.length} plugins`}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={loadPlugins}
              className="rounded-md border border-[var(--color-btn-border)] bg-[var(--color-btn-bg)] px-2.5 py-1 text-xs font-medium text-[var(--color-fg-default)] hover:bg-[var(--color-btn-hover-bg)]"
            >
              Refresh
            </button>
            <button
              onClick={handlePull}
              disabled={selected.size === 0 || syncing}
              className="rounded-md border border-[var(--color-btn-border)] bg-[var(--color-btn-bg)] px-2.5 py-1 text-xs font-medium text-[var(--color-fg-default)] hover:bg-[var(--color-btn-hover-bg)] disabled:opacity-50"
            >
              Pull ({selected.size})
            </button>
            <button
              onClick={handlePush}
              disabled={selected.size === 0 || syncing}
              className="rounded-md bg-[var(--color-btn-primary-bg)] px-2.5 py-1 text-xs font-medium text-white hover:bg-[var(--color-btn-primary-hover-bg)] disabled:opacity-50"
            >
              Push ({selected.size})
            </button>
          </div>
        </div>

        {/* Rows */}
        {plugins.length === 0 ? (
          <div className="px-4 py-8 text-center text-sm text-[var(--color-fg-muted)]">
            No plugins installed.
          </div>
        ) : (
          <div className="divide-y divide-[var(--color-border-muted)]">
            {plugins.map((plugin) => (
              <label
                key={plugin.id}
                className="flex cursor-pointer items-center gap-4 px-4 py-2.5 transition hover:bg-[var(--color-canvas-subtle)]"
              >
                <input
                  type="checkbox"
                  checked={selected.has(plugin.id)}
                  onChange={() => togglePlugin(plugin.id)}
                  className="h-3.5 w-3.5 shrink-0 rounded accent-[var(--color-accent-fg)]"
                />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-[var(--color-accent-fg)]">
                      {plugin.name}
                    </span>
                    <span className="rounded-full border border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)] px-1.5 py-0.5 text-xs text-[var(--color-fg-muted)]">
                      v{plugin.version}
                    </span>
                    <span
                      className={`rounded-full px-2 py-0.5 text-xs font-medium ${
                        plugin.enabled
                          ? "bg-emerald-100 text-emerald-800"
                          : "bg-gray-100 text-[var(--color-fg-muted)]"
                      }`}
                    >
                      {plugin.enabled ? "Enabled" : "Disabled"}
                    </span>
                  </div>
                  <div className="mt-0.5 truncate text-xs text-[var(--color-fg-muted)]">
                    {plugin.source_repo ?? plugin.source_type}
                  </div>
                </div>
                <span className="hidden shrink-0 text-xs text-[var(--color-fg-muted)] sm:block">
                  {plugin.marketplace}
                </span>
                <span className="hidden shrink-0 text-xs text-[var(--color-fg-muted)] sm:block">
                  {plugin.source_type}
                </span>
              </label>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
