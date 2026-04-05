import { useState, useEffect, useCallback } from "react";
import { invokeCommand } from "../lib/backend";
import type { AuthStatusInfo, SyncStatus } from "../lib/types";

interface Props {
  onComplete: () => void;
  onCancel?: () => void;
  /** 현재 SyncStatus — 이미 초기화된 상태에서 설정 편집 시 전달 */
  status?: SyncStatus | null;
}

export default function Settings({ onComplete, onCancel, status }: Props) {
  const isInitialized = !!status?.initialized;

  /* ---- Prerequisites ---- */
  const [gitVersion, setGitVersion] = useState<string | null>(null);
  const [gitError, setGitError] = useState<string | null>(null);
  const [defaultDeviceId, setDefaultDeviceId] = useState("my-device");

  /* ---- Repository ---- */
  const [repoUrl, setRepoUrl] = useState(status?.repo_url || "git@github.com:");

  /* ---- Authentication ---- */
  const [authMethod, setAuthMethod] = useState("ssh_agent");
  const [deviceId, setDeviceId] = useState(status?.device_id || "");

  /* ---- Sync Options ---- */
  const [syncMemory, setSyncMemory] = useState(status?.sync_memory ?? false);
  const [syncTeams, setSyncTeams] = useState(status?.sync_teams ?? true);
  const [syncSkills, setSyncSkills] = useState(status?.sync_skills ?? true);
  const [syncPlugins, setSyncPlugins] = useState(status?.sync_plugins ?? true);

  /* ---- UI State ---- */
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    checkPrerequisites();
  }, []);

  async function checkPrerequisites() {
    try {
      const version = await invokeCommand("check_git");
      setGitVersion(version);
    } catch (e) {
      setGitError(`${e}`);
    }
    try {
      const id = await invokeCommand("get_default_device_id");
      setDefaultDeviceId(id);
      if (!deviceId) setDeviceId(id);
    } catch {
      // ignore
    }
  }

  /** 초기 셋업 실행 */
  async function handleSetup() {
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      const result = await invokeCommand("run_setup", {
        input: {
          repo_url: repoUrl,
          auth_method: authMethod,
          device_id: deviceId || defaultDeviceId,
          sync_memory: syncMemory,
          sync_teams: syncTeams,
          sync_skills: syncSkills,
          sync_plugins: syncPlugins,
        },
      });
      setMessage(result);
      setTimeout(() => onComplete(), 800);
    } catch (e) {
      setError(`${e}`);
    } finally {
      setSaving(false);
    }
  }

  /** Sync 옵션만 즉시 업데이트 (이미 초기화된 상태) */
  async function handleUpdateSyncOptions(
    memory: boolean,
    teams: boolean,
    skills: boolean,
    plugins: boolean,
  ) {
    try {
      await invokeCommand("update_sync_options", {
        input: {
          sync_memory: memory,
          sync_teams: teams,
          sync_skills: skills,
          sync_plugins: plugins,
        },
      });
    } catch (e) {
      setError(`${e}`);
    }
  }

  /** 스위치 토글 핸들러 — 상태 업데이트 + 초기화 완료 시 즉시 저장 */
  function toggleOption(
    key: "memory" | "teams" | "skills" | "plugins",
    value: boolean,
  ) {
    const next = {
      memory: key === "memory" ? value : syncMemory,
      teams: key === "teams" ? value : syncTeams,
      skills: key === "skills" ? value : syncSkills,
      plugins: key === "plugins" ? value : syncPlugins,
    };
    setSyncMemory(next.memory);
    setSyncTeams(next.teams);
    setSyncSkills(next.skills);
    setSyncPlugins(next.plugins);

    if (isInitialized) {
      handleUpdateSyncOptions(next.memory, next.teams, next.skills, next.plugins);
    }
  }

  const canInitialize =
    gitVersion && repoUrl && repoUrl !== "git@github.com:";

  return (
    <div className="mx-auto max-w-2xl space-y-5">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-[var(--color-fg-default)]">
          Settings
        </h2>
        {onCancel && (
          <button
            onClick={onCancel}
            className="rounded-md border border-[var(--color-btn-border)] bg-[var(--color-btn-bg)] px-3 py-1.5 text-xs font-medium text-[var(--color-fg-muted)] hover:bg-[var(--color-btn-hover-bg)] hover:text-[var(--color-fg-default)]"
          >
            Close
          </button>
        )}
      </div>

      {/* Prerequisites */}
      <Section title="Prerequisites" description="Git 설치 상태를 확인합니다.">
        <div className="grid gap-3 sm:grid-cols-2">
          <StatusCard
            label="Git"
            value={gitVersion ?? "Checking..."}
            ok={!!gitVersion}
            error={gitError}
          />
          <StatusCard
            label="Device"
            value={deviceId || defaultDeviceId}
            ok
          />
        </div>
      </Section>

      {/* Repository */}
      <Section title="Repository" description="동기화할 GitHub 저장소 URL을 입력합니다.">
        <label className="block">
          <span className="text-sm font-medium text-[var(--color-fg-default)]">URL</span>
          <input
            type="text"
            value={repoUrl}
            onChange={(e) => setRepoUrl(e.target.value)}
            placeholder="git@github.com:user/claude-config.git"
            disabled={isInitialized}
            className="mt-1.5 w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)] px-3 py-2 text-sm disabled:opacity-60"
          />
        </label>
        {isInitialized && (
          <p className="mt-1.5 text-xs text-[var(--color-fg-subtle)]">
            저장소 URL은 초기 설정 이후 변경할 수 없습니다.
          </p>
        )}
      </Section>

      {/* Authentication */}
      <AuthSection
        authMethod={authMethod}
        setAuthMethod={setAuthMethod}
        deviceId={deviceId}
        setDeviceId={setDeviceId}
        defaultDeviceId={defaultDeviceId}
        repoUrl={repoUrl}
        isInitialized={isInitialized}
      />

      {/* Sync Options */}
      <Section title="Sync Options" description="동기화할 데이터 종류를 선택합니다.">
        <div className="divide-y divide-[var(--color-border-muted)] rounded-md border border-[var(--color-border-default)]">
          <ToggleRow
            label="Skills"
            detail="스킬 폴더 동기화"
            checked={syncSkills}
            onChange={(v) => toggleOption("skills", v)}
          />
          <ToggleRow
            label="Teams"
            detail="팀 설정 동기화"
            checked={syncTeams}
            onChange={(v) => toggleOption("teams", v)}
          />
          <ToggleRow
            label="Memory"
            detail="자동 메모리 파일 동기화"
            checked={syncMemory}
            onChange={(v) => toggleOption("memory", v)}
          />
          <ToggleRow
            label="Plugins"
            detail="플러그인 메타데이터 동기화"
            checked={syncPlugins}
            onChange={(v) => toggleOption("plugins", v)}
          />
        </div>
      </Section>

      {/* Messages */}
      {error && (
        <div className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">
          {error}
        </div>
      )}
      {message && (
        <div className="rounded-md border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-800">
          {message}
        </div>
      )}

      {/* Actions */}
      {!isInitialized && (
        <div className="flex justify-end gap-2 border-t border-[var(--color-border-muted)] pt-4">
          {onCancel && (
            <button
              onClick={onCancel}
              className="rounded-md border border-[var(--color-btn-border)] bg-[var(--color-btn-bg)] px-4 py-2 text-sm font-medium text-[var(--color-fg-default)] hover:bg-[var(--color-btn-hover-bg)]"
            >
              Cancel
            </button>
          )}
          <button
            onClick={handleSetup}
            disabled={!canInitialize || saving}
            className="rounded-md bg-[var(--color-btn-primary-bg)] px-4 py-2 text-sm font-medium text-white hover:bg-[var(--color-btn-primary-hover-bg)] disabled:opacity-50"
          >
            {saving ? "Initializing..." : "Initialize"}
          </button>
        </div>
      )}
    </div>
  );
}

