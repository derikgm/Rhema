import { useEffect, useRef, useState } from "react";
import { useTranscription } from "@/hooks/useTranscription";
import { applyVoicePunctuation } from "@/utils/postprocessPunctuation";
import type { DictationState } from "@/state/dictationMachine";

interface Props {
  value: string;
  onChange: (v: string) => void;
  voicePunctuation: boolean;
}

const PUNCT = [",", ".", "?", "!"];

export default function Editor({ value, onChange, voicePunctuation }: Props) {
  const [state, setState] = useState<DictationState>("idle");
  const [pending, setPending] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const valueRef = useRef(value);

  useEffect(() => {
    valueRef.current = value;
  }, [value]);

  useTranscription(
    (t) => {
      const raw = voicePunctuation ? applyVoicePunctuation(t) : t;
      const base = valueRef.current.trimEnd();
      const next = base ? base + " " + raw : raw;
      onChange(next);
      setPending("");
      setState("listening");
    },
    (p) => {
      setPending(voicePunctuation ? applyVoicePunctuation(p) : p);
      setState("transcribing-segment");
    }
  );

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (PUNCT.includes(e.key) && pending.trim()) {
      e.preventDefault();
      const base = value.trimEnd();
      const confirmed = base ? base + " " + pending.trim() : pending.trim();
      onChange(confirmed + e.key);
      setPending("");
      setState("listening");
      return;
    }
    if (state === "idle") setState("listening");
  }

  return (
    <div className="w-full space-y-2">
      <div className="text-xs text-slate-400">Estado: {state}{pending ? ` · pendiente: "${pending}"` : ""}</div>
      <textarea
        ref={textareaRef}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={handleKeyDown}
        className="w-full h-64 p-4 rounded bg-slate-800 border border-slate-700 text-slate-100 text-lg resize-none"
        placeholder="El texto dictado aparecerá aquí..."
      />
    </div>
  );
}
