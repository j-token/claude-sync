import { createContext, useContext } from "react";

export type NoticeTone = "success" | "error";
export type Notify = (tone: NoticeTone, text: string) => void;

export const NotifyContext = createContext<Notify>(() => {});

export function useNotify(): Notify {
  return useContext(NotifyContext);
}
