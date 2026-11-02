import type { CommandInput, CommandName, CommandOutput } from "./commands";
import { mockInvoke } from "./mock";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

type CommandArgs<K extends CommandName> = CommandInput<K> extends void ? [] : [CommandInput<K>];

export async function invokeCommand<K extends CommandName>(
  command: K,
  ...args: CommandArgs<K>
): Promise<CommandOutput<K>> {
  const input: unknown = args[0];

  if (!isTauriRuntime()) {
    return (await mockInvoke(command, input)) as CommandOutput<K>;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  if (input === undefined) {
    return invoke<CommandOutput<K>>(command);
  }
  return invoke<CommandOutput<K>>(command, { input });
}
