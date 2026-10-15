import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ModelStatus {
  id: string;
  name: string;
  language: string;
  streaming: boolean;
  url: string;
  filename: string;
  size_mb: number;
  downloaded: boolean;
  active: boolean;
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
  const [models, setModels] = useState<ModelStatus[]>([]);
  const [progress, setProgress] = useState<Record<string, Progress>>({});
  const [activeModel, setActiveModel] = useState<string | null>(null);
  const [modelsDir, setModelsDir] = useState<string>("");
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string>("");

  const loadModels = useCallback(async () => {
    try {
      const [res, active, dir] = await Promise.all([
        invoke<ModelStatus[]>("list_models"),
        invoke<string | null>("get_active_model"),
        invoke<string>("models_dir_path"),
      ]);
      setModels(res);
      setActiveModel(active);
      setModelsDir(dir);
    } catch (e) {
      setMessage(`No se pudieron cargar los modelos: ${String(e)}`);
    }
  }, []);

  useEffect(() => {
    if (!open) return;
    loadModels();
  }, [open, loadModels]);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    const unlisten = listen<Progress>("model-download-progress", (e) => {
      setProgress((p) => ({ ...p, [e.payload.filename]: e.payload }));
    });
    return () => {
      unlisten.then((f) => f()).catch(() => {});
    };
  }, []);

  const handleDownload = async (m: ModelStatus) => {
    setBusy(m.filename);
    setMessage(`Descargando ${m.name}…`);
    try {
      const path = await invoke<string>("download_model", {
        url: m.url,
        filename: m.filename,
      });
      setMessage(`Descargado en ${path}`);
      await loadModels();
    } catch (e) {
      setMessage(`Error al descargar: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  };

  const handleDelete = async (m: ModelStatus) => {
    setBusy(m.filename);
    try {
      await invoke("delete_model", { filename: m.filename });
      setMessage(`Modelo ${m.name} eliminado.`);
      if (activeModel === m.id) setActiveModel(null);
      await loadModels();
    } catch (e) {
      setMessage(`Error al eliminar: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  };

  const handleUse = async (m: ModelStatus) => {
    try {
      await invoke("set_active_model", { id: m.id });
      setActiveModel(m.id);
      setMessage(`Modelo activo: ${m.name}`);
    } catch (e) {
      setMessage(`Error al activar: ${String(e)}`);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[90] flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/60" onClick={onClose} />

      <div
        className="relative z-10 w-full max-w-2xl max-h-[90vh] rounded-2xl border flex flex-col overflow-hidden shadow-2xl"
        style={{ backgroundColor: "var(--color-bg-surface)", borderColor: "var(--color-border)" }}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between px-6 h-14 shrink-0 border-b"
          style={{ borderColor: "var(--color-border)" }}
        >
          <h2 className="text-base font-semibold" style={{ color: "var(--color-text-primary)" }}>
            Ajustes
          </h2>
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:opacity-80"
            style={{ color: "var(--color-text-secondary)" }}
            title="Cerrar"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto px-6 py-5 space-y-8">
          {/* Modelos */}
          <section>
            <div className="flex items-center justify-between mb-1">
              <h3 className="text-sm font-semibold" style={{ color: "var(--color-text-primary)" }}>
                Modelos de reconocimiento
              </h3>
              <button
                onClick={loadModels}
                className="text-xs px-2.5 py-1 rounded hover:opacity-80"
                style={{ backgroundColor: "var(--color-bg-elevated)", color: "var(--color-text-secondary)" }}
              >
                Actualizar
              </button>
            </div>
            <p className="text-xs mb-3" style={{ color: "var(--color-text-secondary)" }}>
              Los modelos se descargan y ejecutan localmente. Elige uno y actívalo para el dictado.
            </p>

            <div className="space-y-3">
              {models.length === 0 && (
                <p className="text-xs" style={{ color: "var(--color-text-secondary)" }}>
                  No se encontraron modelos.
                </p>
              )}

              {models.map((m) => {
                const p = progress[m.filename];
                const pct = p && p.total_mb > 0 ? Math.min(100, (p.downloaded_mb / p.total_mb) * 100) : 0;
                const isActive = activeModel === m.id;
                const isBusy = busy === m.filename;

                return (
                  <div
                    key={m.id}
                    className="rounded-xl border p-3.5"
                    style={{
                      borderColor: isActive ? "var(--color-accent)" : "var(--color-border)",
                      backgroundColor: "var(--color-bg-elevated)",
                    }}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <div className="flex items-center gap-2 flex-wrap">
                          <span className="text-sm font-medium" style={{ color: "var(--color-text-primary)" }}>
                            {m.name}
                          </span>
                          {isActive && (
                            <span
                              className="text-[10px] px-1.5 py-0.5 rounded-full"
                              style={{ backgroundColor: "var(--color-success)", color: "#04222b" }}
                            >
                              ACTIVO
                            </span>
                          )}
                        </div>
                        <div className="text-xs mt-1 flex items-center gap-2 flex-wrap" style={{ color: "var(--color-text-secondary)" }}>
                          <span>{m.language}</span>
                          <span>·</span>
                          <span>{m.streaming ? "streaming" : "offline"}</span>
                          <span>·</span>
                          <span>~{m.size_mb} MB</span>
                          {m.downloaded && (
                            <>
                              <span>·</span>
                              <span style={{ color: "var(--color-success)" }}>descargado</span>
                            </>
                          )}
                        </div>
                      </div>

                      <div className="flex items-center gap-2 shrink-0">
                        {m.downloaded ? (
                          <>
                            <button
                              onClick={() => handleUse(m)}
                              disabled={isActive}
                              className="text-xs px-3 py-1.5 rounded transition-colors disabled:opacity-50"
                              style={{
                                backgroundColor: isActive ? "var(--color-bg-surface)" : "var(--color-accent)",
                                color: isActive ? "var(--color-text-secondary)" : "white",
                              }}
                            >
                              {isActive ? "En uso" : "Usar"}
                            </button>
                            <button
                              onClick={() => handleDelete(m)}
                              disabled={isBusy}
                              className="text-xs px-2.5 py-1.5 rounded transition-colors disabled:opacity-50"
                              style={{ backgroundColor: "var(--color-bg-surface)", color: "var(--color-danger)" }}
                              title="Eliminar modelo"
                            >
                              Eliminar
                            </button>
                          </>
                        ) : (
                          <button
                            onClick={() => handleDownload(m)}
                            disabled={isBusy}
                            className="text-xs px-3 py-1.5 rounded transition-colors disabled:opacity-50"
                            style={{ backgroundColor: "var(--color-accent)", color: "white" }}
                          >
                            {isBusy ? "Descargando…" : "Descargar"}
                          </button>
                        )}
                      </div>
                    </div>

                    {p && !p.done && (
                      <div className="mt-3">
                        <div className="h-1.5 rounded-full overflow-hidden" style={{ backgroundColor: "var(--color-bg-surface)" }}>
                          <div
                            className="h-full rounded-full transition-all"
                            style={{ width: `${pct}%`, backgroundColor: "var(--color-accent)" }}
                          />
                        </div>
                        <div className="text-[11px] mt-1 text-right" style={{ color: "var(--color-text-secondary)" }}>
                          {p.downloaded_mb.toFixed(1)} / {p.total_mb.toFixed(1)} MB ({pct.toFixed(0)}%)
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>

            {modelsDir && (
              <p className="text-[11px] mt-3 break-all" style={{ color: "var(--color-text-secondary)" }}>
                Carpeta de modelos: {modelsDir}
              </p>
            )}
          </section>

          {/* Dictado */}
          <section>
            <h3 className="text-sm font-semibold mb-1" style={{ color: "var(--color-text-primary)" }}>
              Dictado
            </h3>
            <div className="divide-y" style={{ borderColor: "var(--color-border)" }}>
              <Toggle
                checked={voiceCommands}
                onChange={onVoiceCommandsChange}
                label="Comandos de voz"
                description='Convierte "coma", "punto" o "nueva línea" en su símbolo.'
              />
              <Toggle
                checked={autoSave}
                onChange={onAutoSaveChange}
                label="Autoguardado"
                description="Guarda la nota automáticamente cada 30 segundos."
              />
            </div>
          </section>

          {/* Privacidad */}
          <section
            className="rounded-xl border p-3.5"
            style={{ borderColor: "var(--color-border)", backgroundColor: "var(--color-bg-elevated)" }}
          >
            <h3 className="text-sm font-semibold mb-1" style={{ color: "var(--color-text-primary)" }}>
              Privacidad
            </h3>
            <p className="text-xs" style={{ color: "var(--color-text-secondary)" }}>
              Todo el audio y la transcripción se procesan en este dispositivo. No se envían datos a
              servidores externos ni se recopila telemetría.
            </p>
          </section>
        </div>

        {/* Footer / status */}
        {message && (
          <div
            className="px-6 py-3 shrink-0 border-t text-xs"
            style={{ borderColor: "var(--color-border)", color: "var(--color-text-secondary)" }}
          >
            {message}
          </div>
        )}
      </div>
    </div>
  );
}
