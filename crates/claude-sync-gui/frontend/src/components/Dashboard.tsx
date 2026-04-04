import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SyncStatus } from "../lib/types";

interface Props {
  status: SyncStatus | null;
  onRefresh: () => void;
}

export default function Dashboard({ status, onRefresh }: Props) {
  const [pushing, setPushing] = useState(false);
  const [pulling, setPulling] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  async function handlePush() {
    setPushing(true);
    setMessage(null);
    try {
      const result = await invoke<string>("sync_push");
      setMessage(result);
      onRefresh();
    } catch (e) {
      setMessage(`Push failed: ${e}`);
    } finally {
      setPushing(false);
    }
  }

  async function handlePull() {
    setPulling(true);
    setMessage(null);
    try {
      const result = await invoke<string>("sync_pull");
      setMessage(result);
      onRefresh();
    } catch (e) {
      setMessage(`Pull failed: ${e}`);
    } finally {
      setPulling(false);
    }
  }

  if (!status) {
    return <div className="p-6 text-gray-400">Loading...</div>;
  }

  if (!status.initialized) {
    return (
      <div className="rounded-lg border border-yellow-800 bg-yellow-950/30 p-6">
        <h2 className="text-lg font-semibold text-yellow-400">Setup Required</h2>
        <p className="mt-2 text-gray-400">
          Run <code className="rounded bg-gray-800 px-2 py-0.5">claude-sync init</code> in your
          terminal to configure sync.
        </p>
      </div>
    );
  }

  const syncLabel =
    status.ahead === 0 && status.behind === 0
      ? "Up to date"
      : status.ahead > 0 && status.behind > 0
        ? `${status.ahead} ahead, ${status.behind} behind`
        : status.ahead > 0
          ? `${status.ahead} ahead`
          : `${status.behind} behind`;

  const syncColor =
    status.ahead === 0 && status.behind === 0 ? "text-green-400" : "text-yellow-400";

  return (
    <div className="space-y-4">
      {/* Status Card */}
      <div className="rounded-lg border border-gray-800 bg-gray-900 p-6">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">Sync Status</h2>
          <button
            onClick={onRefresh}
            className="rounded px-3 py-1 text-sm text-gray-400 hover:bg-gray-800 transition-colors"
          >
            Refresh
          </button>
        </div>

        <div className="mt-4 grid grid-cols-2 gap-4">
          <div>
            <span className="text-sm text-gray-500">Device</span>
            <p className="font-medium">{status.device_id}</p>
          </div>
          <div>
            <span className="text-sm text-gray-500">Status</span>
            <p className={`font-medium ${syncColor}`}>{syncLabel}</p>
          </div>
          <div>
            <span className="text-sm text-gray-500">Syncable Files</span>
            <p className="font-medium">{status.syncable_files}</p>
          </div>
          <div>
            <span className="text-sm text-gray-500">Skills</span>
            <p className="font-medium">{status.skills_count}</p>
          </div>
          <div>
            <span className="text-sm text-gray-500">Last Sync</span>
            <p className="font-medium text-sm">
              {status.last_sync ?? <span className="text-gray-500">Never</span>}
            </p>
          </div>
          <div>
            <span className="text-sm text-gray-500">Repo</span>
            <p className="font-medium text-sm truncate">{status.repo_url}</p>
          </div>
        </div>
      </div>

      {/* Actions */}
      <div className="flex gap-3">
        <button
          onClick={handlePush}
          disabled={pushing}
          className="flex-1 rounded-lg bg-green-600 px-4 py-3 font-medium hover:bg-green-500 disabled:opacity-50 transition-colors"
        >
          {pushing ? "Pushing..." : "Push"}
        </button>
        <button
          onClick={handlePull}
          disabled={pulling}
          className="flex-1 rounded-lg bg-purple-600 px-4 py-3 font-medium hover:bg-purple-500 disabled:opacity-50 transition-colors"
        >
          {pulling ? "Pulling..." : "Pull"}
        </button>
      </div>

      {/* Sync Options */}
      <div className="rounded-lg border border-gray-800 bg-gray-900 p-4">
        <h3 className="text-sm font-semibold text-gray-400 mb-2">Sync Options</h3>
        <div className="flex gap-4">
          <span className={status.sync_memory ? "text-green-400" : "text-gray-500"}>
            Memory: {status.sync_memory ? "ON" : "OFF"}
          </span>
          <span className={status.sync_teams ? "text-green-400" : "text-gray-500"}>
            Teams: {status.sync_teams ? "ON" : "OFF"}
          </span>
          <span className={status.sync_skills ? "text-green-400" : "text-gray-500"}>
            Skills: {status.sync_skills ? "ON" : "OFF"}
          </span>
        </div>
      </div>

      {/* Message */}
      {message && (
        <div
          className={`rounded-lg p-3 text-sm ${
            message.includes("fail") || message.includes("error")
              ? "border border-red-800 bg-red-950/30 text-red-400"
              : "border border-green-800 bg-green-950/30 text-green-400"
          }`}
        >
          {message}
        </div>
      )}
    </div>
  );
}
