import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ModelInfo {
  id: string;
  name: string;
  language: string;
  streaming: boolean;
  url: string;
  filename: string;
  size_mb: number;
}

interface Progress {
  filename: string;
  downloaded_mb: number;
  total_mb: number;
  done: boolean;
}

export default function ModelPicker() {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [status, setStatus] = useState("");
  const [progress, setProgress] = useState<Record<string, Progress>>({});

  useEffect(() => {
    const unlisten = listen<Progress>("model-download-progress", (e) => {
      setProgress((p) => ({ ...p, [e.payload.filename]: e.payload }));
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  async function load() {
    const res = await invoke<ModelInfo[]>("list_models");
    setModels(res);
  }

  async function download(m: ModelInfo) {
    setStatus(`Descargando ${m.name}...`);
    try {
      const path = await invoke<string>("download_model", { id: m.id });
      setStatus(`Descargado en ${path}`);
    } catch (e) {
      setStatus(`Error: ${String(e)}`);
    }
  }

  return (
    <div className="p-4 space-y-3">
      <button className="px-3 py-2 bg-indigo-500 rounded" onClick={load}>Cargar modelos</button>
      {models.map((m) => {
        const p = progress[m.filename];
        return (
          <div key={m.id} className="bg-slate-800 p-3 rounded space-y-2">
            <div className="flex items-center justify-between">
              <div>
                <div className="font-semibold">{m.name}</div>
                <div className="text-slate-400 text-sm">{m.language} · {m.streaming ? "streaming" : "offline"} · ~{m.size_mb} MB</div>
              </div>
              <button className="px-3 py-1 bg-slate-700 rounded" onClick={() => download(m)}>
                Descargar
              </button>
            </div>
            {p && (
              <div className="text-cyan-300 text-sm">
                {p.downloaded_mb.toFixed(1)} / {p.total_mb.toFixed(1)} MB {p.done ? "✅" : ""}
              </div>
            )}
          </div>
        );
      })}
      {status && <p className="text-cyan-300">{status}</p>}
    </div>
  );
}
