import { useState } from "react";
import { invokeCommand } from "../lib/backend";
import { pickOpenPath } from "../lib/dialog";
import { errorText } from "../lib/format";
import { useNotify } from "../lib/notify";
import type { BundlePreview, ImportMode } from "../lib/types";
import BundlePreviewPanel from "./BundlePreviewPanel";
import { Button, Panel } from "./ui";

export default function ImportTab({ onChanged }: { onChanged: () => void }) {
  const notify = useNotify();
  const [preview, setPreview] = useState<BundlePreview | null>(null);
  const [isOpening, setIsOpening] = useState(false);

  async function openBundle() {
    const path = await pickOpenPath();
    if (!path) return;
    setIsOpening(true);
    try {
      setPreview(await invokeCommand("inspect_bundle", { path }));
    } catch (e) {
      notify("error", `ZIP 파일을 열지 못했습니다: ${errorText(e)}`);
    } finally {
      setIsOpening(false);
    }
  }

  async function importSelected(path: string, ids: string[], mode: ImportMode) {
    const report = await invokeCommand("import_bundle", { path, ids, mode });
    onChanged();
    return report;
  }

  if (preview) {
    return (
      <BundlePreviewPanel
        key={preview.path}
        preview={preview}
        title={`번들 미리보기 · ${preview.path}`}
        onImport={(ids, mode) => importSelected(preview.path, ids, mode)}
        onClose={() => setPreview(null)}
      />
    );
  }

  return (
    <Panel title="ZIP 번들 가져오기">
      <div className="space-y-3 px-4 py-4 text-sm">
        <p className="m-0 text-[var(--color-fg-muted)]">
          다른 기기에서 내보낸 Claude Sync 번들(.zip)을 열면, 가져오기 전에 항목별로 이 기기와 비교한 결과를 보여줍니다.
          가져올 항목과 방식을 고른 뒤 적용하세요. 설정에서 켜 두면 가져오기 전에 자동으로 백업을 만듭니다.
        </p>
        <Button variant="primary" onClick={openBundle} disabled={isOpening}>
          {isOpening ? "여는 중..." : "ZIP 파일 열기"}
        </Button>
      </div>
    </Panel>
  );
}
