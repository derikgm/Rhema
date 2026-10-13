import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

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
    </main>
  );
}

export default App;
