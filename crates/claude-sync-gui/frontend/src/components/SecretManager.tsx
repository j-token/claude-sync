import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DetectedSecret } from "../lib/types";

export default function SecretManager() {
  const [secrets, setSecrets] = useState<DetectedSecret[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadSecrets();
  }, []);

  async function loadSecrets() {
    setLoading(true);
    try {
      const result = await invoke<DetectedSecret[]>("list_secrets");
      setSecrets(result);
    } catch {
      // Config not initialized
      setSecrets([]);
    } finally {
      setLoading(false);
    }
  }

  if (loading) {
    return <div className="p-6 text-gray-400">Scanning secrets...</div>;
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Secret Manager</h2>
        <button
          onClick={loadSecrets}
          className="rounded px-3 py-1 text-sm text-gray-400 hover:bg-gray-800 transition-colors"
        >
          Scan Now
        </button>
      </div>

      <p className="text-sm text-gray-400">
        These values will be masked (replaced with empty string) when pushing to GitHub.
      </p>

      {secrets.length === 0 ? (
        <p className="text-gray-500">No secrets detected</p>
      ) : (
        <div className="space-y-1">
          {secrets.map((secret, i) => (
            <div
              key={i}
              className="flex items-center justify-between rounded-lg border border-gray-800 bg-gray-900 p-3"
            >
              <div className="min-w-0 flex-1">
                <p className="font-mono text-sm">{secret.json_path}</p>
                <p className="text-xs text-gray-500">Pattern: {secret.pattern_name}</p>
              </div>
              <div className="ml-4 flex items-center gap-2">
                <code className="rounded bg-gray-800 px-2 py-0.5 text-xs text-yellow-400">
                  {secret.preview}
                </code>
                <span className="rounded bg-red-900 px-2 py-0.5 text-xs text-red-300">
                  MASKED
                </span>
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-4 text-sm text-gray-400">
        <p>
          To add custom patterns, use the CLI:
        </p>
        <code className="mt-1 block rounded bg-gray-800 p-2 text-xs">
          claude-sync secret add "my pattern" "path.*.to.*_SECRET"
        </code>
      </div>
    </div>
  );
}
