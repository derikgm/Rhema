import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface LocalModelFile {
  filename: string;
  exists: boolean;
  size_bytes: number;
}

interface LocalModelStatus {
  id: string | null;
  folder_name: string;
  folder_path: string;
  name: string;
  description: string;
  language: string;
  streaming: boolean;
  files: LocalModelFile[];
  size_mb: number;
  complete: boolean;
  active: boolean;
}

interface ModelInfo {
  id: string;
  folder_name: string;
  name: string;
  description: string;
  language: string;
  streaming: boolean;
  files: Array<{ url: string; filename: string; size_mb: number }>;
  size_mb: number;
}

interface Progress {
  filename: string;
  downloaded_mb: number;
  total_mb: number;
  done: boolean;
}

interface SettingsPanelProps {
  open: boolean;
  onClose: () => void;
  voiceCommands: boolean;
  onVoiceCommandsChange: (v: boolean) => void;
  autoSave: boolean;
  onAutoSaveChange: (v: boolean) => void;
}

function Toggle({
  checked,
  onChange,
  label,
  description,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  description?: string;
}) {
  return (
    <label className="flex items-start gap-3 cursor-pointer py-2">
      <button
        type="button"
        onClick={() => onChange(!checked)}
        className="mt-0.5 relative w-10 h-6 rounded-full transition-colors shrink-0"
        style={{ backgroundColor: checked ? "var(--color-accent)" : "var(--color-border)" }}
        aria-pressed={checked}
      >
        <span
          className="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white transition-transform"
          style={{ transform: checked ? "translateX(16px)" : "translateX(0)" }}
        />
      </button>
      <span className="flex-1">
        <span className="block text-sm" style={{ color: "var(--color-text-primary)" }}>
          {label}
        </span>
        {description && (
          <span className="block text-xs mt-0.5" style={{ color: "var(--color-text-secondary)" }}>
            {description}
          </span>
        )}
      </span>
    </label>
  );
}

