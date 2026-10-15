import { useState, MouseEvent as ReactMouseEvent } from "react";

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  children?: FileEntry[];
}

interface FolderTreeProps {
  entry: FileEntry;
  selectedPath: string | null;
  onSelect: (path: string, isDir: boolean) => void;
  onContextMenu?: (e: ReactMouseEvent, entry: FileEntry) => void;
  depth?: number;
}

function TreeItem({ entry, selectedPath, onSelect, onContextMenu, depth = 0 }: FolderTreeProps) {
  const [expanded, setExpanded] = useState(depth < 2);
  const isSelected = entry.path === selectedPath;
  const isDir = entry.is_dir;

  return (
    <div>
      <div
        className="flex items-center gap-1.5 py-1 px-2 rounded cursor-pointer text-sm transition-colors"
        style={{
          paddingLeft: `${depth * 16 + 8}px`,
          backgroundColor: isSelected ? "var(--color-accent-dim)" : "transparent",
          color: isSelected ? "var(--color-text-primary)" : "var(--color-text-secondary)",
        }}
        onClick={() => onSelect(entry.path, isDir)}
        onDoubleClick={() => isDir && setExpanded(!expanded)}
        onContextMenu={(e) => {
          if (onContextMenu) {
            e.preventDefault();
            onContextMenu(e, entry);
          }
        }}
      >
        {isDir ? (
          <>
            <span
              className="inline-block transition-transform text-xs"
              style={{
                transform: expanded ? "rotate(90deg)" : "rotate(0deg)",
                color: "var(--color-accent)",
              }}
            >
              ▶
            </span>
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="var(--color-accent)" stroke="none">
              <path d="M10 4H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-8l-2-2z" />
            </svg>
          </>
        ) : (
          <>
            <span className="inline-block w-3.5" />
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--color-text-secondary)" strokeWidth="2">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
            </svg>
          </>
        )}
        <span className="truncate">{entry.name}</span>
      </div>
      {isDir && expanded && entry.children?.map((child) => (
        <TreeItem
          key={child.path}
          entry={child}
          selectedPath={selectedPath}
          onSelect={onSelect}
          onContextMenu={onContextMenu}
          depth={depth + 1}
        />
      ))}
    </div>
  );
}

export function FolderTree(props: FolderTreeProps) {
  return (
    <div className="py-1">
      <TreeItem {...props} />
    </div>
  );
}
