import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Sidebar } from "./components/Sidebar";
import { FileEntry } from "./components/FolderTree";
import { SettingsPanel } from "./components/SettingsPanel";
import { applyVoicePunctuation } from "./utils/postprocessPunctuation";

type EditorState = "idle" | "listening" | "loading";

interface NoteData {
  name: string;
  path: string;
  content: string;
}

function App() {
  const [text, setText] = useState("");
  const [editorState, setEditorState] = useState<EditorState>("idle");
  const [showOptions, setShowOptions] = useState(false);
  const [voiceCommands, setVoiceCommands] = useState(true);
  const [autoSave, setAutoSave] = useState(true);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  // Folder state
  const [rootPath, setRootPath] = useState<string | null>(null);
  const [rootTree, setRootTree] = useState<FileEntry | null>(null);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [currentNote, setCurrentNote] = useState<NoteData | null>(null);
  const [isMobile, setIsMobile] = useState(false);

  // Save state
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
  const [lastSavedAt, setLastSavedAt] = useState<Date | null>(null);

  // Dictado
  const [micError, setMicError] = useState("");
  const [audioLevel, setAudioLevel] = useState(0);

  // Responsive detection
  useEffect(() => {
    const checkMobile = () => {
      const mobile = window.innerWidth < 768;
      setIsMobile(mobile);
      if (mobile) setSidebarCollapsed(true);
    };
    checkMobile();
    window.addEventListener("resize", checkMobile);
    return () => window.removeEventListener("resize", checkMobile);
  }, [sidebarCollapsed]);

  // Load note when selected
  useEffect(() => {
    if (!selectedPath) return;
    const loadNote = async () => {
      try {
        const note = await invoke<NoteData>("read_note", { path: selectedPath });
        setCurrentNote(note);
        setText(note.content);
        setHasUnsavedChanges(false);
      } catch {
        setCurrentNote(null);
        setText("");
        setHasUnsavedChanges(false);
      }
    };
    if (!selectedPath.endsWith("/") && !selectedPath.endsWith("\\")) {
      loadNote();
    }
  }, [selectedPath]);

  // Mark as dirty when text changes
  useEffect(() => {
    if (currentNote && text !== currentNote.content) {
      setHasUnsavedChanges(true);
    }
  }, [text, currentNote]);

  // Save function
  const saveNote = useCallback(async () => {
    if (!currentNote) return;
    try {
      await invoke("save_note", { path: currentNote.path, content: text });
      setCurrentNote((prev) => (prev ? { ...prev, content: text } : prev));
      setHasUnsavedChanges(false);
      setLastSavedAt(new Date());
    } catch (e) {
      console.error("Failed to save:", e);
    }
  }, [currentNote, text]);

  // Auto-save
  useEffect(() => {
    if (!autoSave || !currentNote || !hasUnsavedChanges) return;
    const timer = setTimeout(() => {
      saveNote();
    }, 30000);
    return () => clearTimeout(timer);
  }, [text, autoSave, currentNote, hasUnsavedChanges, saveNote]);

  // Save on note switch
  useEffect(() => {
    const handleBeforeUnload = () => {
      if (currentNote && hasUnsavedChanges) {
        invoke("save_note", { path: currentNote.path, content: text }).catch(console.error);
      }
    };
    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => window.removeEventListener("beforeunload", handleBeforeUnload);
  }, [currentNote, text, hasUnsavedChanges]);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === "s") {
        e.preventDefault();
        saveNote();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [saveNote]);

  const handlePickFolder = async () => {
    try {
      const path = await invoke<string>("pick_folder");
      setRootPath(path);
      const tree = await invoke<FileEntry>("read_folder_tree", { path, depth: 10 });
      setRootTree(tree);
      setMobileMenuOpen(false);
    } catch (e) {
      console.error("Failed to pick folder:", e);
    }
  };

  const refreshTree = useCallback(async () => {
    if (!rootPath) return;
    try {
      const tree = await invoke<FileEntry>("read_folder_tree", { path: rootPath, depth: 10 });
      setRootTree(tree);
    } catch (e) {
      console.error("Failed to refresh tree:", e);
    }
  }, [rootPath]);

  const handleSelectPath = useCallback(async (path: string, isDir: boolean) => {
    // Save current note before switching
    if (currentNote && hasUnsavedChanges) {
      await invoke("save_note", { path: currentNote.path, content: text }).catch(console.error);
      setHasUnsavedChanges(false);
    }

    setSelectedPath(path);
    if (isDir) {
      if (path === rootPath) {
        const tree = await invoke<FileEntry>("read_folder_tree", { path, depth: 10 });
        setRootTree(tree);
      }
    }
    if (isMobile) setMobileMenuOpen(false);
  }, [rootPath, isMobile, currentNote, hasUnsavedChanges, text]);

  // Transcripción en vivo (eventos emitidos por el backend)
  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let cancelled = false;
    const unsubs: Array<() => void> = [];

    listen<string>("final-transcript", (e) => {
      const raw = voiceCommands ? applyVoicePunctuation(e.payload) : e.payload;
      const clean = raw.trim();
      if (!clean) return;
      setText((prev) => {
        const base = prev.replace(/\s+$/, "");
        return base ? `${base} ${clean}` : clean;
      });
      setHasUnsavedChanges(true);
    }).then((u) => (cancelled ? u() : unsubs.push(u)));

    listen<number>("audio-level", (e) => setAudioLevel(e.payload)).then((u) =>
      cancelled ? u() : unsubs.push(u),
    );

    return () => {
      cancelled = true;
      unsubs.forEach((u) => u());
    };
  }, [voiceCommands]);

  const toggleListening = useCallback(async () => {
    if (editorState === "listening" || editorState === "loading") {
      setEditorState("idle");
      setAudioLevel(0);
      try {
        await invoke("stop_listening");
      } catch (e) {
        console.error(e);
      }
      try {
        await invoke("stop_asr");
      } catch (e) {
        console.error(e);
      }
      return;
    }

    setMicError("");
    setEditorState("loading");
    try {
      await invoke("start_asr");
      await invoke("start_listening");
      setEditorState("listening");
    } catch (e) {
      setMicError(typeof e === "string" ? e : String(e));
      setEditorState("idle");
      try {
        await invoke("stop_listening");
      } catch {
        /* noop */
      }
      try {
        await invoke("stop_asr");
      } catch {
        /* noop */
      }
    }
  }, [editorState]);

  // Atajo Ctrl+Space (usa un ref para no re-suscribir en cada render)
  const toggleRef = useRef(toggleListening);
  useEffect(() => {
    toggleRef.current = toggleListening;
  }, [toggleListening]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.code === "Space") {
        e.preventDefault();
        toggleRef.current();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const wordCount = text.trim() ? text.trim().split(/\s+/).length : 0;
  const charCount = text.length;

  const formatTime = (date: Date) =>
    date.toLocaleTimeString("es", { hour: "2-digit", minute: "2-digit" });

  return (
    <div className="h-screen flex overflow-hidden" style={{ backgroundColor: "var(--color-bg-primary)" }}>
      {/* Desktop Sidebar */}
      {!isMobile && (
        <Sidebar
          rootPath={rootPath}
          rootTree={rootTree}
          selectedPath={selectedPath}
          onSelectPath={handleSelectPath}
          onPickFolder={handlePickFolder}
          onRefresh={refreshTree}
          collapsed={sidebarCollapsed}
          onToggle={() => setSidebarCollapsed(!sidebarCollapsed)}
        />
      )}

      {/* Mobile Drawer */}
      {isMobile && mobileMenuOpen && (
        <div className="fixed inset-0 z-50 flex">
          <div className="absolute inset-0 bg-black/50" onClick={() => setMobileMenuOpen(false)} />
          <div className="relative z-10 flex h-full">
            <Sidebar
              rootPath={rootPath}
              rootTree={rootTree}
              selectedPath={selectedPath}
              onSelectPath={handleSelectPath}
              onPickFolder={handlePickFolder}
              onRefresh={refreshTree}
              collapsed={false}
              onToggle={() => setMobileMenuOpen(false)}
            />
          </div>
        </div>
      )}

      {/* Main Content */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Header */}
        <header
          className="flex items-center justify-between px-4 md:px-6 h-14 border-b shrink-0"
          style={{ borderColor: "var(--color-border)", backgroundColor: "var(--color-bg-surface)" }}
        >
          <div className="flex items-center gap-3 min-w-0">
            {isMobile && (
              <button
                onClick={() => setMobileMenuOpen(true)}
                className="p-2 rounded-lg hover:opacity-80 shrink-0"
                style={{ color: "var(--color-text-secondary)" }}
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <line x1="3" y1="12" x2="21" y2="12" />
                  <line x1="3" y1="6" x2="21" y2="6" />
                  <line x1="3" y1="18" x2="21" y2="18" />
                </svg>
              </button>
            )}
            <div className="flex items-center gap-3 min-w-0">
              <h1 className="text-lg font-semibold shrink-0" style={{ color: "var(--color-text-primary)" }}>
                Rhema
              </h1>
              <span
                className="text-xs px-2 py-0.5 rounded-full hidden sm:inline-block shrink-0"
                style={{ backgroundColor: "var(--color-bg-elevated)", color: "var(--color-text-secondary)" }}
              >
                Offline
              </span>
              {currentNote && (
                <span className="text-xs truncate hidden md:inline-block" style={{ color: "var(--color-text-secondary)" }}>
                  / {currentNote.name}
                </span>
              )}
            </div>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            {/* Save status indicator */}
            {currentNote && (
              <div className="flex items-center gap-2 mr-2">
                {hasUnsavedChanges ? (
                  <div className="flex items-center gap-1.5">
                    <span className="w-2 h-2 rounded-full" style={{ backgroundColor: "var(--color-danger)" }} />
                    <span className="text-xs hidden sm:inline" style={{ color: "var(--color-danger)" }}>
                      Sin guardar
                    </span>
                  </div>
                ) : lastSavedAt ? (
                  <div className="flex items-center gap-1.5">
                    <span className="w-2 h-2 rounded-full" style={{ backgroundColor: "var(--color-success)" }} />
                    <span className="text-xs hidden sm:inline" style={{ color: "var(--color-text-secondary)" }}>
                      Guardado {formatTime(lastSavedAt)}
                    </span>
                  </div>
                ) : null}
              </div>
            )}

            <button
              onClick={saveNote}
              disabled={!currentNote || !hasUnsavedChanges}
              className="px-3 py-1.5 rounded text-xs transition-colors disabled:opacity-40"
              style={{
                backgroundColor: hasUnsavedChanges ? "var(--color-accent)" : "var(--color-bg-elevated)",
                color: hasUnsavedChanges ? "white" : "var(--color-text-secondary)",
              }}
              title="Guardar (Ctrl+S)"
            >
              Guardar
            </button>

            <button
              onClick={() => setShowOptions(true)}
              className="p-2 rounded-lg transition-colors hover:opacity-80"
              style={{ color: "var(--color-text-secondary)" }}
              title="Ajustes"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
              </svg>
            </button>
          </div>
        </header>

        {/* Editor Area */}
        <main className="flex-1 flex flex-col items-center px-4 md:px-6 py-6 md:py-8 gap-6 overflow-y-auto">
          {/* Editor */}
          <div
            className="w-full max-w-3xl rounded-xl border p-4 flex-1 min-h-[300px]"
            style={{ borderColor: "var(--color-border)", backgroundColor: "var(--color-bg-surface)" }}
          >
            <textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder={currentNote ? "Escribe aquí..." : "Selecciona o crea una nota para empezar..."}
              className="w-full h-full bg-transparent resize-none outline-none text-base leading-relaxed min-h-[260px]"
              style={{ color: "var(--color-text-primary)" }}
              readOnly={!currentNote}
            />
          </div>

          {/* Controls */}
          <div className="flex flex-col items-center gap-4 w-full max-w-3xl">
            <div className="flex items-center gap-6">
              {/* Mic Button */}
              <button
                onClick={toggleListening}
                disabled={editorState === "loading"}
                className="relative w-16 h-16 rounded-full flex items-center justify-center transition-all hover:scale-105 active:scale-95 disabled:opacity-70 disabled:cursor-wait"
                style={{
                  backgroundColor:
                    editorState === "listening"
                      ? "var(--color-danger)"
                      : editorState === "loading"
                        ? "var(--color-bg-elevated)"
                        : "var(--color-accent)",
                }}
                title={
                  editorState === "idle"
                    ? "Iniciar dictado (Ctrl+Space)"
                    : editorState === "loading"
                      ? "Cargando modelo…"
                      : "Detener dictado"
                }
              >
                {editorState === "listening" && (
                  <span
                    className="absolute inset-0 rounded-full animate-pulse-recording"
                    style={{ backgroundColor: "var(--color-danger)", opacity: 0.3 }}
                  />
                )}
                {editorState === "idle" && (
                  <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
                    <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
                    <line x1="12" x2="12" y1="19" y2="22" />
                  </svg>
                )}
                {editorState === "listening" && (
                  <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="white" stroke="white" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <rect x="6" y="6" width="12" height="12" rx="2" />
                  </svg>
                )}
                {editorState === "loading" && (
                  <svg className="animate-spin" xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="var(--color-accent-bright)" strokeWidth="2" strokeLinecap="round">
                    <path d="M21 12a9 9 0 1 1-6.219-8.56" />
                  </svg>
                )}
              </button>

              {/* Recording indicator + nivel de entrada */}
              {editorState === "listening" && (
                <div className="flex items-center gap-3">
                  <span
                    className="w-2.5 h-2.5 rounded-full animate-pulse-recording"
                    style={{ backgroundColor: "var(--color-success)" }}
                  />
                  <div className="flex flex-col gap-1.5">
                    <span className="text-sm" style={{ color: "var(--color-text-secondary)" }}>
                      Escuchando...
                    </span>
                    <div
                      className="w-40 h-1.5 rounded-full overflow-hidden"
                      style={{ backgroundColor: "var(--color-bg-elevated)" }}
                    >
                      <div
                        className="h-full rounded-full transition-all duration-75"
                        style={{
                          width: `${Math.round(audioLevel * 100)}%`,
                          backgroundColor: "var(--color-success)",
                        }}
                      />
                    </div>
                  </div>
                </div>
              )}

              {editorState === "loading" && (
                <span className="text-sm" style={{ color: "var(--color-text-secondary)" }}>
                  Cargando modelo…
                </span>
              )}
            </div>

            {micError && (
              <div
                className="w-full text-xs px-3 py-2 rounded-lg border break-words"
                style={{
                  borderColor: "var(--color-danger)",
                  color: "var(--color-danger)",
                  backgroundColor: "rgba(239,68,68,0.08)",
                }}
              >
                No se pudo iniciar el micrófono: {micError}
              </div>
            )}
          </div>

          {/* Status Bar */}
          <div className="flex gap-6 text-xs" style={{ color: "var(--color-text-secondary)" }}>
            <span>{wordCount} palabras</span>
            <span>{charCount} caracteres</span>
            <span className="opacity-60 hidden sm:inline">Ctrl+Space para dictar</span>
            <span className="opacity-60 hidden sm:inline">Ctrl+S para guardar</span>
          </div>
        </main>
      </div>

      {/* Settings Panel */}
      <SettingsPanel
        open={showOptions}
        onClose={() => setShowOptions(false)}
        voiceCommands={voiceCommands}
        onVoiceCommandsChange={setVoiceCommands}
        autoSave={autoSave}
        onAutoSaveChange={setAutoSave}
      />
    </div>
  );
}

export default App;
