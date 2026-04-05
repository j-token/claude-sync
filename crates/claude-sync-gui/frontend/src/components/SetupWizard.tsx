import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AuthStatusInfo } from "../lib/types";

interface Props {
  onComplete: () => void;
  onCancel?: () => void;
}

type Step = "welcome" | "repo" | "auth" | "options" | "progress" | "done";

export default function SetupWizard({ onComplete, onCancel }: Props) {
  const [step, setStep] = useState<Step>("welcome");
  const [gitVersion, setGitVersion] = useState<string | null>(null);
  const [gitError, setGitError] = useState<string | null>(null);
  const [defaultDeviceId, setDefaultDeviceId] = useState("my-device");

  // Form state
  const [repoUrl, setRepoUrl] = useState("git@github.com:");
  const [authMethod, setAuthMethod] = useState("ssh_agent");
  const [deviceId, setDeviceId] = useState("");
  const [syncMemory, setSyncMemory] = useState(false);
  const [syncTeams, setSyncTeams] = useState(true);
  const [syncSkills, setSyncSkills] = useState(true);

  const [setupError, setSetupError] = useState<string | null>(null);
  const [setupMessage, setSetupMessage] = useState("");

  useEffect(() => {
    checkPrerequisites();
  }, []);

  async function checkPrerequisites() {
    try {
      const version = await invoke<string>("check_git");
      setGitVersion(version);
    } catch (e) {
      setGitError(`${e}`);
    }

    try {
      const id = await invoke<string>("get_default_device_id");
      setDefaultDeviceId(id);
      setDeviceId(id);
    } catch {
      // ignore
    }
  }

  async function handleSetup() {
    setStep("progress");
    setSetupError(null);
    setSetupMessage("Initializing...");

    try {
      const result = await invoke<string>("run_setup", {
        input: {
          repo_url: repoUrl,
          auth_method: authMethod,
          device_id: deviceId || defaultDeviceId,
          sync_memory: syncMemory,
          sync_teams: syncTeams,
          sync_skills: syncSkills,
        },
      });
      setSetupMessage(result);
      setStep("done");
    } catch (e) {
      setSetupError(`${e}`);
      setStep("options");
    }
  }

  return (
    <div className="mx-auto max-w-lg space-y-6">
      {/* Cancel bar */}
      {onCancel && step !== "done" && step !== "progress" && (
        <div className="flex justify-end">
          <button
            onClick={onCancel}
            className="rounded px-3 py-1 text-sm text-gray-500 hover:text-gray-300 hover:bg-gray-800 transition-colors"
          >
            Cancel
          </button>
        </div>
      )}

      {/* Progress indicator */}
      <div className="flex items-center justify-center gap-2">
        {(["welcome", "repo", "auth", "options"] as const).map((s, i) => (
          <div key={s} className="flex items-center gap-2">
            <div
              className={`h-2.5 w-2.5 rounded-full ${
                step === s
                  ? "bg-blue-500"
                  : ["welcome", "repo", "auth", "options"].indexOf(step) > i ||
                      step === "progress" ||
                      step === "done"
                    ? "bg-green-500"
                    : "bg-gray-700"
              }`}
            />
            {i < 3 && <div className="h-px w-8 bg-gray-700" />}
          </div>
        ))}
      </div>

      {/* Step: Welcome */}
      {step === "welcome" && (
        <div className="rounded-lg border border-gray-800 bg-gray-900 p-8 text-center">
          <h2 className="text-2xl font-bold">Welcome to Claude Sync</h2>
          <p className="mt-3 text-gray-400">
            Sync your Claude Code configuration across devices via GitHub.
          </p>

          <div className="mt-6 space-y-2 text-left text-sm">
            <div className="flex items-center gap-2">
              {gitVersion ? (
                <>
                  <span className="text-green-400">&#10003;</span>
                  <span className="text-gray-300">{gitVersion}</span>
                </>
              ) : gitError ? (
                <>
                  <span className="text-red-400">&#10007;</span>
                  <span className="text-red-400">Git not found. Please install git first.</span>
                </>
              ) : (
                <span className="text-gray-500">Checking git...</span>
              )}
            </div>
          </div>

          <button
            onClick={() => setStep("repo")}
            disabled={!gitVersion}
            className="mt-6 w-full rounded-lg bg-blue-600 px-4 py-3 font-medium hover:bg-blue-500 disabled:opacity-50 transition-colors"
          >
            Get Started
          </button>
        </div>
      )}

      {/* Step: Repository */}
      {step === "repo" && (
        <div className="rounded-lg border border-gray-800 bg-gray-900 p-8">
          <h2 className="text-xl font-bold">GitHub Repository</h2>
          <p className="mt-2 text-sm text-gray-400">
            Enter the URL of a private GitHub repository to store your config.
          </p>

          <label className="mt-4 block">
            <span className="text-sm text-gray-400">Repository URL</span>
            <input
              type="text"
              value={repoUrl}
              onChange={(e) => setRepoUrl(e.target.value)}
              placeholder="git@github.com:user/claude-config.git"
              className="mt-1 w-full rounded-md border border-gray-700 bg-gray-800 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
            />
          </label>

          <p className="mt-2 text-xs text-gray-500">
            Tip: Create a new private repo on GitHub, then paste the SSH or HTTPS URL here.
          </p>

          <div className="mt-6 flex gap-3">
            <button
              onClick={() => setStep("welcome")}
              className="rounded-lg border border-gray-700 px-4 py-2 text-sm hover:bg-gray-800 transition-colors"
            >
              Back
            </button>
            <button
              onClick={() => setStep("auth")}
              disabled={!repoUrl || repoUrl === "git@github.com:"}
              className="flex-1 rounded-lg bg-blue-600 px-4 py-2 font-medium hover:bg-blue-500 disabled:opacity-50 transition-colors"
            >
              Next
            </button>
          </div>
        </div>
      )}

      {/* Step: Auth & Device */}
      {step === "auth" && (
        <AuthStep
          authMethod={authMethod}
          setAuthMethod={setAuthMethod}
          deviceId={deviceId}
          setDeviceId={setDeviceId}
          defaultDeviceId={defaultDeviceId}
          repoUrl={repoUrl}
          onBack={() => setStep("repo")}
          onNext={() => setStep("options")}
        />
      )}

      {/* Step: Sync Options */}
      {step === "options" && (
        <div className="rounded-lg border border-gray-800 bg-gray-900 p-8">
          <h2 className="text-xl font-bold">Sync Options</h2>
          <p className="mt-2 text-sm text-gray-400">
            Choose what to sync. Settings, rules, commands, and agents are always synced.
          </p>

          <div className="mt-4 space-y-3">
            <label className="flex items-center gap-3 rounded-lg border border-gray-700 p-3 cursor-pointer hover:border-gray-600">
              <input
                type="checkbox"
                checked={syncSkills}
                onChange={(e) => setSyncSkills(e.target.checked)}
                className="h-4 w-4 accent-blue-500"
              />
              <div>
                <p className="font-medium">Skills</p>
                <p className="text-xs text-gray-500">All installed skills</p>
              </div>
            </label>

            <label className="flex items-center gap-3 rounded-lg border border-gray-700 p-3 cursor-pointer hover:border-gray-600">
              <input
                type="checkbox"
                checked={syncTeams}
                onChange={(e) => setSyncTeams(e.target.checked)}
                className="h-4 w-4 accent-blue-500"
              />
              <div>
                <p className="font-medium">Teams</p>
                <p className="text-xs text-gray-500">Team configurations</p>
              </div>
            </label>

            <label className="flex items-center gap-3 rounded-lg border border-gray-700 p-3 cursor-pointer hover:border-gray-600">
              <input
                type="checkbox"
                checked={syncMemory}
                onChange={(e) => setSyncMemory(e.target.checked)}
                className="h-4 w-4 accent-blue-500"
              />
              <div>
                <p className="font-medium">Memory</p>
                <p className="text-xs text-gray-500">Auto-memory files</p>
              </div>
            </label>
          </div>

          {setupError && (
            <div className="mt-4 rounded-lg border border-red-800 bg-red-950/30 p-3 text-sm text-red-400">
              {setupError}
            </div>
          )}

          <div className="mt-6 flex gap-3">
            <button
              onClick={() => setStep("auth")}
              className="rounded-lg border border-gray-700 px-4 py-2 text-sm hover:bg-gray-800 transition-colors"
            >
              Back
            </button>
            <button
              onClick={handleSetup}
              className="flex-1 rounded-lg bg-green-600 px-4 py-2 font-medium hover:bg-green-500 transition-colors"
            >
              Complete Setup
            </button>
          </div>
        </div>
      )}

      {/* Step: Progress */}
      {step === "progress" && (
        <div className="rounded-lg border border-gray-800 bg-gray-900 p-8 text-center">
          <div className="mx-auto h-8 w-8 animate-spin rounded-full border-2 border-blue-500 border-t-transparent" />
          <p className="mt-4 text-gray-400">{setupMessage}</p>
        </div>
      )}

      {/* Step: Done */}
      {step === "done" && (
        <div className="rounded-lg border border-gray-800 bg-gray-900 p-8 text-center">
          <div className="text-4xl">&#10003;</div>
          <h2 className="mt-3 text-xl font-bold text-green-400">Setup Complete!</h2>
          <p className="mt-2 text-gray-400">{setupMessage}</p>

          <button
            onClick={onComplete}
            className="mt-6 w-full rounded-lg bg-blue-600 px-4 py-3 font-medium hover:bg-blue-500 transition-colors"
          >
            Go to Dashboard
          </button>
        </div>
      )}
    </div>
  );
}

