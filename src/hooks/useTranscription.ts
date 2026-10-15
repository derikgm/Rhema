import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

export function useTranscription(onFinal: (text: string) => void, onPartial?: (text: string) => void) {
  useEffect(() => {
    const unsubs: Array<() => void> = [];
    listen<string>("final-transcript", (e) => onFinal(e.payload)).then((u) => unsubs.push(u));
    if (onPartial) {
      listen<string>("partial-transcript", (e) => onPartial(e.payload)).then((u) => unsubs.push(u));
    }
    return () => {
      unsubs.forEach((u) => u());
    };
  }, [onFinal, onPartial]);
}
