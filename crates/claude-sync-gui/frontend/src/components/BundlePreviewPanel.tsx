import { useEffect, useMemo, useState } from "react";
import { invokeCommand } from "../lib/backend";
import { errorText, formatDate, formatSize } from "../lib/format";
import { filterByTemplate } from "../lib/glob";
import { kindLabel } from "../lib/kinds";
import { useNotify } from "../lib/notify";
import type { BundlePreview, ImportMode, ImportReport, PreviewItem, PreviewStatus, TemplateInfo } from "../lib/types";
import { Badge, Button, Checkbox, Notice, Panel, inputClass, tableCellClass, tableHeadClass, type BadgeTone } from "./ui";

const STATUS_BADGES: Record<PreviewStatus, { label: string; tone: BadgeTone }> = {
  new: { label: "새 항목", tone: "success" },
  same: { label: "동일", tone: "neutral" },
  changed: { label: "변경됨", tone: "attention" },
};

const MODE_LABELS: Record<ImportMode, string> = {
  merge: "병합",
  overwrite: "덮어쓰기",
  skip_existing: "기존 항목 건너뛰기",
};

// 실행 항목은 이 기기에서 명령을 돌리므로 기본 선택에서 빼고 사용자가 직접 고르게 한다.
function defaultSelection(items: PreviewItem[]): string[] {
  return items.filter((i) => i.status !== "same" && !i.executable).map((i) => i.id);
}

export default function BundlePreviewPanel({
  preview,
  title,
  action = "가져오기",
  importLabel = action,
  showMode = true,
  onImport,
  onClose,
}: {
  preview: BundlePreview;
  title: string;
  action?: "가져오기" | "복원";
  importLabel?: string;
  showMode?: boolean;
  onImport: (ids: string[], mode: ImportMode) => Promise<ImportReport>;
  onClose: () => void;
}) {
  const notify = useNotify();
  const [selected, setSelected] = useState<Set<string>>(() => new Set(defaultSelection(preview.items)));
  const [mode, setMode] = useState<ImportMode>("merge");
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [templateFilter, setTemplateFilter] = useState("");
  const [isImporting, setIsImporting] = useState(false);
  const [report, setReport] = useState<ImportReport | null>(null);

  useEffect(() => {
    invokeCommand("list_templates")
      .then(setTemplates)
      .catch((e) => notify("error", `템플릿 목록을 불러오지 못했습니다: ${errorText(e)}`));
  }, [notify]);

  const executableItems = preview.items.filter((i) => i.executable);
  const selectedExecutables = executableItems.filter((i) => selected.has(i.id));
  const allIds = useMemo(() => preview.items.map((i) => i.id), [preview.items]);

  function applyTemplateFilter(name: string) {
    setTemplateFilter(name);
    const base = defaultSelection(preview.items);
    const template = templates.find((t) => t.name === name);
    if (!template) {
      setSelected(new Set(base));
      return;
    }
    const matched = new Set(filterByTemplate(allIds, template.include, template.exclude));
    setSelected(new Set(base.filter((id) => matched.has(id))));
  }

  function toggle(id: string, checked: boolean) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (checked) next.add(id);
      else next.delete(id);
      return next;
    });
  }

  async function runImport() {
    setIsImporting(true);
    try {
      const ids = allIds.filter((id) => selected.has(id));
      const result = await onImport(ids, mode);
      setReport(result);
      notify("success", `${result.imported.length}개 항목을 ${action === "복원" ? "복원했습니다" : "가져왔습니다"}.`);
    } catch (e) {
      notify("error", `${action} 실패: ${errorText(e)}`);
    } finally {
      setIsImporting(false);
    }
  }

  if (report) {
    return <ImportReportPanel report={report} action={action} onClose={onClose} />;
  }

  const isAllSelected = selected.size === preview.items.length && preview.items.length > 0;

  return (
    <div className="space-y-3">
      <Panel
        title={title}
        actions={
          <Button onClick={onClose} disabled={isImporting}>
            닫기
          </Button>
        }
      >
        <dl className="m-0 grid grid-cols-2 gap-x-6 gap-y-1 px-4 py-3 text-sm sm:grid-cols-4">
          <Meta label="만든 기기" value={`${preview.device_name} (${preview.platform})`} />
          <Meta label="만든 시각" value={formatDate(preview.created_at)} />
          <Meta label="템플릿" value={preview.template ?? "-"} />
          <Meta label="크기" value={`${formatSize(preview.size_bytes)} / ${preview.items.length}개 항목`} />
          {preview.label && <Meta label="라벨" value={preview.label} />}
        </dl>
      </Panel>

      {executableItems.length > 0 && (
        <Notice tone="attention">
          <strong>실행 항목 {executableItems.length}개가 포함되어 있습니다.</strong> 훅, MCP 서버, 플러그인, statusLine은
          이 기기에서 명령을 실행합니다. 아래 명령 줄을 확인하고 믿을 수 있는 경우에만 직접 선택하세요. 기본으로는
          선택되지 않습니다.
          {selectedExecutables.length > 0 && (
            <div className="mt-1 font-semibold text-[var(--color-danger-fg)]">
              현재 실행 항목 {selectedExecutables.length}개가 선택되어 있습니다.
            </div>
          )}
        </Notice>
      )}

      <Panel
        title={`${selected.size} / ${preview.items.length}개 선택`}
        actions={
          <>
            <label className="flex items-center gap-1.5 text-xs text-[var(--color-fg-muted)]">
              템플릿 필터
              <select
                className={inputClass}
                value={templateFilter}
                onChange={(e) => applyTemplateFilter(e.target.value)}
              >
                <option value="">사용 안 함</option>
                {templates.map((t) => (
                  <option key={t.name} value={t.name}>
                    {t.name}
                  </option>
                ))}
              </select>
            </label>
            {showMode && (
              <label className="flex items-center gap-1.5 text-xs text-[var(--color-fg-muted)]">
                방식
                <select className={inputClass} value={mode} onChange={(e) => setMode(e.target.value as ImportMode)}>
                  {(Object.keys(MODE_LABELS) as ImportMode[]).map((m) => (
                    <option key={m} value={m}>
                      {MODE_LABELS[m]}
                    </option>
                  ))}
                </select>
              </label>
            )}
            <Button variant="primary" onClick={runImport} disabled={selected.size === 0 || isImporting}>
              {isImporting ? "진행 중..." : `${importLabel} (${selected.size})`}
            </Button>
          </>
        }
      >
        <div className="overflow-x-auto">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr>
                <th className={`${tableHeadClass} w-8`}>
                  <Checkbox
                    label="전체 선택"
                    checked={isAllSelected}
                    indeterminate={selected.size > 0 && !isAllSelected}
                    onChange={(checked) => setSelected(new Set(checked ? allIds : []))}
                  />
                </th>
                <th className={tableHeadClass}>항목</th>
                <th className={tableHeadClass}>종류</th>
                <th className={tableHeadClass}>상태</th>
                <th className={`${tableHeadClass} text-right`}>크기</th>
              </tr>
            </thead>
            <tbody>
              {preview.items.map((previewItem) => (
                <PreviewRow
                  key={previewItem.id}
                  item={previewItem}
                  checked={selected.has(previewItem.id)}
                  onToggle={(checked) => toggle(previewItem.id, checked)}
                />
              ))}
            </tbody>
          </table>
        </div>
      </Panel>
    </div>
  );
}