// --- Auth Step Component ---

interface AuthStepProps {
  authMethod: string;
  setAuthMethod: (v: string) => void;
  deviceId: string;
  setDeviceId: (v: string) => void;
  defaultDeviceId: string;
  repoUrl: string;
  onBack: () => void;
  onNext: () => void;
}

function AuthStep({
  authMethod,
  setAuthMethod,
  deviceId,
  setDeviceId,
  defaultDeviceId,
  repoUrl,
  onBack,
  onNext,
}: AuthStepProps) {
  const [authStatus, setAuthStatus] = useState<AuthStatusInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [token, setToken] = useState("");
  const [loginMessage, setLoginMessage] = useState<string | null>(null);
  const [loginError, setLoginError] = useState<string | null>(null);

  const isHttps = repoUrl.startsWith("https://");

  const checkAuth = useCallback(async () => {
    setLoading(true);
    try {
      const status = await invoke<AuthStatusInfo>("check_auth_status");
      setAuthStatus(status);

      // 자동 인증 방식 추천
      if (status.gh_authenticated) {
        setAuthMethod("gh_cli");
      } else if (status.ssh_key_found && !isHttps) {
        setAuthMethod("ssh_agent");
      } else if (isHttps) {
        setAuthMethod("https_token");
      }
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
      const result = await invoke<string>("login_with_gh_cli");
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
      const result = await invoke<string>("login_with_token", { token: token.trim() });
      setLoginMessage(result);
      setAuthMethod("https_token");
      checkAuth();
    } catch (e) {
      setLoginError(`${e}`);
    }
  }

  if (loading) {
    return (
      <div className="rounded-lg border border-gray-800 bg-gray-900 p-8 text-center text-gray-400">
        Checking authentication...
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-gray-800 bg-gray-900 p-8">
      <h2 className="text-xl font-bold">Authentication & Device</h2>

      {/* Auth status indicators */}
      {authStatus && (
        <div className="mt-4 space-y-2 text-sm">
          <div className="flex items-center gap-2">
            <span className={authStatus.gh_authenticated ? "text-green-400" : "text-gray-500"}>
              {authStatus.gh_authenticated ? "\u2713" : "\u2717"}
            </span>
            <span>
              gh CLI{" "}
              {authStatus.gh_authenticated && authStatus.gh_username
                ? `(${authStatus.gh_username})`
                : authStatus.gh_cli_available
                  ? "(not logged in)"
                  : "(not installed)"}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className={authStatus.ssh_key_found ? "text-green-400" : "text-gray-500"}>
              {authStatus.ssh_key_found ? "\u2713" : "\u2717"}
            </span>
            <span>
              SSH Key{" "}
              {authStatus.ssh_key_found
                ? `(${authStatus.ssh_keys.join(", ")})`
                : "(not found)"}
            </span>
          </div>
        </div>
      )}

      {/* Auth method selection */}
      <label className="mt-4 block">
        <span className="text-sm text-gray-400">Auth Method</span>
        <select
          value={authMethod}
          onChange={(e) => setAuthMethod(e.target.value)}
          className="mt-1 w-full rounded-md border border-gray-700 bg-gray-800 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
        >
          <option value="ssh_agent">SSH Agent</option>
          <option value="ssh_key">SSH Key</option>
          <option value="https_token">HTTPS (Personal Access Token)</option>
          <option value="gh_cli">gh CLI</option>
        </select>
      </label>

      {/* gh CLI login */}
      {authMethod === "gh_cli" && !authStatus?.gh_authenticated && (
        <div className="mt-3">
          <p className="text-xs text-gray-500 mb-2">
            Run <code className="rounded bg-gray-800 px-1">gh auth login</code> in terminal first, then:
          </p>
          <button
            onClick={handleGhLogin}
            className="rounded-md bg-gray-700 px-3 py-1.5 text-sm hover:bg-gray-600 transition-colors"
          >
            Detect gh CLI Login
          </button>
        </div>
      )}

      {/* PAT input */}
      {authMethod === "https_token" && (
        <div className="mt-3">
          <label className="block">
            <span className="text-sm text-gray-400">Personal Access Token</span>
            <div className="mt-1 flex gap-2">
              <input
                type="password"
                value={token}
                onChange={(e) => setToken(e.target.value)}
                placeholder="ghp_xxxxxxxxxxxx"
                className="flex-1 rounded-md border border-gray-700 bg-gray-800 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
              />
              <button
                onClick={handleTokenLogin}
                disabled={!token.trim()}
                className="rounded-md bg-blue-600 px-3 py-2 text-sm hover:bg-blue-500 disabled:opacity-50 transition-colors"
              >
                Save
              </button>
            </div>
          </label>
          <p className="mt-1 text-xs text-gray-500">
            Generate at GitHub Settings &gt; Developer settings &gt; Personal access tokens
          </p>
        </div>
      )}

      {/* Status messages */}
      {loginMessage && (
        <div className="mt-3 rounded-lg border border-green-800 bg-green-950/30 p-2 text-sm text-green-400">
          {loginMessage}
        </div>
      )}
      {loginError && (
        <div className="mt-3 rounded-lg border border-red-800 bg-red-950/30 p-2 text-sm text-red-400">
          {loginError}
        </div>
      )}

      {/* Device name */}
      <label className="mt-4 block">
        <span className="text-sm text-gray-400">Device Name</span>
        <input
          type="text"
          value={deviceId}
          onChange={(e) => setDeviceId(e.target.value)}
          placeholder={defaultDeviceId}
          className="mt-1 w-full rounded-md border border-gray-700 bg-gray-800 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
        />
      </label>

      <div className="mt-6 flex gap-3">
        <button
          onClick={onBack}
          className="rounded-lg border border-gray-700 px-4 py-2 text-sm hover:bg-gray-800 transition-colors"
        >
          Back
        </button>
        <button
          onClick={onNext}
          className="flex-1 rounded-lg bg-blue-600 px-4 py-2 font-medium hover:bg-blue-500 transition-colors"
        >
          Next
        </button>
      </div>
    </div>
  );
}