export function SettingsPanel({
  open,
  onClose,
  voiceCommands,
  onVoiceCommandsChange,
  autoSave,
  onAutoSaveChange,
}: SettingsPanelProps) {
  const [localModels, setLocalModels] = useState<LocalModelStatus[]>([]);
  const [catalog, setCatalog] = useState<ModelInfo[]>([]);
  const [progress, setProgress] = useState<Record<string, Progress>>({});
  const [activeFolder, setActiveFolder] = useState<string | null>(null);
  const [modelsDir, setModelsDir] = useState<string>("");
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string>("");

  const loadAll = useCallback(async () => {
    try {
      const [locals, cat, active, dir] = await Promise.all([
        invoke<LocalModelStatus[]>("list_models"),
        invoke<ModelInfo[]>("list_catalog"),
        invoke<string | null>("get_active_model_folder"),
        invoke<string>("models_dir_path"),
      ]);
      setLocalModels(locals);
      setCatalog(cat);
      setActiveFolder(active);
      setModelsDir(dir);
    } catch (e) {
      setMessage(`No se pudieron cargar los modelos: ${String(e)}`);
    }
  }, []);

  useEffect(() => {
    if (!open) return;
    loadAll();
  }, [open, loadAll]);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    const unlisten = listen<Progress>("model-download-progress", (e) => {
      setProgress((p) => ({ ...p, [e.payload.filename]: e.payload }));
    });
    return () => {
      unlisten.then((f) => f()).catch(() => {});
    };
  }, []);

  const handleDownload = async (m: ModelInfo) => {
    setBusy(m.id);
    setMessage(`Descargando ${m.name}…`);
    try {
      const path = await invoke<string>("download_model", { id: m.id });
      setMessage(`Descargado en ${path}`);
      await loadAll();
    } catch (e) {
      setMessage(`Error al descargar: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  };

  const handleDelete = async (folderName: string) => {
    setBusy(folderName);
    try {
      await invoke("delete_model_folder", { folderName });
      setMessage(`Modelo eliminado.`);
      if (activeFolder === folderName) setActiveFolder(null);
      await loadAll();
    } catch (e) {
      setMessage(`Error al eliminar: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  };

  const handleUse = async (folderName: string) => {
    try {
      await invoke("set_active_model_folder", { folderName });
      setActiveFolder(folderName);
      setMessage(`Modelo activo actualizado`);
      await loadAll();
    } catch (e) {
      setMessage(`Error al activar: ${String(e)}`);
    }
  };

  const handleImport = async () => {
    try {
      const paths = await invoke<string[]>("pick_file");
      if (!paths || paths.length === 0) {
        setMessage("No se seleccionaron archivos");
        return;
      }
      setBusy("import");
      setMessage("Importando modelo...");
      await invoke("import_model_files", { paths });
      setMessage("Modelo importado correctamente");
      await loadAll();
    } catch (e) {
      setMessage(`Error al importar: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  };

  const handleChangeDir = async () => {
    try {
      const dir = await invoke<string>("pick_folder");
      if (!dir) return;
      await invoke("set_models_dir", { dir });
      setMessage(`Carpeta de modelos actualizada`);
      await loadAll();
    } catch (e) {
      setMessage(`Error al cambiar carpeta: ${String(e)}`);
    }
  };

  if (!open) return null;

  const downloadedFolders = new Set(localModels.filter((l) => l.complete).map((l) => l.folder_name));

  return (
    <div className="fixed inset-0 z-[90] flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/60" onClick={onClose} />
      <div
        className="relative z-10 w-full max-w-2xl max-h-[90vh] rounded-2xl border flex flex-col overflow-hidden shadow-2xl"
        style={{ backgroundColor: "var(--color-bg-surface)", borderColor: "var(--color-border)" }}
      >
        <div className="flex items-center justify-between px-6 h-14 shrink-0 border-b" style={{ borderColor: "var(--color-border)" }}>
          <h2 className="text-base font-semibold" style={{ color: "var(--color-text-primary)" }}>Ajustes</h2>
          <button onClick={onClose} className="p-2 rounded-lg hover:opacity-80" style={{ color: "var(--color-text-secondary)" }} title="Cerrar">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div className="flex-1 overflow-y-auto px-6 py-5 space-y-8">
          <section>
            <div className="flex items-center justify-between mb-1">
              <h3 className="text-sm font-semibold" style={{ color: "var(--color-text-primary)" }}>Modelos de reconocimiento</h3>
              <div className="flex gap-2">
                <button onClick={handleImport} disabled={busy === "import"} className="text-xs px-2.5 py-1 rounded hover:opacity-80" style={{ backgroundColor: "var(--color-bg-elevated)", color: "var(--color-text-secondary)" }}>
                  {busy === "import" ? "Importando..." : "Importar"}
                </button>
                <button onClick={loadAll} className="text-xs px-2.5 py-1 rounded hover:opacity-80" style={{ backgroundColor: "var(--color-bg-elevated)", color: "var(--color-text-secondary)" }}>
                  Actualizar
                </button>
              </div>
            </div>
            <p className="text-xs mb-3" style={{ color: "var(--color-text-secondary)" }}>
              Los modelos se ejecutan 100% offline. Descarga desde catálogo o importa archivos locales.
            </p>

            <div className="space-y-3">
              {catalog.map((m) => {
                const downloaded = downloadedFolders.has(m.folder_name);
                const local = localModels.find((l) => l.folder_name === m.folder_name);
                const isActive = activeFolder === m.folder_name;
                const isBusy = busy === m.id;
                const parts = m.files.map((f) => progress[f.filename]).filter(Boolean);
                const downloadedMb = parts.reduce((s, p) => s + p.downloaded_mb, 0);
                const totalMb = parts.reduce((s, p) => s + p.total_mb, 0);
                const pct = totalMb > 0 ? Math.min(100, (downloadedMb / totalMb) * 100) : 0;
                const downloading = parts.length > 0 && parts.some((p) => !p.done);

                return (
                  <div key={m.id} className="rounded-xl border p-3.5" style={{ borderColor: isActive ? "var(--color-accent)" : "var(--color-border)", backgroundColor: "var(--color-bg-elevated)" }}>
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <div className="flex items-center gap-2 flex-wrap">
                          <span className="text-sm font-medium" style={{ color: "var(--color-text-primary)" }}>{m.name}</span>
                          {isActive && <span className="text-[10px] px-1.5 py-0.5 rounded-full" style={{ backgroundColor: "var(--color-success)", color: "#04222b" }}>ACTIVO</span>}
                          {downloaded && !isActive && <span className="text-[10px] px-1.5 py-0.5 rounded-full" style={{ backgroundColor: "var(--color-bg-surface)", color: "var(--color-text-secondary)" }}>DESCARGADO</span>}
                        </div>
                        <p className="text-xs mt-1" style={{ color: "var(--color-text-secondary)" }}>{m.description}</p>
                        <div className="text-xs mt-1 flex items-center gap-2 flex-wrap" style={{ color: "var(--color-text-secondary)" }}>
                          <span>{m.language}</span>
                          <span>·</span>
                          <span>~{m.size_mb.toFixed(0)} MB</span>
                        </div>
                      </div>
                      <div className="flex items-center gap-2 shrink-0">
                        {downloaded ? (
                          <>
                            <button onClick={() => handleUse(m.folder_name)} disabled={isActive} className="text-xs px-3 py-1.5 rounded transition-colors disabled:opacity-50" style={{ backgroundColor: isActive ? "var(--color-bg-surface)" : "var(--color-accent)", color: isActive ? "var(--color-text-secondary)" : "white" }}>
                              {isActive ? "En uso" : "Usar"}
                            </button>
                            <button onClick={() => handleDelete(m.folder_name)} disabled={isBusy} className="text-xs px-2.5 py-1.5 rounded transition-colors disabled:opacity-50" style={{ backgroundColor: "var(--color-bg-surface)", color: "var(--color-danger)" }} title="Eliminar modelo">
                              Eliminar
                            </button>
                          </>
                        ) : (
                          <button onClick={() => handleDownload(m)} disabled={isBusy || downloading} className="text-xs px-3 py-1.5 rounded transition-colors disabled:opacity-50" style={{ backgroundColor: "var(--color-accent)", color: "white" }}>
                            {downloading ? "Descargando…" : isBusy ? "Descargando…" : "Descargar"}
                          </button>
                        )}
                      </div>
                    </div>
                    {downloading && (
                      <div className="mt-3">
                        <div className="h-1.5 rounded-full overflow-hidden" style={{ backgroundColor: "var(--color-bg-surface)" }}>
                          <div className="h-full rounded-full transition-all" style={{ width: `${pct}%`, backgroundColor: "var(--color-accent)" }} />
                        </div>
                        <div className="text-[11px] mt-1 text-right" style={{ color: "var(--color-text-secondary)" }}>
                          {downloadedMb.toFixed(1)} / {totalMb.toFixed(1)} MB ({pct.toFixed(0)}%)
                        </div>
                      </div>
                    )}
                    {local && !local.complete && (
                      <div className="text-[11px] mt-2" style={{ color: "var(--color-danger)" }}>
                        Archivos incompletos en esta carpeta
                      </div>
                    )}
                  </div>
                );
              })}
            </div>

            {localModels.filter((l) => l.id === null).length > 0 && (
              <div className="mt-4">
                <h4 className="text-xs font-medium mb-2" style={{ color: "var(--color-text-primary)" }}>Modelos importados/locales</h4>
                <div className="space-y-2">
                  {localModels
                    .filter((l) => l.id === null)
                    .map((m) => (
                      <div key={m.folder_name} className="rounded-lg border p-2.5 flex items-center justify-between" style={{ borderColor: activeFolder === m.folder_name ? "var(--color-accent)" : "var(--color-border)", backgroundColor: "var(--color-bg-elevated)" }}>
                        <div>
                          <div className="flex items-center gap-2">
                            <span className="text-xs font-medium" style={{ color: "var(--color-text-primary)" }}>{m.name} ({m.folder_name})</span>
                            {activeFolder === m.folder_name && <span className="text-[9px] px-1 py-0.5 rounded-full" style={{ backgroundColor: "var(--color-success)", color: "#04222b" }}>ACTIVO</span>}
                          </div>
                          <div className="text-[10px]" style={{ color: "var(--color-text-secondary)" }}>{m.size_mb.toFixed(1)} MB {m.complete ? "· completo" : "· incompleto"}</div>
                        </div>
                        <div className="flex gap-1.5">
                          {m.complete && (
                            <button onClick={() => handleUse(m.folder_name)} disabled={activeFolder === m.folder_name} className="text-[10px] px-2 py-1 rounded disabled:opacity-50" style={{ backgroundColor: activeFolder === m.folder_name ? "var(--color-bg-surface)" : "var(--color-accent)", color: activeFolder === m.folder_name ? "var(--color-text-secondary)" : "white" }}>
                              Usar
                            </button>
                          )}
                          <button onClick={() => handleDelete(m.folder_name)} className="text-[10px] px-2 py-1 rounded" style={{ backgroundColor: "var(--color-bg-surface)", color: "var(--color-danger)" }}>
                            Eliminar
                          </button>
                        </div>
                      </div>
                    ))}
                </div>
              </div>
            )}

            <div className="mt-3 flex items-center justify-between">
              <button onClick={handleChangeDir} className="text-xs px-2.5 py-1 rounded hover:opacity-80" style={{ backgroundColor: "var(--color-bg-elevated)", color: "var(--color-text-secondary)" }}>
                Cambiar carpeta de modelos
              </button>
              {modelsDir && (
                <p className="text-[11px] break-all text-right max-w-[60%]" style={{ color: "var(--color-text-secondary)" }}>
                  {modelsDir}
                </p>
              )}
            </div>
          </section>

          <section>
            <h3 className="text-sm font-semibold mb-1" style={{ color: "var(--color-text-primary)" }}>Dictado</h3>
            <div className="divide-y" style={{ borderColor: "var(--color-border)" }}>
              <Toggle checked={voiceCommands} onChange={onVoiceCommandsChange} label="Comandos de voz" description='Convierte "coma", "punto" o "nueva línea" en su símbolo.' />
              <Toggle checked={autoSave} onChange={onAutoSaveChange} label="Autoguardado" description="Guarda la nota automáticamente cada 30 segundos." />
            </div>
          </section>

          <section className="rounded-xl border p-3.5" style={{ borderColor: "var(--color-border)", backgroundColor: "var(--color-bg-elevated)" }}>
            <h3 className="text-sm font-semibold mb-1" style={{ color: "var(--color-text-primary)" }}>Privacidad</h3>
            <p className="text-xs" style={{ color: "var(--color-text-secondary)" }}>
              Todo el audio y la transcripción se procesan en este dispositivo. No se envían datos a servidores externos ni se recopila telemetría.
            </p>
          </section>
        </div>

        {message && (
          <div className="px-6 py-3 shrink-0 border-t text-xs" style={{ borderColor: "var(--color-border)", color: "var(--color-text-secondary)" }}>
            {message}
          </div>
        )}
      </div>
    </div>
  );
}
