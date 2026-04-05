import { useState, useEffect } from "react";
import { invokeCommand } from "../lib/backend";
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
      const result = await invokeCommand("list_secrets");
      setSecrets(result);
    } catch {
      setSecrets([]);
    } finally {
      setLoading(false);
    }
  }

  if (loading) {
    return (
      <div className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)] p-4 text-sm text-[var(--color-fg-muted)]">
        Scanning for secrets...
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {/* Table */}
      <div className="overflow-hidden rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)]">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)] px-4 py-2.5">
          <span className="text-sm font-semibold text-[var(--color-fg-default)]">
            {secrets.length} detected secret{secrets.length !== 1 ? "s" : ""}
          </span>
          <button
            onClick={loadSecrets}
            className="rounded-md border border-[var(--color-btn-border)] bg-[var(--color-btn-bg)] px-2.5 py-1 text-xs font-medium text-[var(--color-fg-default)] hover:bg-[var(--color-btn-hover-bg)]"
          >
            Rescan
          </button>
        </div>

        {/* Rows */}
        {secrets.length === 0 ? (
          <div className="px-4 py-8 text-center">
            <div className="text-sm font-medium text-[var(--color-fg-default)]">
              No secrets detected
            </div>
            <p className="mt-1 text-xs text-[var(--color-fg-muted)]">
              Run another scan after adding tokens or API keys.
            </p>
          </div>
        ) : (
          <div className="divide-y divide-[var(--color-border-muted)]">
            {secrets.map((secret, i) => (
              <div
                key={`${secret.json_path}-${i}`}
                className="flex items-center gap-4 px-4 py-2.5"
              >
                <div className="min-w-0 flex-1">
                  <div className="truncate font-mono text-sm text-[var(--color-fg-default)]">
                    {secret.json_path}
                  </div>
                </div>
                <span className="hidden shrink-0 text-xs text-[var(--color-fg-muted)] sm:block">
                  {secret.pattern_name}
                </span>
                <code className="hidden shrink-0 rounded bg-[var(--color-canvas-subtle)] px-1.5 py-0.5 text-xs text-[var(--color-attention-fg)] sm:block">
                  {secret.preview}
                </code>
                <span className="shrink-0 rounded-full bg-red-100 px-2 py-0.5 text-xs font-medium text-red-800">
                  Masked
                </span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* CLI hint */}
      <div className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)] px-4 py-3">
        <p className="text-xs text-[var(--color-fg-muted)]">
          Add custom patterns via CLI:{" "}
          <code className="rounded bg-[var(--color-canvas-subtle)] px-1.5 py-0.5 text-xs text-[var(--color-fg-default)]">
            claude-sync secret add "pattern" "path.*.to.*_SECRET"
          </code>
        </p>
      </div>
    </div>
  );
}
