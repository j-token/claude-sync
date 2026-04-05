import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PluginEntry } from "../lib/types";

export default function PluginManager() {
  const [plugins, setPlugins] = useState<PluginEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadPlugins();
  }, []);

  async function loadPlugins() {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<PluginEntry[]>("list_plugins");
      setPlugins(result);
    } catch (e) {
      setError(`${e}`);
    } finally {
      setLoading(false);
    }
  }

  function getSourceLabel(plugin: PluginEntry): string {
    if (plugin.source_repo) {
      return plugin.source_repo;
    }
    return plugin.source_type;
  }

  if (loading) {
    return <div className="p-6 text-gray-400">Loading plugins...</div>;
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Plugin Manager</h2>
        <button
          onClick={loadPlugins}
          className="rounded px-3 py-1 text-sm text-gray-400 hover:bg-gray-800 transition-colors"
        >
          Refresh
        </button>
      </div>

      <p className="text-sm text-gray-400">
        Plugin metadata (install list &amp; sources) is synced. Other devices can reinstall from this data.
      </p>

      {error && (
        <div className="rounded-lg border border-red-800 bg-red-950/30 p-3 text-sm text-red-400">
          {error}
        </div>
      )}

      {plugins.length === 0 ? (
        <p className="text-gray-500">No plugins installed</p>
      ) : (
        <div className="space-y-1">
          {plugins.map((plugin) => (
            <div
              key={plugin.id}
              className="flex items-center gap-3 rounded-lg border border-gray-800 bg-gray-900 p-3"
            >
              {/* Status indicator */}
              <div
                className={`h-2.5 w-2.5 rounded-full ${
                  plugin.enabled ? "bg-green-500" : "bg-gray-600"
                }`}
                title={plugin.enabled ? "Enabled" : "Disabled"}
              />

              {/* Plugin info */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium">{plugin.name}</span>
                  <span className="rounded bg-gray-800 px-1.5 py-0.5 text-xs text-gray-400">
                    v{plugin.version}
                  </span>
                  {plugin.enabled ? (
                    <span className="rounded bg-green-900 px-1.5 py-0.5 text-xs text-green-300">
                      Enabled
                    </span>
                  ) : (
                    <span className="rounded bg-gray-800 px-1.5 py-0.5 text-xs text-gray-500">
                      Disabled
                    </span>
                  )}
                </div>
                <div className="mt-0.5 flex items-center gap-2 text-xs text-gray-500">
                  <span>{plugin.marketplace}</span>
                  <span>&middot;</span>
                  <span className="truncate">{getSourceLabel(plugin)}</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Summary */}
      {plugins.length > 0 && (
        <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-3 text-sm text-gray-400">
          <div className="flex gap-4">
            <span>
              Total: <strong className="text-gray-200">{plugins.length}</strong>
            </span>
            <span>
              Enabled:{" "}
              <strong className="text-green-400">
                {plugins.filter((p) => p.enabled).length}
              </strong>
            </span>
            <span>
              Disabled:{" "}
              <strong className="text-gray-500">
                {plugins.filter((p) => !p.enabled).length}
              </strong>
            </span>
          </div>
        </div>
      )}
    </div>
  );
}
