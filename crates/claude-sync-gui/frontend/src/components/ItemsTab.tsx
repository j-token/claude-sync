import { useEffect, useMemo, useState, type FormEvent } from "react";
import { invokeCommand } from "../lib/backend";
import { defaultBundleFileName, pickSavePath } from "../lib/dialog";
import { errorText, formatSize } from "../lib/format";
import { KIND_ORDER, kindLabel } from "../lib/kinds";
import { useNotify } from "../lib/notify";
import type { ItemKind, ItemSummary, TemplateInfo } from "../lib/types";
import { Badge, Button, Checkbox, EmptyRow, Panel, inputClass } from "./ui";

type OpenForm = "backup" | "template" | null;

export default function ItemsTab({
  refreshKey,
  onChanged,
  onShare,
}: {
  refreshKey: number;
  onChanged: () => void;
  onShare: (ids: string[]) => Promise<void>;
}) {
  const notify = useNotify();
  const [items, setItems] = useState<ItemSummary[]>([]);
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [appliedTemplate, setAppliedTemplate] = useState<string | null>(null);
  const [kindFilter, setKindFilter] = useState<ItemKind | "all">("all");
  const [search, setSearch] = useState("");
  const [progress, setProgress] = useState<string | null>(null);
  const [openForm, setOpenForm] = useState<OpenForm>(null);

  useEffect(() => {
    let isCurrent = true;
    setIsLoading(true);
    Promise.all([invokeCommand("list_items"), invokeCommand("list_templates")])
      .then(([loadedItems, loadedTemplates]) => {
        if (!isCurrent) return;
        setItems(loadedItems);
        setTemplates(loadedTemplates);
        const existingIds = new Set(loadedItems.map((i) => i.id));
        setSelected((prev) => new Set([...prev].filter((id) => existingIds.has(id))));
      })
      .catch((e) => notify("error", `항목을 불러오지 못했습니다: ${errorText(e)}`))
      .finally(() => isCurrent && setIsLoading(false));
    return () => {
      isCurrent = false;
    };
  }, [refreshKey, notify]);

  const kindCounts = useMemo(() => {
    const counts = new Map<ItemKind, number>();
    for (const i of items) counts.set(i.kind, (counts.get(i.kind) ?? 0) + 1);
    return counts;
  }, [items]);

  const visibleGroups = useMemo(() => {
    const query = search.trim().toLowerCase();
    const matchesQuery = (i: ItemSummary) =>
      query === "" ||
      i.id.toLowerCase().includes(query) ||
      i.name.toLowerCase().includes(query) ||
      (i.description ?? "").toLowerCase().includes(query);
    return KIND_ORDER.filter((kind) => kindFilter === "all" || kind === kindFilter)
      .map((kind) => ({ kind, items: items.filter((i) => i.kind === kind && matchesQuery(i)) }))
      .filter((group) => group.items.length > 0);
  }, [items, kindFilter, search]);

  const selectedIds = items.filter((i) => selected.has(i.id)).map((i) => i.id);
  const visibleIds = visibleGroups.flatMap((g) => g.items.map((i) => i.id));
  const isBusy = progress !== null;

  function setChecked(ids: string[], checked: boolean) {
    setAppliedTemplate(null);
    setSelected((prev) => {
      const next = new Set(prev);
      for (const id of ids) {
        if (checked) next.add(id);
        else next.delete(id);
      }
      return next;
    });
  }

  async function applyTemplate(name: string) {
    if (!name) {
      setAppliedTemplate(null);
      return;
    }
    try {
      const ids = await invokeCommand("resolve_template", { name });
      setSelected(new Set(ids));
      setAppliedTemplate(name);
    } catch (e) {
      notify("error", `템플릿 적용 실패: ${errorText(e)}`);
    }
  }

  async function exportSelection() {
    const path = await pickSavePath(defaultBundleFileName());
    if (!path) return;
    setProgress("ZIP으로 내보내는 중...");
    try {
      const result = await invokeCommand("export_bundle", {
        ids: selectedIds,
        path,
        label: null,
        template: appliedTemplate,
      });
      const maskedNote = result.masked_secret_count > 0 ? ` · 시크릿 ${result.masked_secret_count}개 마스킹` : "";
      notify(
        "success",
        `${result.item_count}개 항목, 파일 ${result.file_count}개(${formatSize(result.size_bytes)})를 내보냈습니다: ${result.path}${maskedNote}`,
      );
    } catch (e) {
      notify("error", `내보내기 실패: ${errorText(e)}`);
    } finally {
      setProgress(null);
    }
  }

  async function createBackup(label: string) {
    setProgress("백업 만드는 중...");
    try {
      const backup = await invokeCommand("create_backup", { ids: selectedIds, template: null, label: label || null });
      notify("success", `백업을 만들었습니다: ${backup.id} (${backup.item_count}개 항목)`);
      setOpenForm(null);
      onChanged();
    } catch (e) {
      notify("error", `백업 실패: ${errorText(e)}`);
    } finally {
      setProgress(null);
    }
  }

  async function saveAsTemplate(name: string, description: string) {
    setProgress("템플릿 저장 중...");
    try {
      const saved = await invokeCommand("save_template", { name, description, include: selectedIds, exclude: [] });
      notify("success", `템플릿 '${saved.name}'을(를) 저장했습니다.`);
      setOpenForm(null);
      setAppliedTemplate(saved.name);
      onChanged();
    } catch (e) {
      notify("error", `템플릿 저장 실패: ${errorText(e)}`);
    } finally {
      setProgress(null);
    }
  }

  const isAllVisibleSelected = visibleIds.length > 0 && visibleIds.every((id) => selected.has(id));
  const isSomeVisibleSelected = visibleIds.some((id) => selected.has(id));

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center gap-2">
        <KindChip label="전체" count={items.length} isActive={kindFilter === "all"} onClick={() => setKindFilter("all")} />
        {KIND_ORDER.filter((kind) => kindCounts.has(kind)).map((kind) => (
          <KindChip
            key={kind}
            label={kindLabel(kind)}
            count={kindCounts.get(kind) ?? 0}
            isActive={kindFilter === kind}
            onClick={() => setKindFilter(kind)}
          />
        ))}
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <input
          type="search"
          className={`${inputClass} min-w-[220px] flex-1`}
          placeholder="이름, id, 설명으로 검색"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <label className="flex items-center gap-1.5 text-xs text-[var(--color-fg-muted)]">
          템플릿 적용
          <select
            className={inputClass}
            value={appliedTemplate ?? ""}
            onChange={(e) => applyTemplate(e.target.value)}
          >
            <option value="">선택 안 함</option>
            {templates.map((t) => (
              <option key={t.name} value={t.name}>
                {t.name}
              </option>
            ))}
          </select>
        </label>
      </div>

      <Panel
        title={
          <span className="flex items-center gap-3">
            <Checkbox
              label="보이는 항목 전체 선택"
              checked={isAllVisibleSelected}
              indeterminate={isSomeVisibleSelected && !isAllVisibleSelected}
              onChange={(checked) => setChecked(visibleIds, checked)}
            />
            {selectedIds.length > 0 ? `${selectedIds.length}개 선택` : `${items.length}개 항목`}
            {appliedTemplate && <Badge tone="accent">템플릿: {appliedTemplate}</Badge>}
          </span>
        }
        actions={
          <>
            {progress && <span className="text-xs text-[var(--color-fg-muted)]">{progress}</span>}
            <Button onClick={() => setChecked(selectedIds, false)} disabled={selectedIds.length === 0 || isBusy}>
              선택 해제
            </Button>
            <Button
              onClick={() => setOpenForm(openForm === "template" ? null : "template")}
              disabled={selectedIds.length === 0 || isBusy}
            >
              템플릿으로 저장
            </Button>
            <Button onClick={() => onShare(selectedIds)} disabled={selectedIds.length === 0 || isBusy}>
              P2P로 공유
            </Button>
            <Button
              onClick={() => setOpenForm(openForm === "backup" ? null : "backup")}
              disabled={selectedIds.length === 0 || isBusy}
            >
              백업 만들기
            </Button>
            <Button variant="primary" onClick={exportSelection} disabled={selectedIds.length === 0 || isBusy}>
              ZIP으로 내보내기
            </Button>
          </>
        }
      >
        {openForm === "backup" && (
          <BackupForm count={selectedIds.length} isBusy={isBusy} onSubmit={createBackup} onCancel={() => setOpenForm(null)} />
        )}
        {openForm === "template" && (
          <TemplateForm
            count={selectedIds.length}
            isBusy={isBusy}
            onSubmit={saveAsTemplate}
            onCancel={() => setOpenForm(null)}
          />
        )}

        {isLoading && items.length === 0 ? (
          <EmptyRow>불러오는 중...</EmptyRow>
        ) : visibleGroups.length === 0 ? (
          <EmptyRow>조건에 맞는 항목이 없습니다.</EmptyRow>
        ) : (
          visibleGroups.map((group) => {
            const groupIds = group.items.map((i) => i.id);
            const selectedInGroup = groupIds.filter((id) => selected.has(id)).length;
            return (
              <div key={group.kind}>
                <div className="flex items-center gap-3 border-b border-[var(--color-border-muted)] bg-[var(--color-canvas-subtle)] px-4 py-1.5">
                  <Checkbox
                    label={`${kindLabel(group.kind)} 전체 선택`}
                    checked={selectedInGroup === groupIds.length}
                    indeterminate={selectedInGroup > 0 && selectedInGroup < groupIds.length}
                    onChange={(checked) => setChecked(groupIds, checked)}
                  />
                  <span className="text-xs font-semibold">{kindLabel(group.kind)}</span>
                  <span className="text-xs text-[var(--color-fg-muted)]">
                    {selectedInGroup > 0 ? `${selectedInGroup} / ${groupIds.length}` : groupIds.length}
                  </span>
                </div>
                {group.items.map((summary) => (
                  <ItemRow
                    key={summary.id}
                    item={summary}
                    checked={selected.has(summary.id)}
                    onToggle={(checked) => setChecked([summary.id], checked)}
                  />
                ))}
              </div>
            );
          })
        )}
      </Panel>
    </div>
  );
}

