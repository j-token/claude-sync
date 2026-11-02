import { useEffect, useState, type FormEvent } from "react";
import { invokeCommand } from "../lib/backend";
import { errorText, formatSize } from "../lib/format";
import { useNotify } from "../lib/notify";
import type { BundlePreview, ImportMode, PeerInfo, SharePhase, TemplateInfo } from "../lib/types";
import type { ShareSession } from "../lib/useShareSession";
import BundlePreviewPanel from "./BundlePreviewPanel";
import { Badge, Button, EmptyRow, Notice, Panel, inputClass, type BadgeTone } from "./ui";

const PHASE_BADGES: Record<SharePhase, { label: string; tone: BadgeTone }> = {
  idle: { label: "대기", tone: "neutral" },
  waiting: { label: "연결 대기 중", tone: "accent" },
  transferring: { label: "전송 중", tone: "attention" },
  completed: { label: "완료", tone: "success" },
  failed: { label: "실패", tone: "danger" },
  stopped: { label: "중지됨", tone: "neutral" },
};

export default function P2PTab({
  refreshKey,
  session,
  pendingIds,
  onClearPending,
  onChanged,
}: {
  refreshKey: number;
  session: ShareSession;
  pendingIds: string[];
  onClearPending: () => void;
  onChanged: () => void;
}) {
  const [preview, setPreview] = useState<BundlePreview | null>(null);

  async function importReceived(path: string, ids: string[], mode: ImportMode) {
    const report = await invokeCommand("import_bundle", { path, ids, mode });
    onChanged();
    return report;
  }

  if (preview) {
    return (
      <BundlePreviewPanel
        key={preview.path}
        preview={preview}
        title={`받은 번들 · ${preview.device_name}`}
        onImport={(ids, mode) => importReceived(preview.path, ids, mode)}
        onClose={() => setPreview(null)}
      />
    );
  }

  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <SharePanel refreshKey={refreshKey} session={session} pendingIds={pendingIds} onClearPending={onClearPending} />
      <ReceivePanel onReceived={setPreview} />
    </div>
  );
}

function SharePanel({
  refreshKey,
  session,
  pendingIds,
  onClearPending,
}: {
  refreshKey: number;
  session: ShareSession;
  pendingIds: string[];
  onClearPending: () => void;
}) {
  const notify = useNotify();
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [templateName, setTemplateName] = useState("");

  useEffect(() => {
    Promise.all([invokeCommand("list_templates"), invokeCommand("get_config")])
      .then(([loadedTemplates, config]) => {
        setTemplates(loadedTemplates);
        setTemplateName((current) => current || config.default_template);
      })
      .catch((e) => notify("error", `템플릿 목록을 불러오지 못했습니다: ${errorText(e)}`));
  }, [refreshKey, notify]);

  async function startShare() {
    if (pendingIds.length > 0) {
      await session.start(pendingIds);
      return;
    }
    try {
      const ids = await invokeCommand("resolve_template", { name: templateName });
      await session.start(ids);
    } catch (e) {
      notify("error", `템플릿을 해석하지 못했습니다: ${errorText(e)}`);
    }
  }

  const { info, status, isActive, isStarting } = session;
  const phase = status?.phase ?? (info ? "waiting" : "idle");
  const phaseBadge = PHASE_BADGES[phase];

  return (
    <Panel title="공유하기" actions={info && <Badge tone={phaseBadge.tone}>{phaseBadge.label}</Badge>}>
      <div className="space-y-4 px-4 py-4 text-sm">
        {info && (
          <div className="space-y-3">
            {isActive && (
              <>
                <div className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)] px-4 py-4 text-center">
                  <div className="text-xs text-[var(--color-fg-muted)]">페어링 코드</div>
                  <div className="font-mono text-4xl font-semibold tracking-[0.15em] select-all">{info.code}</div>
                  <div className="mt-1 text-xs text-[var(--color-fg-muted)]">
                    {info.item_count}개 항목 · {formatSize(info.size_bytes)} · 포트 {info.port}
                  </div>
                </div>
                <div>
                  <div className="mb-1 text-xs font-semibold text-[var(--color-fg-muted)]">이 기기의 주소</div>
                  <ul className="m-0 list-none space-y-0.5 p-0 font-mono text-sm">
                    {info.addresses.map((address) => (
                      <li key={address} className="select-all">
                        {address}
                      </li>
                    ))}
                  </ul>
                </div>
              </>
            )}
            {status && (
              <div className="text-[var(--color-fg-muted)]">
                {status.message}
                {status.peer && <span> · 상대: {status.peer}</span>}
                {status.failed_attempts > 0 && <span> · 잘못된 코드 시도 {status.failed_attempts}회</span>}
              </div>
            )}
            {isActive && (
              <Button variant="danger" onClick={session.stop}>
                공유 중지
              </Button>
            )}
          </div>
        )}

        {!isActive && (
          <div className="space-y-3">
            {pendingIds.length > 0 ? (
              <div className="flex flex-wrap items-center gap-2">
                <span>항목 탭에서 선택한 {pendingIds.length}개 항목</span>
                <Button onClick={onClearPending}>템플릿으로 고르기</Button>
              </div>
            ) : (
              <label className="flex items-center gap-2">
                <span className="text-[var(--color-fg-muted)]">공유할 템플릿</span>
                <select className={inputClass} value={templateName} onChange={(e) => setTemplateName(e.target.value)}>
                  {templates.map((t) => (
                    <option key={t.name} value={t.name}>
                      {t.name}
                    </option>
                  ))}
                </select>
              </label>
            )}
            <Button
              variant="primary"
              onClick={startShare}
              disabled={isStarting || (pendingIds.length === 0 && !templateName)}
            >
              {isStarting ? "시작하는 중..." : info ? "다시 공유 시작" : "공유 시작"}
            </Button>
          </div>
        )}

        <p className="m-0 text-xs text-[var(--color-fg-muted)]">
          받는 기기에서 위 주소 중 하나와 페어링 코드를 입력하면 같은 네트워크 안에서 직접 전송됩니다. 처음 공유할 때
          Windows 방화벽이 네트워크 접근 허용을 물어볼 수 있습니다. 개인 네트워크에서 허용해 주세요.
        </p>
      </div>
    </Panel>
  );
}

