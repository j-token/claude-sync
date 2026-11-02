import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import { invokeCommand } from "../lib/backend";
import { errorText } from "../lib/format";
import { useNotify } from "../lib/notify";
import type { AppConfig, SecretAction, SecretPatternInfo, TemplateInfo } from "../lib/types";
import { Button, EmptyRow, Panel, Toggle, inputClass, tableCellClass, tableHeadClass } from "./ui";

const SECRET_ACTION_LABELS: Record<SecretAction, string> = {
  mask: "마스킹",
  remove: "제거",
  keep: "유지",
};

export default function SettingsTab({ refreshKey, onChanged }: { refreshKey: number; onChanged: () => void }) {
  const notify = useNotify();
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    Promise.all([invokeCommand("get_config"), invokeCommand("list_templates")])
      .then(([loadedConfig, loadedTemplates]) => {
        setConfig(loadedConfig);
        setTemplates(loadedTemplates);
      })
      .catch((e) => notify("error", `설정을 불러오지 못했습니다: ${errorText(e)}`));
  }, [refreshKey, notify]);

  if (!config) {
    return (
      <Panel>
        <EmptyRow>불러오는 중...</EmptyRow>
      </Panel>
    );
  }

  function update(patch: Partial<AppConfig>) {
    setConfig((prev) => (prev ? { ...prev, ...patch } : prev));
  }

  function updatePattern(index: number, patch: Partial<SecretPatternInfo>) {
    if (!config) return;
    update({ secret_patterns: config.secret_patterns.map((p, i) => (i === index ? { ...p, ...patch } : p)) });
  }

  async function save(e: FormEvent) {
    e.preventDefault();
    if (!config) return;
    setIsSaving(true);
    try {
      const secretPatterns = config.secret_patterns.filter((p) => p.name.trim() !== "" && p.json_path.trim() !== "");
      setConfig(await invokeCommand("update_config", { ...config, secret_patterns: secretPatterns }));
      notify("success", "설정을 저장했습니다.");
      onChanged();
    } catch (err) {
      notify("error", `설정 저장 실패: ${errorText(err)}`);
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <form onSubmit={save} className="space-y-3">
      <Panel title="일반">
        <div className="divide-y divide-[var(--color-border-muted)]">
          <Field label="기기 이름" hint="P2P 공유와 번들 정보에 표시됩니다.">
            <input
              className={`${inputClass} w-64`}
              value={config.device_name}
              onChange={(e) => update({ device_name: e.target.value })}
              required
            />
          </Field>
          <Field label="기본 템플릿" hint="빠른 백업·내보내기·공유에 쓰입니다.">
            <select
              className={inputClass}
              value={config.default_template}
              onChange={(e) => update({ default_template: e.target.value })}
            >
              {templates.map((t) => (
                <option key={t.name} value={t.name}>
                  {t.name}
                </option>
              ))}
            </select>
          </Field>
        </div>
      </Panel>

      <Panel title="백업">
        <div className="divide-y divide-[var(--color-border-muted)]">
          <Field label="가져오기 전 자동 백업" hint="가져오기·복원 직전에 현재 상태를 백업합니다.">
            <Toggle
              label="가져오기 전 자동 백업"
              checked={config.backup_before_import}
              onChange={(checked) => update({ backup_before_import: checked })}
            />
          </Field>
          <Field label="자동 백업 보관 개수" hint="넘으면 오래된 자동 백업부터 지웁니다. 수동 백업은 지우지 않습니다.">
            <input
              type="number"
              min={1}
              className={`${inputClass} w-24`}
              value={config.max_auto_backups}
              onChange={(e) => update({ max_auto_backups: Number(e.target.value) })}
            />
          </Field>
        </div>
      </Panel>

      <Panel title="P2P">
        <div className="divide-y divide-[var(--color-border-muted)]">
          <Field label="포트" hint="공유할 때 이 포트로 연결을 기다립니다.">
            <input
              type="number"
              min={1024}
              max={65535}
              className={`${inputClass} w-28`}
              value={config.p2p_port}
              onChange={(e) => update({ p2p_port: Number(e.target.value) })}
            />
          </Field>
          <Field label="주변 기기 찾기 허용" hint="같은 네트워크의 다른 기기가 이 기기의 공유를 찾을 수 있습니다.">
            <Toggle
              label="주변 기기 찾기 허용"
              checked={config.p2p_discovery}
              onChange={(checked) => update({ p2p_discovery: checked })}
            />
          </Field>
        </div>
      </Panel>

      <Panel
        title="시크릿 감지 규칙"
        actions={
          <Button
            onClick={() =>
              update({ secret_patterns: [...config.secret_patterns, { name: "", json_path: "", action: "mask" }] })
            }
          >
            규칙 추가
          </Button>
        }
      >
        {config.secret_patterns.length === 0 ? (
          <EmptyRow>규칙이 없습니다.</EmptyRow>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-sm">
              <thead>
                <tr>
                  <th className={tableHeadClass}>이름</th>
                  <th className={tableHeadClass}>JSON 경로 패턴</th>
                  <th className={tableHeadClass}>처리</th>
                  <th className={tableHeadClass} />
                </tr>
              </thead>
              <tbody>
                {config.secret_patterns.map((pattern, index) => (
                  <tr key={index}>
                    <td className={tableCellClass}>
                      <input
                        className={`${inputClass} w-full`}
                        value={pattern.name}
                        onChange={(e) => updatePattern(index, { name: e.target.value })}
                      />
                    </td>
                    <td className={tableCellClass}>
                      <input
                        className={`${inputClass} w-full font-mono text-xs`}
                        value={pattern.json_path}
                        onChange={(e) => updatePattern(index, { json_path: e.target.value })}
                      />
                    </td>
                    <td className={tableCellClass}>
                      <select
                        className={inputClass}
                        value={pattern.action}
                        onChange={(e) => updatePattern(index, { action: e.target.value as SecretAction })}
                      >
                        {(Object.keys(SECRET_ACTION_LABELS) as SecretAction[]).map((action) => (
                          <option key={action} value={action}>
                            {SECRET_ACTION_LABELS[action]}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td className={`${tableCellClass} text-right`}>
                      <Button
                        variant="danger"
                        onClick={() =>
                          update({ secret_patterns: config.secret_patterns.filter((_, i) => i !== index) })
                        }
                      >
                        삭제
                      </Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Panel>

      <div className="flex justify-end">
        <Button type="submit" variant="primary" disabled={isSaving}>
          {isSaving ? "저장하는 중..." : "설정 저장"}
        </Button>
      </div>
    </form>
  );
}

function Field({ label, hint, children }: { label: string; hint: string; children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
      <div>
        <div className="text-sm font-medium">{label}</div>
        <div className="text-xs text-[var(--color-fg-muted)]">{hint}</div>
      </div>
      {children}
    </div>
  );
}
