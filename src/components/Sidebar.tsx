import { useState, MouseEvent as ReactMouseEvent } from "react";
import { FolderTree, FileEntry } from "./FolderTree";
import { ContextMenu, ContextMenuItem } from "./ContextMenu";
import { invoke } from "@tauri-apps/api/core";

interface SidebarProps {
  rootPath: string | null;
  rootTree: FileEntry | null;
  selectedPath: string | null;
  onSelectPath: (path: string, isDir: boolean) => void;
  onPickFolder: () => void;
  onRefresh: () => void;
  collapsed: boolean;
  onToggle: () => void;
}

interface CreatingState {
  type: "note" | "folder";
  parentDir: string;
}

interface ContextState {
  x: number;
  y: number;
  entry: FileEntry | null;
}

export function Sidebar({
  rootPath,
  rootTree,
  selectedPath,
  onSelectPath,
  onPickFolder,
  onRefresh,
  collapsed,
  onToggle,
}: SidebarProps) {
  const [newItemName, setNewItemName] = useState("");
  const [creating, setCreating] = useState<CreatingState | null>(null);
  const [context, setContext] = useState<ContextState | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<FileEntry | null>(null);

  const handleCreate = async () => {
    if (!creating || !newItemName.trim()) return;
    try {
      if (creating.type === "note") {
        const note = await invoke<{ path: string }>("create_note", {
          dirPath: creating.parentDir,
          name: newItemName.trim(),
        });
        onRefresh();
        onSelectPath(note.path, false);
      } else {
        await invoke("create_folder", {
          dirPath: creating.parentDir,
          name: newItemName.trim(),
        });
        onRefresh();
      }
      setNewItemName("");
      setCreating(null);
    } catch (e) {
      console.error(e);
      setNewItemName("");
      setCreating(null);
    }
  };

  const handleDelete = async () => {
    if (!confirmDelete) return;
    try {
      await invoke("delete_entry", { path: confirmDelete.path });
    } catch (e) {
      console.error(e);
    }
    setConfirmDelete(null);
    onRefresh();
  };

  const openBackgroundContext = (e: ReactMouseEvent) => {
    if (!rootPath) return;
    e.preventDefault();
    setContext({ x: e.clientX, y: e.clientY, entry: null });
  };

  const openItemContext = (e: ReactMouseEvent, entry: FileEntry) => {
    setContext({ x: e.clientX, y: e.clientY, entry });
  };

  const buildMenuItems = (ctx: ContextState): ContextMenuItem[] => {
    const items: ContextMenuItem[] = [];
    const parentDir = ctx.entry
      ? ctx.entry.is_dir
        ? ctx.entry.path
        : ctx.entry.path.replace(/[/\\][^/\\]*$/, "")
      : rootPath;

    const newNoteIcon = (
      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
      </svg>
    );
    const newFolderIcon = (
      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
      </svg>
    );
    const trashIcon = (
      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <polyline points="3 6 5 6 21 6" />
        <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
      </svg>
    );

    if (parentDir) {
      items.push({
        label: "Nueva nota",
        icon: newNoteIcon,
        onClick: () => setCreating({ type: "note", parentDir }),
      });
      items.push({
        label: "Nueva carpeta",
        icon: newFolderIcon,
        onClick: () => setCreating({ type: "folder", parentDir }),
      });
    }

    if (ctx.entry) {
      items.push({
        label: "Eliminar",
        icon: trashIcon,
        danger: true,
        onClick: () => setConfirmDelete(ctx.entry),
      });
    }

    return items;
  };

  if (collapsed) {
    return (
      <div
        className="flex flex-col items-center py-4 px-2 gap-3 h-full shrink-0"
        style={{ backgroundColor: "var(--color-bg-surface)", borderColor: "var(--color-border)" }}
      >
        <button
          onClick={onToggle}
          className="p-2 rounded-lg hover:opacity-80"
          style={{ color: "var(--color-text-secondary)" }}
          title="Expandir"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <rect x="3" y="3" width="18" height="18" rx="2" />
            <line x1="9" y1="3" x2="9" y2="21" />
          </svg>
        </button>
        <button
          onClick={onPickFolder}
          className="p-2 rounded-lg hover:opacity-80"
          style={{ color: "var(--color-accent)" }}
          title="Seleccionar carpeta"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
            <line x1="12" y1="11" x2="12" y2="17" />
            <line x1="9" y1="14" x2="15" y2="14" />
          </svg>
        </button>
      </div>
    );
  }

  return (
    <div
      className="flex flex-col h-full w-64 shrink-0 border-r"
      style={{ backgroundColor: "var(--color-bg-surface)", borderColor: "var(--color-border)" }}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-3 h-14 shrink-0 border-b" style={{ borderColor: "var(--color-border)" }}>
        <span className="text-sm font-medium truncate" style={{ color: "var(--color-text-primary)" }}>
          {rootPath ? rootPath.split(/[/\\]/).pop() : "Notas"}
        </span>
        <button
          onClick={onToggle}
          className="p-1.5 rounded hover:opacity-80 shrink-0"
          style={{ color: "var(--color-text-secondary)" }}
          title="Colapsar"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <rect x="3" y="3" width="18" height="18" rx="2" />
            <line x1="9" y1="3" x2="9" y2="21" />
          </svg>
        </button>
      </div>

      {/* Actions */}
      <div className="flex gap-2 px-3 py-2 shrink-0 border-b" style={{ borderColor: "var(--color-border)" }}>
        <button
          onClick={() => rootPath && setCreating({ type: "note", parentDir: rootPath })}
          disabled={!rootPath}
          className="flex-1 text-xs py-1.5 rounded transition-colors disabled:opacity-40"
          style={{ backgroundColor: "var(--color-accent-dim)", color: "white" }}
        >
          + Nota
        </button>
        <button
          onClick={() => rootPath && setCreating({ type: "folder", parentDir: rootPath })}
          disabled={!rootPath}
          className="flex-1 text-xs py-1.5 rounded transition-colors disabled:opacity-40"
          style={{ backgroundColor: "var(--color-bg-elevated)", color: "var(--color-text-secondary)" }}
        >
          + Carpeta
        </button>
      </div>

      {/* New item input */}
      {creating && (
        <div className="px-3 py-2 shrink-0 border-b" style={{ borderColor: "var(--color-border)" }}>
          <input
            type="text"
            value={newItemName}
            onChange={(e) => setNewItemName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleCreate();
              if (e.key === "Escape") { setCreating(null); setNewItemName(""); }
            }}
            placeholder={creating.type === "note" ? "nombre-nota" : "nombre-carpeta"}
            className="w-full text-xs px-2 py-1.5 rounded outline-none"
            style={{ backgroundColor: "var(--color-bg-primary)", color: "var(--color-text-primary)", border: "1px solid var(--color-border)" }}
            autoFocus
            onBlur={() => { if (!newItemName.trim()) { setCreating(null); } }}
          />
        </div>
      )}

      {/* Tree */}
      <div className="flex-1 overflow-y-auto py-1" onContextMenu={openBackgroundContext}>
        {rootTree ? (
          <FolderTree
            entry={rootTree}
            selectedPath={selectedPath}
            onSelect={onSelectPath}
            onContextMenu={openItemContext}
          />
        ) : (
          <div className="px-3 py-8 text-center">
            <p className="text-xs mb-3" style={{ color: "var(--color-text-secondary)" }}>
              Selecciona una carpeta para empezar
            </p>
            <button
              onClick={onPickFolder}
              className="text-xs px-3 py-1.5 rounded"
              style={{ backgroundColor: "var(--color-accent)", color: "white" }}
            >
              Elegir carpeta
            </button>
          </div>
        )}
      </div>

      {/* Context menu */}
      {context && (
        <ContextMenu
          x={context.x}
          y={context.y}
          items={buildMenuItems(context)}
          onClose={() => setContext(null)}
        />
      )}

      {/* Delete confirmation */}
      {confirmDelete && (
        <div className="fixed inset-0 z-[110] flex items-center justify-center">
          <div className="absolute inset-0 bg-black/50" onClick={() => setConfirmDelete(null)} />
          <div
            className="relative z-10 rounded-xl border p-5 w-80 max-w-[90vw] shadow-xl"
            style={{ backgroundColor: "var(--color-bg-surface)", borderColor: "var(--color-border)" }}
          >
            <h3 className="text-sm font-medium mb-2" style={{ color: "var(--color-text-primary)" }}>
              Eliminar {confirmDelete.is_dir ? "carpeta" : "nota"}
            </h3>
            <p className="text-xs mb-4 break-words" style={{ color: "var(--color-text-secondary)" }}>
              ¿Seguro que quieres eliminar <strong>{confirmDelete.name}</strong>?
              {confirmDelete.is_dir && " Se eliminará todo su contenido."} Esta acción no se puede deshacer.
            </p>
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setConfirmDelete(null)}
                className="text-xs px-3 py-1.5 rounded"
                style={{ backgroundColor: "var(--color-bg-elevated)", color: "var(--color-text-secondary)" }}
              >
                Cancelar
              </button>
              <button
                onClick={handleDelete}
                className="text-xs px-3 py-1.5 rounded"
                style={{ backgroundColor: "var(--color-danger)", color: "white" }}
              >
                Eliminar
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