/* ================================================================
   Sub-components
   ================================================================ */

/** 섹션 카드 래퍼 */
function Section({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <div className="overflow-hidden rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)]">
      <div className="border-b border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)] px-4 py-2.5">
        <span className="text-sm font-semibold text-[var(--color-fg-default)]">{title}</span>
        <p className="mt-0.5 text-xs text-[var(--color-fg-muted)]">{description}</p>
      </div>
      <div className="p-4">{children}</div>
    </div>
  );
}

/** 인증 섹션 */
function AuthSection({
  authMethod,
  setAuthMethod,
  deviceId,
  setDeviceId,
  defaultDeviceId,
  repoUrl,
  isInitialized,
}: {
  authMethod: string;
  setAuthMethod: (v: string) => void;
  deviceId: string;
  setDeviceId: (v: string) => void;
  defaultDeviceId: string;
  repoUrl: string;
  isInitialized: boolean;
}) {
  const [authStatus, setAuthStatus] = useState<AuthStatusInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [token, setToken] = useState("");
  const [loginMessage, setLoginMessage] = useState<string | null>(null);
  const [loginError, setLoginError] = useState<string | null>(null);

  const isHttps = repoUrl.startsWith("https://");

  const checkAuth = useCallback(async () => {
    setLoading(true);
    try {
      const status = await invokeCommand("check_auth_status");
      setAuthStatus(status);
      if (status.gh_authenticated) setAuthMethod("gh_cli");
      else if (status.ssh_key_found && !isHttps) setAuthMethod("ssh_agent");
      else if (isHttps) setAuthMethod("https_token");
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, [isHttps, setAuthMethod]);

  useEffect(() => {
    checkAuth();
  }, [checkAuth]);

  async function handleGhLogin() {
    setLoginError(null);
    setLoginMessage(null);
    try {
      const result = await invokeCommand("login_with_gh_cli");
      setLoginMessage(result);
      checkAuth();
    } catch (e) {
      setLoginError(`${e}`);
    }
  }

  async function handleTokenLogin() {
    if (!token.trim()) return;
    setLoginError(null);
    setLoginMessage(null);
    try {
      const result = await invokeCommand("login_with_token", { token: token.trim() });
      setLoginMessage(result);
      setAuthMethod("https_token");
      checkAuth();
    } catch (e) {
      setLoginError(`${e}`);
    }
  }

  return (
    <Section title="Authentication" description="저장소 인증 방식과 디바이스 이름을 설정합니다.">
      {loading ? (
        <p className="text-sm text-[var(--color-fg-muted)]">Checking authentication...</p>
      ) : (
        <>
          {authStatus && (
            <div className="mb-4 grid gap-3 sm:grid-cols-2">
              <StatusCard
                label="GitHub CLI"
                value={
                  authStatus.gh_authenticated && authStatus.gh_username
                    ? authStatus.gh_username
                    : authStatus.gh_cli_available
                      ? "Not logged in"
                      : "Not installed"
                }
                ok={authStatus.gh_authenticated}
              />
              <StatusCard
                label="SSH keys"
                value={authStatus.ssh_key_found ? authStatus.ssh_keys.join(", ") : "None found"}
                ok={authStatus.ssh_key_found}
              />
            </div>
          )}

          <label className="block">
            <span className="text-sm font-medium text-[var(--color-fg-default)]">Method</span>
            <select
              value={authMethod}
              onChange={(e) => setAuthMethod(e.target.value)}
              disabled={isInitialized}
              className="mt-1.5 w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)] px-3 py-2 text-sm disabled:opacity-60"
            >
              <option value="ssh_agent">SSH Agent</option>
              <option value="ssh_key">SSH Key</option>
              <option value="https_token">HTTPS Token</option>
              <option value="gh_cli">GitHub CLI</option>
            </select>
          </label>

          {authMethod === "gh_cli" && !authStatus?.gh_authenticated && (
            <div className="mt-4 rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)] p-3">
              <p className="text-sm text-[var(--color-fg-muted)]">
                Run <code className="rounded bg-[var(--color-canvas-default)] px-1 py-0.5 text-xs">gh auth login</code> first.
              </p>
              <button
                onClick={handleGhLogin}
                className="mt-2 rounded-md border border-[var(--color-btn-border)] bg-[var(--color-btn-bg)] px-3 py-1.5 text-xs font-medium text-[var(--color-fg-default)] hover:bg-[var(--color-btn-hover-bg)]"
              >
                Detect login
              </button>
            </div>
          )}

          {authMethod === "https_token" && (
            <div className="mt-4 rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)] p-3">
              <label className="block">
                <span className="text-sm font-medium text-[var(--color-fg-default)]">Token</span>
                <div className="mt-1.5 flex gap-2">
                  <input
                    type="password"
                    value={token}
                    onChange={(e) => setToken(e.target.value)}
                    placeholder="ghp_xxxxxxxxxxxx"
                    className="flex-1 rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)] px-3 py-2 text-sm"
                  />
                  <button
                    onClick={handleTokenLogin}
                    disabled={!token.trim()}
                    className="rounded-md bg-[var(--color-btn-primary-bg)] px-3 py-2 text-sm font-medium text-white hover:bg-[var(--color-btn-primary-hover-bg)] disabled:opacity-50"
                  >
                    Save
                  </button>
                </div>
              </label>
            </div>
          )}

          {loginMessage && (
            <div className="mt-3 rounded-md border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-800">
              {loginMessage}
            </div>
          )}
          {loginError && (
            <div className="mt-3 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">
              {loginError}
            </div>
          )}

          <label className="mt-4 block">
            <span className="text-sm font-medium text-[var(--color-fg-default)]">Device name</span>
            <input
              type="text"
              value={deviceId}
              onChange={(e) => setDeviceId(e.target.value)}
              placeholder={defaultDeviceId}
              disabled={isInitialized}
              className="mt-1.5 w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)] px-3 py-2 text-sm disabled:opacity-60"
            />
          </label>
        </>
      )}
    </Section>
  );
}

/** 상태 카드 */
function StatusCard({
  label,
  value,
  ok,
  error,
}: {
  label: string;
  value: string;
  ok: boolean;
  error?: string | null;
}) {
  return (
    <div
      className={`rounded-md border p-3 ${
        error
          ? "border-red-200 bg-red-50"
          : ok
            ? "border-emerald-200 bg-emerald-50"
            : "border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)]"
      }`}
    >
      <div className="text-xs font-medium text-[var(--color-fg-muted)]">{label}</div>
      <div className="mt-1 text-sm font-medium text-[var(--color-fg-default)]">{value}</div>
      {error && <div className="mt-1 text-xs text-red-700">{error}</div>}
    </div>
  );
}

/** Switch 토글 행 */
function ToggleRow({
  label,
  detail,
  checked,
  onChange,
}: {
  label: string;
  detail: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex cursor-pointer items-center justify-between px-4 py-3">
      <div>
        <div className="text-sm font-medium text-[var(--color-fg-default)]">{label}</div>
        <div className="text-xs text-[var(--color-fg-muted)]">{detail}</div>
      </div>
      <div className="toggle-switch">
        <input
          type="checkbox"
          checked={checked}
          onChange={(e) => onChange(e.target.checked)}
        />
        <span className="slider" />
      </div>
    </label>
  );
}
