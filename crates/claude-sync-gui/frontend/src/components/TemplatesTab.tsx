import { useEffect, useMemo, useState, type FormEvent } from "react";
import { invokeCommand } from "../lib/backend";
import { errorText } from "../lib/format";
import { matchesTemplate } from "../lib/glob";
import { useNotify } from "../lib/notify";
import type { ItemSummary, TemplateInfo } from "../lib/types";
import { Badge, Button, EmptyRow, Panel, inputClass } from "./ui";

interface TemplateDraft {
  originalName: string | null;
  name: string;
  description: string;
  includeText: string;
  excludeText: string;
}

const EMPTY_DRAFT: TemplateDraft = { originalName: null, name: "", description: "", includeText: "", excludeText: "" };

const PATTERN_HELP = [
  "skill:<이름>",
  "agent:<파일>",
  "command:<파일>",
  "rule:<파일>",
  "setting:<최상위 키>",
  "hook:<이벤트>",
  "hook_script:<파일>",
  "mcp:user:<이름>",
  "mcp:settings:<이름>",
  "plugin:<이름@마켓>",
  "output_style:<파일>",
  "memory:<파일>",
  "team:<이름>",
  "claude_md",
  "keybindings",
];

function parsePatterns(text: string): string[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

function draftFrom(template: TemplateInfo, asCopy: boolean): TemplateDraft {
  return {
    originalName: asCopy ? null : template.name,
    name: asCopy ? `${template.name}-copy` : template.name,
    description: template.description,
    includeText: template.include.join("\n"),
    excludeText: template.exclude.join("\n"),
  };
}

export default function TemplatesTab({ refreshKey, onChanged }: { refreshKey: number; onChanged: () => void }) {
  const notify = useNotify();
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [items, setItems] = useState<ItemSummary[]>([]);
  const [draft, setDraft] = useState<TemplateDraft | null>(null);
  const [confirmDeleteName, setConfirmDeleteName] = useState<string | null>(null);
  const [progress, setProgress] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([invokeCommand("list_templates"), invokeCommand("list_items")])
      .then(([loadedTemplates, loadedItems]) => {
        setTemplates(loadedTemplates);
        setItems(loadedItems);
      })
      .catch((e) => notify("error", `템플릿 목록을 불러오지 못했습니다: ${errorText(e)}`));
  }, [refreshKey, notify]);

  function countMatches(include: string[], exclude: string[]): number {
    return items.filter((i) => matchesTemplate(i.id, include, exclude)).length;
  }

  async function deleteTemplate(name: string) {
    setProgress("삭제하는 중...");
    try {
      notify("success", await invokeCommand("delete_template", { name }));
      setConfirmDeleteName(null);
      onChanged();
    } catch (e) {
      notify("error", `템플릿 삭제 실패: ${errorText(e)}`);
    } finally {
      setProgress(null);
    }
  }

  async function saveDraft(toSave: TemplateDraft) {
    setProgress("저장하는 중...");
    try {
      const saved = await invokeCommand("save_template", {
        name: toSave.name.trim(),
        description: toSave.description.trim(),
        include: parsePatterns(toSave.includeText),
        exclude: parsePatterns(toSave.excludeText),
      });
      // 이름을 바꾼 경우 save_template은 새 이름으로 저장만 하므로 이전 이름은 따로 지운다.
      if (toSave.originalName && toSave.originalName !== saved.name) {
        await invokeCommand("delete_template", { name: toSave.originalName });
      }
      notify("success", `템플릿 '${saved.name}'을(를) 저장했습니다.`);
      setDraft(null);
      onChanged();
    } catch (e) {
      notify("error", `템플릿 저장 실패: ${errorText(e)}`);
    } finally {
      setProgress(null);
    }
  }

  const isBusy = progress !== null;

  return (
    <div className="space-y-3">
      {draft && (
        <TemplateEditor
          draft={draft}
          items={items}
          isBusy={isBusy}
          onChange={setDraft}
          onSubmit={saveDraft}
          onCancel={() => setDraft(null)}
        />
      )}

      <Panel
        title={`템플릿 ${templates.length}개`}
        actions={
          <>
            {progress && <span className="text-xs text-[var(--color-fg-muted)]">{progress}</span>}
            <Button variant="primary" onClick={() => setDraft(EMPTY_DRAFT)} disabled={isBusy}>
              새 템플릿
            </Button>
          </>
        }
      >
        {templates.length === 0 ? (
          <EmptyRow>템플릿이 없습니다.</EmptyRow>
        ) : (
          <ul className="m-0 list-none p-0">
            {templates.map((template) => (
              <li
                key={template.name}
                className="flex flex-wrap items-start gap-3 border-b border-[var(--color-border-muted)] px-4 py-3 last:border-b-0"
              >
                <div className="min-w-0 flex-1 space-y-1.5">
                  <div className="flex flex-wrap items-center gap-1.5">
                    <span className="text-sm font-semibold">{template.name}</span>
                    {template.builtin && <Badge>기본 제공</Badge>}
                    <span className="text-xs text-[var(--color-fg-muted)]">
                      로컬 항목 {countMatches(template.include, template.exclude)}개 해당
                    </span>
                  </div>
                  {template.description && (
                    <div className="text-xs text-[var(--color-fg-muted)]">{template.description}</div>
                  )}
                  <PatternChips label="포함" patterns={template.include} tone="success" />
                  <PatternChips label="제외" patterns={template.exclude} tone="danger" />
                </div>
                <div className="flex gap-1.5">
                  {template.builtin ? (
                    <Button onClick={() => setDraft(draftFrom(template, true))} disabled={isBusy}>
                      복제
                    </Button>
                  ) : confirmDeleteName === template.name ? (
                    <>
                      <Button variant="danger" onClick={() => deleteTemplate(template.name)} disabled={isBusy}>
                        정말 삭제
                      </Button>
                      <Button onClick={() => setConfirmDeleteName(null)} disabled={isBusy}>
                        취소
                      </Button>
                    </>
                  ) : (
                    <>
                      <Button onClick={() => setDraft(draftFrom(template, false))} disabled={isBusy}>
                        편집
                      </Button>
                      <Button variant="danger" onClick={() => setConfirmDeleteName(template.name)} disabled={isBusy}>
                        삭제
                      </Button>
                    </>
                  )}
                </div>
              </li>
            ))}
          </ul>
        )}
      </Panel>
    </div>
  );
}

