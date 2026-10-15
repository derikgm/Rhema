import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function MicButton() {
  const [listening, setListening] = useState(false);
  const [processing, setProcessing] = useState(false);

  async function toggle() {
    setProcessing(true);
    try {
      if (listening) {
        await invoke("stop_listening");
        await invoke("stop_asr");
        setListening(false);
      } else {
        await invoke("start_listening");
        await invoke("start_asr");
        setListening(true);
      }
    } finally {
      setProcessing(false);
    }
  }

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.ctrlKey && e.code === "Space") {
        e.preventDefault();
        toggle();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [listening]);

  return (
    <button
      onClick={toggle}
      disabled={processing}
      className={`w-20 h-20 rounded-full text-2xl font-bold transition ${
        listening ? "bg-cyan-400 animate-pulse text-slate-900" : "bg-indigo-500 text-white"
      }`}
      aria-label="Toggle dictation"
    >
      🎙
    </button>
  );
}
