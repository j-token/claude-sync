import { useEffect, useState } from "react";
import { invokeCommand } from "../lib/backend";
import { errorText } from "../lib/format";
import { useNotify } from "../lib/notify";
import type { DetectedSecret } from "../lib/types";
import { EmptyRow, Notice, Panel, tableCellClass, tableHeadClass } from "./ui";

export default function SecretsTab({ refreshKey }: { refreshKey: number }) {
  const notify = useNotify();
  const [secrets, setSecrets] = useState<DetectedSecret[]>([]);

  useEffect(() => {
    invokeCommand("list_secrets")
      .then(setSecrets)
      .catch((e) => notify("error", `시크릿 목록을 불러오지 못했습니다: ${errorText(e)}`));
  }, [refreshKey, notify]);

  return (
    <div className="space-y-3">
      <Notice tone="accent">
        아래 값은 내보내기·백업·P2P 공유 번들에서 비워진 채로 저장됩니다. 번들을 가져올 때는 이 기기에 이미 있는 값으로
        다시 채워지므로, 다른 기기의 키가 덮어쓰이거나 유출되지 않습니다. 감지 규칙은 설정 탭에서 바꿀 수 있습니다.
      </Notice>
      <Panel title={`감지된 시크릿 ${secrets.length}개`}>
        {secrets.length === 0 ? (
          <EmptyRow>감지된 시크릿이 없습니다.</EmptyRow>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-sm">
              <thead>
                <tr>
                  <th className={tableHeadClass}>항목</th>
                  <th className={tableHeadClass}>위치</th>
                  <th className={tableHeadClass}>규칙</th>
                  <th className={tableHeadClass}>미리보기</th>
                </tr>
              </thead>
              <tbody>
                {secrets.map((secret) => (
                  <tr key={`${secret.item_id}${secret.pointer}`}>
                    <td className={`${tableCellClass} font-mono text-xs`}>{secret.item_id}</td>
                    <td className={`${tableCellClass} font-mono text-xs`}>{secret.pointer}</td>
                    <td className={tableCellClass}>{secret.pattern_name}</td>
                    <td className={`${tableCellClass} font-mono text-xs text-[var(--color-fg-muted)]`}>{secret.preview}</td>
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
