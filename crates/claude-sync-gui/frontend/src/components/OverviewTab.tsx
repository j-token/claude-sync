import { useState } from "react";
import type { TabId } from "../App";
import { invokeCommand } from "../lib/backend";
import { defaultBundleFileName, pickSavePath } from "../lib/dialog";
import { errorText, formatDate, formatSize } from "../lib/format";
import { useNotify } from "../lib/notify";
import type { Overview } from "../lib/types";
import { Button, EmptyRow, Panel } from "./ui";

export default function OverviewTab({
  overview,
  onNavigate,
  onChanged,
}: {
  overview: Overview | null;
  onNavigate: (tab: TabId) => void;
  onChanged: () => void;
}) {
  const notify = useNotify();
  const [progress, setProgress] = useState<string | null>(null);

  if (!overview) {
    return (
      <Panel>
        <EmptyRow>불러오는 중...</EmptyRow>
      </Panel>
    );
  }

  const defaultTemplate = overview.default_template;

  async function backupNow() {
    setProgress("백업 만드는 중...");
    try {
      const backup = await invokeCommand("create_backup", { ids: null, template: defaultTemplate, label: null });
      notify("success", `백업을 만들었습니다: ${backup.id} (${backup.item_count}개 항목)`);
      onChanged();
    } catch (e) {
      notify("error", `백업 실패: ${errorText(e)}`);
    } finally {
      setProgress(null);
    }
  }

  async function exportDefault() {
    const path = await pickSavePath(defaultBundleFileName());
    if (!path) return;
    setProgress("내보내는 중...");
    try {
      const ids = await invokeCommand("resolve_template", { name: defaultTemplate });
      const result = await invokeCommand("export_bundle", { ids, path, label: null, template: defaultTemplate });
      notify(
        "success",
        `${result.item_count}개 항목(${formatSize(result.size_bytes)})을 내보냈습니다: ${result.path}` +
          (result.masked_secret_count > 0 ? ` · 시크릿 ${result.masked_secret_count}개 마스킹` : ""),
      );
    } catch (e) {
      notify("error", `내보내기 실패: ${errorText(e)}`);
    } finally {
      setProgress(null);
    }
  }

  const isBusy = progress !== null;

  return (
    <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
      <div className="space-y-4">
        <Panel title="이 기기">
          <dl className="m-0 divide-y divide-[var(--color-border-muted)] text-sm">
            <InfoRow label="기기 이름" value={overview.device_name} />
            <InfoRow label="플랫폼" value={overview.platform} />
            <InfoRow label="Claude 폴더" value={overview.claude_dir} mono />
            <InfoRow label="데이터 폴더" value={overview.sync_home} mono />
            <InfoRow label="기본 템플릿" value={defaultTemplate} />
          </dl>
        </Panel>

        <Panel
          title={`항목 ${overview.total_items}개`}
          actions={<Button onClick={() => onNavigate("items")}>항목 보기</Button>}
        >
          {overview.kinds.length === 0 ? (
            <EmptyRow>찾은 항목이 없습니다.</EmptyRow>
          ) : (
            <ul className="m-0 grid list-none grid-cols-2 gap-px bg-[var(--color-border-muted)] p-0 sm:grid-cols-3 lg:grid-cols-4">
              {overview.kinds.map((kind) => (
                <li key={kind.kind} className="flex items-center justify-between bg-[var(--color-canvas-default)] px-4 py-2.5">
                  <span className="text-sm text-[var(--color-fg-muted)]">{kind.label}</span>
                  <span className="text-sm font-semibold">{kind.count}</span>
                </li>
              ))}
            </ul>
          )}
        </Panel>
      </div>

      <div className="space-y-4">
        <Panel title="빠른 작업">
          <div className="grid gap-2 p-4">
            <Button variant="primary" onClick={backupNow} disabled={isBusy}>
              지금 백업
            </Button>
            <Button onClick={exportDefault} disabled={isBusy}>
              내보내기 ({defaultTemplate})
            </Button>
            <Button onClick={() => onNavigate("import")} disabled={isBusy}>
              가져오기
            </Button>
            <Button onClick={() => onNavigate("p2p")} disabled={isBusy}>
              P2P 공유
            </Button>
            {progress && <div className="text-xs text-[var(--color-fg-muted)]">{progress}</div>}
          </div>
        </Panel>

        <Panel title="백업" actions={<Button onClick={() => onNavigate("backups")}>관리</Button>}>
          <dl className="m-0 divide-y divide-[var(--color-border-muted)] text-sm">
            <InfoRow label="백업 수" value={`${overview.backup_count}개`} />
            <InfoRow label="마지막 백업" value={overview.last_backup ? formatDate(overview.last_backup) : "없음"} />
            <InfoRow label="템플릿 수" value={`${overview.template_count}개`} />
          </dl>
        </Panel>
      </div>
    </div>
  );
}

function InfoRow({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex items-baseline gap-4 px-4 py-2">
      <dt className="w-28 shrink-0 text-[var(--color-fg-muted)]">{label}</dt>
      <dd className={`m-0 min-w-0 break-all ${mono ? "font-mono text-xs" : ""}`}>{value}</dd>
    </div>
  );
}