function KindChip({
  label,
  count,
  isActive,
  onClick,
}: {
  label: string;
  count: number;
  isActive: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`rounded-full border px-3 py-0.5 text-xs font-medium transition ${
        isActive
          ? "border-[var(--color-accent-fg)] bg-[var(--color-accent-fg)] text-white"
          : "border-[var(--color-border-default)] bg-[var(--color-canvas-default)] text-[var(--color-fg-default)] hover:bg-[var(--color-btn-hover-bg)]"
      }`}
    >
      {label} <span className={isActive ? "opacity-80" : "text-[var(--color-fg-muted)]"}>{count}</span>
    </button>
  );
}

function ItemRow({
  item,
  checked,
  onToggle,
}: {
  item: ItemSummary;
  checked: boolean;
  onToggle: (checked: boolean) => void;
}) {
  return (
    <label
      className={`flex cursor-pointer items-start gap-3 border-b border-[var(--color-border-muted)] px-4 py-2 hover:bg-[var(--color-canvas-subtle)] ${
        checked ? "bg-[#ddf4ff55]" : ""
      }`}
    >
      <span className="pt-0.5">
        <Checkbox label={item.id} checked={checked} onChange={onToggle} />
      </span>
      <span className="min-w-0 flex-1">
        <span className="flex flex-wrap items-center gap-1.5">
          <span className="text-sm font-medium">{item.name}</span>
          {item.executable && <Badge tone="danger">실행</Badge>}
          {item.secret_count > 0 && <Badge tone="accent">시크릿 마스킹 {item.secret_count}</Badge>}
        </span>
        <span className="block font-mono text-[11px] text-[var(--color-fg-subtle)]">{item.id}</span>
        {item.description && <span className="block text-xs text-[var(--color-fg-muted)]">{item.description}</span>}
      </span>
      <span className="shrink-0 text-right text-xs text-[var(--color-fg-muted)]">
        <span className="block">{formatSize(item.size_bytes)}</span>
        {item.file_count > 1 && <span className="block">파일 {item.file_count}개</span>}
      </span>
    </label>
  );
}

