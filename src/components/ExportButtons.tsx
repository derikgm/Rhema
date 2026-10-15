import { useEffect } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";

export default function ExportButtons({ text }: { text: string }) {
  useEffect(() => {
    const id = setInterval(() => {
      invoke("save_draft", { content: text }).catch(() => {});
    }, 30000);
    return () => clearInterval(id);
  }, [text]);

  async function exportMarkdown() {
    const path = await save({ filters: [{ name: "Markdown", extensions: ["md"] }] });
    if (!path) return;
    const stamp = new Date().toISOString().slice(0, 16).replace("T", " ");
    const md = `# Dictado — ${stamp}\n\n- [${new Date().toLocaleTimeString()}] ${text}\n`;
    await invoke("save_text", { path, content: md });
  }

  async function exportTxt() {
    const path = await save({ filters: [{ name: "Text", extensions: ["txt"] }] });
    if (!path) return;
    await invoke("save_text", { path, content: text });
  }

  return (
    <div className="flex gap-3">
      <button className="px-3 py-2 bg-slate-700 rounded" onClick={exportMarkdown}>Exportar Markdown</button>
      <button className="px-3 py-2 bg-slate-700 rounded" onClick={exportTxt}>Exportar TXT</button>
    </div>
  );
}
