import { useEffect, useState, type FormEvent } from "react";
import { invokeCommand } from "../lib/backend";
import { errorText, formatDate, formatSize } from "../lib/format";
import { useNotify } from "../lib/notify";
import type { BackupInfo, BundlePreview, TemplateInfo } from "../lib/types";
import BundlePreviewPanel from "./BundlePreviewPanel";
import { Badge, Button, EmptyRow, Panel, inputClass, tableCellClass, tableHeadClass } from "./ui";

export default function BackupsTab({ refreshKey, onChanged }: { refreshKey: number; onChanged: () => void }) {
  const notify = useNotify();
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [templateName, setTemplateName] = useState("");
  const [label, setLabel] = useState("");
  const [progress, setProgress] = useState<string | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [restoring, setRestoring] = useState<{ backup: BackupInfo; preview: BundlePreview } | null>(null);

  useEffect(() => {
    Promise.all([invokeCommand("list_backups"), invokeCommand("list_templates"), invokeCommand("get_config")])
      .then(([loadedBackups, loadedTemplates, config]) => {
        setBackups(loadedBackups);
        setTemplates(loadedTemplates);
        setTemplateName((current) => current || config.default_template);
      })
      .catch((e) => notify("error", `백업 목록을 불러오지 못했습니다: ${errorText(e)}`));
  }, [refreshKey, notify]);

  async function createBackup(e: FormEvent) {
    e.preventDefault();
    setProgress("백업 만드는 중...");
    try {
      const backup = await invokeCommand("create_backup", {
        ids: null,
        template: templateName || null,
        label: label.trim() || null,
      });
      notify("success", `백업을 만들었습니다: ${backup.id} (${backup.item_count}개 항목)`);
      setLabel("");
      onChanged();
    } catch (err) {
      notify("error", `백업 실패: ${errorText(err)}`);
    } finally {
      setProgress(null);
    }
  }

  async function openRestore(backup: BackupInfo) {
    setProgress("백업 여는 중...");
    try {
      const preview = await invokeCommand("inspect_bundle", { path: backup.path });
      setRestoring({ backup, preview });
    } catch (e) {
      notify("error", `백업을 열지 못했습니다: ${errorText(e)}`);
    } finally {
      setProgress(null);
    }
  }

  async function deleteBackup(id: string) {
    setProgress("삭제하는 중...");
    try {
      notify("success", await invokeCommand("delete_backup", { id }));
      setConfirmDeleteId(null);
      onChanged();
    } catch (e) {
      notify("error", `삭제 실패: ${errorText(e)}`);
    } finally {
      setProgress(null);
    }
  }

  async function restoreSelected(id: string, ids: string[]) {
    const report = await invokeCommand("restore_backup", { id, ids });
    onChanged();
    return report;
  }

  if (restoring) {
    const { backup, preview } = restoring;
    return (
      <BundlePreviewPanel
        key={backup.id}
        preview={preview}
        title={`백업 미리보기 · ${backup.id}`}
        action="복원"
        importLabel="선택 복원"
        showMode={false}
        onImport={(ids) => restoreSelected(backup.id, ids)}
        onClose={() => setRestoring(null)}
      />
    );
  }

  const isBusy = progress !== null;

  return (
    <div className="space-y-3">
      <Panel title="지금 백업">
        <form onSubmit={createBackup} className="flex flex-wrap items-center gap-2 px-4 py-3">
          <label className="flex items-center gap-1.5 text-xs text-[var(--color-fg-muted)]">
            템플릿
            <select className={inputClass} value={templateName} onChange={(e) => setTemplateName(e.target.value)}>
              {templates.map((t) => (
                <option key={t.name} value={t.name}>
                  {t.name}
                </option>
              ))}
            </select>
          </label>
          <input
            className={`${inputClass} min-w-[200px] flex-1`}
            placeholder="라벨 (선택)"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
          />
          <Button type="submit" variant="primary" disabled={isBusy}>
            지금 백업
          </Button>
          {progress && <span className="text-xs text-[var(--color-fg-muted)]">{progress}</span>}
        </form>
      </Panel>

      <Panel title={`백업 ${backups.length}개`}>
        {backups.length === 0 ? (
          <EmptyRow>아직 백업이 없습니다.</EmptyRow>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-sm">
              <thead>
                <tr>
                  <th className={tableHeadClass}>종류</th>
                  <th className={tableHeadClass}>라벨</th>
                  <th className={tableHeadClass}>만든 시각</th>
                  <th className={`${tableHeadClass} text-right`}>항목</th>
                  <th className={`${tableHeadClass} text-right`}>크기</th>
                  <th className={`${tableHeadClass} text-right`}>작업</th>
                </tr>
              </thead>
              <tbody>
                {backups.map((backup) => (
                  <tr key={backup.id}>
                    <td className={tableCellClass}>
                      <Badge tone={backup.kind === "auto" ? "neutral" : "accent"}>
                        {backup.kind === "auto" ? "자동" : "수동"}
                      </Badge>
                    </td>
                    <td className={tableCellClass}>
                      <div>{backup.label ?? <span className="text-[var(--color-fg-subtle)]">라벨 없음</span>}</div>
                      <div className="font-mono text-[11px] text-[var(--color-fg-subtle)]">{backup.id}</div>
                    </td>
                    <td className={`${tableCellClass} whitespace-nowrap`}>{formatDate(backup.created_at)}</td>
                    <td className={`${tableCellClass} text-right`}>{backup.item_count}</td>
                    <td className={`${tableCellClass} whitespace-nowrap text-right`}>{formatSize(backup.size_bytes)}</td>
                    <td className={`${tableCellClass} text-right`}>
                      <div className="flex justify-end gap-1.5">
                        {confirmDeleteId === backup.id ? (
                          <>
                            <Button variant="danger" onClick={() => deleteBackup(backup.id)} disabled={isBusy}>
                              정말 삭제
                            </Button>
                            <Button onClick={() => setConfirmDeleteId(null)} disabled={isBusy}>
                              취소
                            </Button>
                          </>
                        ) : (
                          <>
                            <Button onClick={() => openRestore(backup)} disabled={isBusy}>
                              미리보기/선택 복원
                            </Button>
                            <Button variant="danger" onClick={() => setConfirmDeleteId(backup.id)} disabled={isBusy}>
                              삭제
                            </Button>
                          </>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Panel>
    </div>
  );
}
