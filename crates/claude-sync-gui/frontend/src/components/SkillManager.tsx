import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SkillEntry } from "../lib/types";

export default function SkillManager() {
  const [skills, setSkills] = useState<SkillEntry[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    loadSkills();
  }, []);

  async function loadSkills() {
    setLoading(true);
    try {
      const result = await invoke<SkillEntry[]>("list_skills");
      setSkills(result);
      // 기본적으로 모두 선택
      setSelected(new Set(result.map((s) => s.name)));
    } catch (e) {
      setMessage(`Failed to load skills: ${e}`);
    } finally {
      setLoading(false);
    }
  }

  function toggleSkill(name: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  }

  function toggleAll() {
    if (selected.size === skills.length) {
      setSelected(new Set());
    } else {
      setSelected(new Set(skills.map((s) => s.name)));
    }
  }

  function formatSize(bytes: number): string {
    if (bytes > 1_000_000) return `${(bytes / 1_000_000).toFixed(1)}MB`;
    if (bytes > 1_000) return `${(bytes / 1_000).toFixed(1)}KB`;
    return `${bytes}B`;
  }

  function getStatusBadge(skill: SkillEntry) {
    if (skill.local_exists && skill.remote_exists) {
      return <span className="rounded bg-green-900 px-2 py-0.5 text-xs text-green-300">Synced</span>;
    }
    if (skill.local_exists) {
      return <span className="rounded bg-yellow-900 px-2 py-0.5 text-xs text-yellow-300">Local only</span>;
    }
    return <span className="rounded bg-blue-900 px-2 py-0.5 text-xs text-blue-300">Remote only</span>;
  }

  if (loading) {
    return <div className="p-6 text-gray-400">Loading skills...</div>;
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Skill Manager</h2>
        <div className="flex gap-2">
          <button
            onClick={toggleAll}
            className="rounded px-3 py-1 text-sm text-gray-400 hover:bg-gray-800 transition-colors"
          >
            {selected.size === skills.length ? "Deselect All" : "Select All"}
          </button>
          <button
            onClick={loadSkills}
            className="rounded px-3 py-1 text-sm text-gray-400 hover:bg-gray-800 transition-colors"
          >
            Refresh
          </button>
        </div>
      </div>

      {skills.length === 0 ? (
        <p className="text-gray-500">No skills found</p>
      ) : (
        <div className="space-y-1">
          {skills.map((skill) => (
            <label
              key={skill.name}
              className="flex items-center gap-3 rounded-lg border border-gray-800 bg-gray-900 p-3 cursor-pointer hover:border-gray-700 transition-colors"
            >
              <input
                type="checkbox"
                checked={selected.has(skill.name)}
                onChange={() => toggleSkill(skill.name)}
                className="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-blue-500"
              />
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium">{skill.name}</span>
                  {getStatusBadge(skill)}
                </div>
                <span className="text-sm text-gray-500">
                  {formatSize(skill.size_bytes)} / {skill.file_count} files
                </span>
              </div>
            </label>
          ))}
        </div>
      )}

      {/* Actions for selected skills */}
      <div className="flex gap-3">
        <button
          disabled={selected.size === 0}
          className="flex-1 rounded-lg bg-green-600 px-4 py-2 text-sm font-medium hover:bg-green-500 disabled:opacity-50 transition-colors"
        >
          Push Selected ({selected.size})
        </button>
        <button
          disabled={selected.size === 0}
          className="flex-1 rounded-lg bg-purple-600 px-4 py-2 text-sm font-medium hover:bg-purple-500 disabled:opacity-50 transition-colors"
        >
          Pull Selected ({selected.size})
        </button>
      </div>

      {message && (
        <div className="rounded-lg border border-red-800 bg-red-950/30 p-3 text-sm text-red-400">
          {message}
        </div>
      )}
    </div>
  );
}
