import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";

interface Entry { name: string; path: string; is_dir: boolean }

export default function FileTree({ onOpen }: { onOpen: (path: string, text: string) => void }) {
  const [root, setRoot] = useState<string>("");
  const [entries, setEntries] = useState<Entry[]>([]);

  async function pick() {
    const dir = await open({ directory: true });
    if (typeof dir === "string") {
      setRoot(dir);
      const res = await invoke<Entry[]>("list_dir", { path: dir });
      setEntries(res);
    }
  }

  async function openEntry(e: Entry) {
    if (e.is_dir) {
      const res = await invoke<Entry[]>("list_dir", { path: e.path });
      setEntries(res);
      setRoot(e.path);
    } else {
      const text = await invoke<string>("read_text", { path: e.path });
      onOpen(e.path, text);
    }
  }

  return (
    <div className="bg-slate-800 p-3 rounded space-y-2">
      <button className="px-3 py-2 bg-indigo-500 rounded" onClick={pick}>Abrir carpeta</button>
      {root && <p className="text-slate-400 text-xs truncate">{root}</p>}
      <ul className="space-y-1">
        {entries.map((e) => (
          <li key={e.path}>
            <button className="text-left w-full hover:text-cyan-300" onClick={() => openEntry(e)}>
              {e.is_dir ? "📁" : "📄"} {e.name}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