function PatternChips({
  label,
  patterns,
  tone,
}: {
  label: string;
  patterns: string[];
  tone: "success" | "danger";
}) {
  if (patterns.length === 0) return null;
  return (
    <div className="flex flex-wrap items-center gap-1">
      <span className="w-8 text-xs text-[var(--color-fg-muted)]">{label}</span>
      {patterns.map((pattern) => (
        <span
          key={pattern}
          className={`rounded-md border px-1.5 font-mono text-[11px] leading-[18px] ${
            tone === "success"
              ? "border-[#1a7f3740] bg-[#dafbe1] text-[var(--color-success-fg)]"
              : "border-[#d1242f40] bg-[#ffebe9] text-[var(--color-danger-fg)]"
          }`}
        >
          {pattern}
        </span>
      ))}
    </div>
  );
}

function TemplateEditor({
  draft,
  items,
  isBusy,
  onChange,
  onSubmit,
  onCancel,
}: {
  draft: TemplateDraft;
  items: ItemSummary[];
  isBusy: boolean;
  onChange: (draft: TemplateDraft) => void;
  onSubmit: (draft: TemplateDraft) => void;
  onCancel: () => void;
}) {
  const matchedItems = useMemo(() => {
    const include = parsePatterns(draft.includeText);
    const exclude = parsePatterns(draft.excludeText);
    return items.filter((i) => matchesTemplate(i.id, include, exclude));
  }, [draft.includeText, draft.excludeText, items]);

  function submit(e: FormEvent) {
    e.preventDefault();
    if (!draft.name.trim()) return;
    onSubmit(draft);
  }

  return (
    <Panel title={draft.originalName ? `템플릿 편집 · ${draft.originalName}` : "새 템플릿"}>
      <form onSubmit={submit} className="grid gap-4 px-4 py-4 text-sm lg:grid-cols-[minmax(0,1fr)_280px]">
        <div className="space-y-3">
          <div className="flex flex-wrap gap-3">
            <label className="block w-48">
              <span className="mb-1 block text-xs font-semibold text-[var(--color-fg-muted)]">이름</span>
              <input
                className={`${inputClass} w-full`}
                value={draft.name}
                onChange={(e) => onChange({ ...draft, name: e.target.value })}
                required
              />
            </label>
            <label className="block min-w-[220px] flex-1">
              <span className="mb-1 block text-xs font-semibold text-[var(--color-fg-muted)]">설명</span>
              <input
                className={`${inputClass} w-full`}
                value={draft.description}
                onChange={(e) => onChange({ ...draft, description: e.target.value })}
              />
            </label>
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="block">
              <span className="mb-1 block text-xs font-semibold text-[var(--color-fg-muted)]">포함 패턴 (한 줄에 하나)</span>
              <textarea
                className={`${inputClass} h-40 w-full font-mono text-xs`}
                placeholder={"skill:*\nagent:*\nclaude_md"}
                value={draft.includeText}
                onChange={(e) => onChange({ ...draft, includeText: e.target.value })}
              />
            </label>
            <label className="block">
              <span className="mb-1 block text-xs font-semibold text-[var(--color-fg-muted)]">제외 패턴 (한 줄에 하나)</span>
              <textarea
                className={`${inputClass} h-40 w-full font-mono text-xs`}
                placeholder="setting:env"
                value={draft.excludeText}
                onChange={(e) => onChange({ ...draft, excludeText: e.target.value })}
              />
            </label>
          </div>
          <p className="m-0 text-xs text-[var(--color-fg-muted)]">
            <code>*</code>는 임의의 문자열과 일치합니다. 항목 id 형식:{" "}
            {PATTERN_HELP.map((pattern) => (
              <code key={pattern} className="mr-1.5 font-mono text-[11px]">
                {pattern}
              </code>
            ))}
          </p>
          <div className="flex gap-2">
            <Button type="submit" variant="primary" disabled={isBusy || !draft.name.trim()}>
              저장
            </Button>
            <Button onClick={onCancel} disabled={isBusy}>
              취소
            </Button>
          </div>
        </div>

        <div className="rounded-md border border-[var(--color-border-default)]">
          <div className="border-b border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)] px-3 py-1.5 text-xs font-semibold">
            미리보기: 로컬 항목 {matchedItems.length} / {items.length}개
          </div>
          <ul className="m-0 max-h-64 list-none overflow-y-auto p-0 font-mono text-[11px]">
            {matchedItems.map((i) => (
              <li key={i.id} className="border-b border-[var(--color-border-muted)] px-3 py-1 last:border-b-0">
                {i.id}
              </li>
            ))}
          </ul>
        </div>
      </form>
    </Panel>
  );
}