function PreviewRow({
  item,
  checked,
  onToggle,
}: {
  item: PreviewItem;
  checked: boolean;
  onToggle: (checked: boolean) => void;
}) {
  const status = STATUS_BADGES[item.status];
  return (
    <tr className={checked ? "bg-[#ddf4ff55]" : undefined}>
      <td className={tableCellClass}>
        <Checkbox label={item.id} checked={checked} onChange={onToggle} />
      </td>
      <td className={tableCellClass}>
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="font-medium">{item.name}</span>
          {item.executable && <Badge tone="danger">실행</Badge>}
          {item.masked_secret_count > 0 && <Badge tone="accent">시크릿 마스킹 {item.masked_secret_count}</Badge>}
        </div>
        <div className="font-mono text-[11px] text-[var(--color-fg-subtle)]">{item.id}</div>
        {item.description && <div className="text-xs text-[var(--color-fg-muted)]">{item.description}</div>}
        {item.executable && item.details.length > 0 && (
          <pre className="mt-1.5 mb-0 overflow-x-auto rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-inset)] px-2 py-1.5 font-mono text-[11px] leading-4">
            {item.details.join("\n")}
          </pre>
        )}
      </td>
      <td className={`${tableCellClass} whitespace-nowrap text-[var(--color-fg-muted)]`}>{kindLabel(item.kind)}</td>
      <td className={tableCellClass}>
        <Badge tone={status.tone}>{status.label}</Badge>
      </td>
      <td className={`${tableCellClass} whitespace-nowrap text-right text-[var(--color-fg-muted)]`}>
        {formatSize(item.size_bytes)}
      </td>
    </tr>
  );
}

function Meta({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <dt className="text-xs text-[var(--color-fg-muted)]">{label}</dt>
      <dd className="m-0 truncate">{value}</dd>
    </div>
  );
}

function ImportReportPanel({ report, action, onClose }: { report: ImportReport; action: string; onClose: () => void }) {
  const doneLabel = action === "복원" ? "복원한" : "가져온";
  return (
    <Panel title={`${action} 결과`} actions={<Button onClick={onClose}>닫기</Button>}>
      <div className="space-y-3 px-4 py-3 text-sm">
        {report.backup_id && (
          <div className="text-[var(--color-fg-muted)]">
            {action} 전 자동 백업: <span className="font-mono text-[var(--color-fg-default)]">{report.backup_id}</span>
          </div>
        )}
        {report.backup_skipped_reason && (
          <div className="text-[var(--color-fg-muted)]">{report.backup_skipped_reason}</div>
        )}
        <ReportList title={`${doneLabel} 항목 ${report.imported.length}개`} lines={report.imported} />
        <ReportList
          title={`건너뛴 항목 ${report.skipped.length}개`}
          lines={report.skipped.map((s) => `${s.id} — ${s.reason}`)}
        />
        {report.warnings.length > 0 && (
          <Notice tone="attention">
            <div className="font-semibold">경고</div>
            <ul className="m-0 mt-1 list-disc pl-5">
              {report.warnings.map((w) => (
                <li key={w}>{w}</li>
              ))}
            </ul>
          </Notice>
        )}
      </div>
    </Panel>
  );
}

function ReportList({ title, lines }: { title: string; lines: string[] }) {
  if (lines.length === 0) return null;
  return (
    <div>
      <div className="mb-1 font-semibold">{title}</div>
      <ul className="m-0 list-none space-y-0.5 p-0 font-mono text-xs text-[var(--color-fg-muted)]">
        {lines.map((line) => (
          <li key={line}>{line}</li>
        ))}
      </ul>
    </div>
  );
}