function BackupForm({
  count,
  isBusy,
  onSubmit,
  onCancel,
}: {
  count: number;
  isBusy: boolean;
  onSubmit: (label: string) => void;
  onCancel: () => void;
}) {
  const [label, setLabel] = useState("");

  function submit(e: FormEvent) {
    e.preventDefault();
    onSubmit(label.trim());
  }

  return (
    <form
      onSubmit={submit}
      className="flex flex-wrap items-center gap-2 border-b border-[var(--color-border-default)] bg-[#ddf4ff55] px-4 py-2.5"
    >
      <span className="text-sm">선택한 {count}개 항목 백업</span>
      <input
        className={`${inputClass} min-w-[200px] flex-1`}
        placeholder="라벨 (선택)"
        value={label}
        onChange={(e) => setLabel(e.target.value)}
        autoFocus
      />
      <Button type="submit" variant="primary" disabled={isBusy}>
        백업
      </Button>
      <Button onClick={onCancel} disabled={isBusy}>
        취소
      </Button>
    </form>
  );
}

function TemplateForm({
  count,
  isBusy,
  onSubmit,
  onCancel,
}: {
  count: number;
  isBusy: boolean;
  onSubmit: (name: string, description: string) => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");

  function submit(e: FormEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    onSubmit(name.trim(), description.trim());
  }

  return (
    <form
      onSubmit={submit}
      className="flex flex-wrap items-center gap-2 border-b border-[var(--color-border-default)] bg-[#ddf4ff55] px-4 py-2.5"
    >
      <span className="text-sm">선택한 {count}개 항목을 템플릿으로</span>
      <input
        className={`${inputClass} w-40`}
        placeholder="이름"
        value={name}
        onChange={(e) => setName(e.target.value)}
        autoFocus
        required
      />
      <input
        className={`${inputClass} min-w-[200px] flex-1`}
        placeholder="설명"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
      />
      <Button type="submit" variant="primary" disabled={isBusy || !name.trim()}>
        저장
      </Button>
      <Button onClick={onCancel} disabled={isBusy}>
        취소
      </Button>
    </form>
  );
}