function ReceivePanel({ onReceived }: { onReceived: (preview: BundlePreview) => void }) {
  const notify = useNotify();
  const [peers, setPeers] = useState<PeerInfo[] | null>(null);
  const [isDiscovering, setIsDiscovering] = useState(false);
  const [address, setAddress] = useState("");
  const [code, setCode] = useState("");
  const [isReceiving, setIsReceiving] = useState(false);

  async function discover() {
    setIsDiscovering(true);
    try {
      setPeers(await invokeCommand("p2p_discover", { timeout_ms: 3_000 }));
    } catch (e) {
      notify("error", `기기 찾기 실패: ${errorText(e)}`);
    } finally {
      setIsDiscovering(false);
    }
  }

  async function receive(e: FormEvent) {
    e.preventDefault();
    setIsReceiving(true);
    try {
      const preview = await invokeCommand("p2p_receive", { address: address.trim(), code: code.trim().toUpperCase() });
      notify("success", `${preview.device_name}에서 ${preview.items.length}개 항목을 받았습니다. 가져올 항목을 고르세요.`);
      setCode("");
      onReceived(preview);
    } catch (err) {
      notify("error", `받기 실패: ${errorText(err)}`);
    } finally {
      setIsReceiving(false);
    }
  }

  return (
    <Panel
      title="받기"
      actions={
        <Button onClick={discover} disabled={isDiscovering}>
          {isDiscovering ? "찾는 중..." : "주변 기기 찾기"}
        </Button>
      }
    >
      {peers !== null &&
        (peers.length === 0 ? (
          <EmptyRow>주변에서 공유 중인 기기를 찾지 못했습니다.</EmptyRow>
        ) : (
          <ul className="m-0 list-none border-b border-[var(--color-border-default)] p-0">
            {peers.map((peer) => (
              <li key={peer.address}>
                <button
                  type="button"
                  onClick={() => setAddress(peer.address)}
                  className={`flex w-full items-center justify-between gap-3 border-b border-[var(--color-border-muted)] px-4 py-2 text-left last:border-b-0 hover:bg-[var(--color-canvas-subtle)] ${
                    address === peer.address ? "bg-[#ddf4ff55]" : ""
                  }`}
                >
                  <span>
                    <span className="block text-sm font-medium">{peer.device_name}</span>
                    <span className="block font-mono text-xs text-[var(--color-fg-muted)]">{peer.address}</span>
                  </span>
                  <span className="text-xs text-[var(--color-fg-muted)]">{peer.item_count}개 항목</span>
                </button>
              </li>
            ))}
          </ul>
        ))}

      <form onSubmit={receive} className="space-y-3 px-4 py-4 text-sm">
        <label className="block">
          <span className="mb-1 block text-xs font-semibold text-[var(--color-fg-muted)]">주소 (ip:port)</span>
          <input
            className={`${inputClass} w-full font-mono`}
            placeholder="192.168.0.24:47321"
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            required
          />
        </label>
        <label className="block">
          <span className="mb-1 block text-xs font-semibold text-[var(--color-fg-muted)]">페어링 코드</span>
          <input
            className={`${inputClass} w-full font-mono tracking-widest uppercase`}
            placeholder="K7P3-QX9M"
            value={code}
            onChange={(e) => setCode(e.target.value)}
            required
          />
        </label>
        <Button type="submit" variant="primary" disabled={isReceiving || !address.trim() || !code.trim()}>
          {isReceiving ? "받는 중..." : "받기"}
        </Button>
        <Notice tone="accent">받은 번들은 바로 적용되지 않습니다. 미리보기에서 가져올 항목을 고른 뒤 적용합니다.</Notice>
      </form>
    </Panel>
  );
}
