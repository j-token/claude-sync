import { useState, useEffect } from "react";
import { invokeCommand } from "../lib/backend";
import type { SkillEntry } from "../lib/types";

export default function SkillManager() {
  const [skills, setSkills] = useState<SkillEntry[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    loadSkills();
  }, []);

  async function loadSkills() {
    setLoading(true);
    try {
      const result = await invokeCommand("list_skills");
      setSkills(result);
      setSelected(new Set(result.map((s) => s.name)));
    } catch (e) {
      setMessage(`Failed to load skills: ${e}`);
    } finally {
      setLoading(false);
    }
  }

  async function handlePush() {
    setSyncing(true);
    setMessage(null);
    try {
      const result = await invokeCommand("push_selected_skills", {
        names: Array.from(selected),
      });
      setMessage(result);
      await loadSkills();
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
      const result = await invokeCommand("pull_selected_skills", {
        names: Array.from(selected),
      });
      setMessage(result);
      await loadSkills();
    } catch (e) {
      setMessage(`Pull failed: ${e}`);
    } finally {
      setSyncing(false);
    }
  }

  function toggleSkill(name: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      next.has(name) ? next.delete(name) : next.add(name);
      return next;
    });
  }

  function toggleAll() {
    setSelected(
      selected.size === skills.length ? new Set() : new Set(skills.map((s) => s.name))
    );
  }

  function formatSize(bytes: number): string {
    if (bytes > 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
    if (bytes > 1_000) return `${(bytes / 1_000).toFixed(1)} KB`;
    return `${bytes} B`;
  }

  function statusOf(skill: SkillEntry) {
    if (skill.local_exists && skill.remote_exists) return { text: "Synced", cls: "bg-emerald-100 text-emerald-800" };
    if (skill.local_exists) return { text: "Local only", cls: "bg-amber-100 text-amber-800" };
    return { text: "Remote only", cls: "bg-blue-100 text-blue-800" };
  }

  if (loading) {
    return (
      <div className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)] p-4 text-sm text-[var(--color-fg-muted)]">
        Loading skills...
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {/* Message banner */}
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
              checked={selected.size === skills.length && skills.length > 0}
              onChange={toggleAll}
              className="h-3.5 w-3.5 rounded accent-[var(--color-accent-fg)]"
            />
            <span className="text-sm font-semibold text-[var(--color-fg-default)]">
              {selected.size > 0 ? `${selected.size} selected` : `${skills.length} skills`}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={loadSkills}
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
        {skills.length === 0 ? (
          <div className="px-4 py-8 text-center text-sm text-[var(--color-fg-muted)]">
            No skills found.
          </div>
        ) : (
          <div className="divide-y divide-[var(--color-border-muted)]">
            {skills.map((skill) => {
              const st = statusOf(skill);
              return (
                <label
                  key={skill.name}
                  className="flex cursor-pointer items-center gap-4 px-4 py-2.5 transition hover:bg-[var(--color-canvas-subtle)]"
                >
                  <input
                    type="checkbox"
                    checked={selected.has(skill.name)}
                    onChange={() => toggleSkill(skill.name)}
                    className="h-3.5 w-3.5 shrink-0 rounded accent-[var(--color-accent-fg)]"
                  />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-[var(--color-accent-fg)]">
                        {skill.name}
                      </span>
                      <span className={`rounded-full px-2 py-0.5 text-xs font-medium ${st.cls}`}>
                        {st.text}
                      </span>
                    </div>
                    <div className="mt-0.5 truncate text-xs text-[var(--color-fg-muted)]">
                      {skill.path}
                    </div>
                  </div>
                  <span className="hidden shrink-0 text-xs text-[var(--color-fg-muted)] sm:block">
                    {formatSize(skill.size_bytes)}
                  </span>
                  <span className="hidden shrink-0 text-xs text-[var(--color-fg-muted)] sm:block">
                    {skill.file_count} files
                  </span>
                </label>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
