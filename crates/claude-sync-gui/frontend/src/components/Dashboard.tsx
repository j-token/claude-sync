import { useState } from "react";
import { invokeCommand } from "../lib/backend";
import type { SyncStatus } from "../lib/types";

interface Props {
  status: SyncStatus | null;
  onRefresh: () => void;
}

export default function Dashboard({ status, onRefresh }: Props) {
  if (!status) {
    return (
      <div className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)] p-4 text-sm text-[var(--color-fg-muted)]">
        Loading status...
      </div>
    );
  }

  if (!status.initialized) {
    return (
      <div className="rounded-md border border-amber-300 bg-amber-50 p-4">
        <p className="text-sm font-medium text-amber-800">
          Setup required. Run <code className="rounded bg-white/60 px-1 py-0.5 text-xs">claude-sync init</code> or
          open settings to connect this machine.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Metrics row */}
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        <MetricCard label="Ahead" value={status.ahead} color="text-[var(--color-accent-fg)]" />
        <MetricCard label="Behind" value={status.behind} color="text-[var(--color-attention-fg)]" />
        <MetricCard label="Tracked" value={status.syncable_files} />
        <MetricCard label="Dirty" value={status.dirty_files} color={status.dirty_files > 0 ? "text-[var(--color-danger-fg)]" : undefined} />
      </div>

      {/* Two-column layout */}
      <div className="grid gap-4 lg:grid-cols-[1fr_320px]">
        {/* Connection info */}
        <div className="overflow-hidden rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)]">
          <div className="border-b border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)] px-4 py-2.5">
            <span className="text-sm font-semibold text-[var(--color-fg-default)]">
              Connection
            </span>
          </div>
          <div className="divide-y divide-[var(--color-border-muted)]">
            <InfoRow label="Device" value={status.device_id} />
            <InfoRow label="Repository" value={status.repo_url} mono />
            <InfoRow label="Last sync" value={status.last_sync ?? "Never"} />
            <InfoRow label="Git" value={status.git_available ? "Available" : "Missing"} />
          </div>
        </div>

        {/* Sync options — 토글 스위치 */}
        <SyncOptionsCard status={status} onRefresh={onRefresh} />
      </div>
    </div>
  );
}

/** Dashboard에서 sync 옵션을 즉시 토글할 수 있는 카드 */
function SyncOptionsCard({
  status,
  onRefresh,
}: {
  status: SyncStatus;
  onRefresh: () => void;
}) {
  const [memory, setMemory] = useState(status.sync_memory);
  const [teams, setTeams] = useState(status.sync_teams);
  const [skills, setSkills] = useState(status.sync_skills);
  const [plugins, setPlugins] = useState(status.sync_plugins);

  async function update(next: {
    sync_memory: boolean;
    sync_teams: boolean;
    sync_skills: boolean;
    sync_plugins: boolean;
  }) {
    try {
      await invokeCommand("update_sync_options", { input: next });
      onRefresh();
    } catch {
      // 실패 시 원래 값 복원
      setMemory(status.sync_memory);
      setTeams(status.sync_teams);
      setSkills(status.sync_skills);
      setPlugins(status.sync_plugins);
    }
  }

  function toggle(key: "memory" | "teams" | "skills" | "plugins", value: boolean) {
    const next = {
      sync_memory: key === "memory" ? value : memory,
      sync_teams: key === "teams" ? value : teams,
      sync_skills: key === "skills" ? value : skills,
      sync_plugins: key === "plugins" ? value : plugins,
    };
    if (key === "memory") setMemory(value);
    if (key === "teams") setTeams(value);
    if (key === "skills") setSkills(value);
    if (key === "plugins") setPlugins(value);
    update(next);
  }

  const options: { label: string; key: "memory" | "teams" | "skills" | "plugins"; checked: boolean }[] = [
    { label: "Memory", key: "memory", checked: memory },
    { label: "Teams", key: "teams", checked: teams },
    { label: "Skills", key: "skills", checked: skills },
    { label: "Plugins", key: "plugins", checked: plugins },
  ];

  return (
    <div className="overflow-hidden rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)]">
      <div className="border-b border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)] px-4 py-2.5">
        <span className="text-sm font-semibold text-[var(--color-fg-default)]">
          Sync options
        </span>
      </div>
      <div className="divide-y divide-[var(--color-border-muted)]">
        {options.map((opt) => (
          <label
            key={opt.key}
            className="flex cursor-pointer items-center justify-between px-4 py-2.5"
          >
            <span className="text-sm text-[var(--color-fg-default)]">{opt.label}</span>
            <div className="toggle-switch">
              <input
                type="checkbox"
                checked={opt.checked}
                onChange={(e) => toggle(opt.key, e.target.checked)}
              />
              <span className="slider" />
            </div>
          </label>
        ))}
      </div>
    </div>
  );
}

function MetricCard({
  label,
  value,
  color,
}: {
  label: string;
  value: number;
  color?: string;
}) {
  return (
    <div className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)] px-4 py-3">
      <div className="text-xs text-[var(--color-fg-muted)]">{label}</div>
      <div className={`mt-1 text-xl font-semibold ${color ?? "text-[var(--color-fg-default)]"}`}>
        {value}
      </div>
    </div>
  );
}

function InfoRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex items-baseline justify-between gap-4 px-4 py-2.5">
      <span className="shrink-0 text-sm text-[var(--color-fg-muted)]">{label}</span>
      <span
        className={`truncate text-right text-sm text-[var(--color-fg-default)] ${mono ? "font-mono text-xs" : ""}`}
      >
        {value}
      </span>
    </div>
  );
}
