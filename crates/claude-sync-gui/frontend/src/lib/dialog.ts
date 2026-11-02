import { isTauriRuntime } from "./backend";

const BUNDLE_FILTERS = [{ name: "Claude Sync Bundle", extensions: ["zip"] }];

export async function pickSavePath(defaultPath: string): Promise<string | null> {
  if (!isTauriRuntime()) {
    return window.prompt("저장할 ZIP 경로 (브라우저 미리보기)", `C:/Users/demo/Downloads/${defaultPath}`);
  }
  const { save } = await import("@tauri-apps/plugin-dialog");
  return save({ defaultPath, filters: BUNDLE_FILTERS });
}

export async function pickOpenPath(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return window.prompt("열 ZIP 경로 (브라우저 미리보기)", "C:/Users/demo/Downloads/claude-sync-bundle.zip");
  }
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({ multiple: false, directory: false, filters: BUNDLE_FILTERS });
  return typeof picked === "string" ? picked : null;
}

export function defaultBundleFileName(): string {
  const stamp = new Date().toISOString().slice(0, 16).replace(/[-:T]/g, "");
  return `claude-sync-${stamp}.zip`;
}
