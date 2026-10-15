import { useEffect, useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import ModelPicker from "@/components/ModelPicker";
import Editor from "@/components/Editor";
import ExportButtons from "@/components/ExportButtons";
import FileTree from "@/components/FileTree";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [text, setText] = useState("");
  const [currentFile, setCurrentFile] = useState("");
  const [autoSaveFile, setAutoSaveFile] = useState(false);

  useEffect(() => {
    if (!autoSaveFile || !currentFile) return;
    const id = setInterval(() => {
      invoke("save_text", { path: currentFile, content: text }).catch(() => {});
    }, 3000);
    return () => clearInterval(id);
  }, [autoSaveFile, currentFile, text]);
  const [voicePunctuation, setVoicePunctuation] = useState(false);

  async function greet() {
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <main className="min-h-screen bg-slate-900 text-white flex flex-col items-center justify-center gap-6 p-8">
      <h1 className="text-3xl font-bold">Welcome to Tauri + React + Tailwind v4</h1>
      <div className="flex gap-6">
        <a href="https://vite.dev" target="_blank">
          <img src="/vite.svg" className="w-16 hover:opacity-80" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="w-16 hover:opacity-80" alt="Tauri logo" />
        </a>
        <a href="https://react.dev" target="_blank">
          <img src={reactLogo} className="w-16 hover:opacity-80" alt="React logo" />
        </a>
      </div>
      <p className="text-slate-300">Click on the Tauri, Vite, and React logos to learn more.</p>
      <form className="flex gap-2" onSubmit={(e) => { e.preventDefault(); greet(); }}>
        <input
          id="greet-input"
          className="px-3 py-2 rounded bg-slate-800 border border-slate-700"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
        />
        <button type="submit" className="px-4 py-2 rounded bg-indigo-500 hover:bg-indigo-400">
          Greet
        </button>
      </form>
      <p>{greetMsg}</p>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="md:col-span-1"><FileTree onOpen={(p, t) => { setCurrentFile(p); setText(t); }} /></div>
        <div className="md:col-span-2 space-y-3">
          {currentFile && <div className="text-xs text-slate-400 truncate">Archivo: {currentFile}</div>}
          {currentFile && (
            <div className="flex items-center gap-4 text-sm text-slate-300">
              <label className="flex items-center gap-2"><input type="checkbox" checked={autoSaveFile} onChange={(e) => setAutoSaveFile(e.target.checked)} /> Autoguardar archivo</label>
              <button className="px-3 py-1 bg-slate-700 rounded" onClick={() => invoke("save_text", { path: currentFile, content: text })}>Guardar ahora</button>
            </div>
          )}
          <Editor value={text} onChange={setText} voicePunctuation={voicePunctuation} />
        </div>
      </div>
      <ExportButtons text={text} />
      <label className="flex items-center gap-2 text-slate-300 text-sm">
        <input type="checkbox" checked={voicePunctuation} onChange={(e) => setVoicePunctuation(e.target.checked)} />
        Puntuación por voz (coma, punto, nueva línea...)
      </label>
      <ExportButtons text={text} />
      <ModelPicker />
    </main>
  );
}

export default App;
