import { useEffect, useRef, ReactNode } from "react";

export interface ContextMenuItem {
  label: string;
  onClick: () => void;
  danger?: boolean;
  icon?: ReactNode;
}

interface ContextMenuProps {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}

export function ContextMenu({ x, y, items, onClose }: ContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleMouseDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", handleMouseDown);
    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("mousedown", handleMouseDown);
      window.removeEventListener("keydown", handleKey);
    };
  }, [onClose]);

  // Keep the menu inside the viewport
  const menuWidth = 190;
  const menuHeight = items.length * 36 + 8;
  const adjustedX = Math.min(x, window.innerWidth - menuWidth - 8);
  const adjustedY = Math.min(y, window.innerHeight - menuHeight - 8);

  return (
    <div
      ref={ref}
      className="fixed z-[100] rounded-lg border py-1 shadow-xl min-w-[190px]"
      style={{
        left: adjustedX,
        top: adjustedY,
        backgroundColor: "var(--color-bg-elevated)",
        borderColor: "var(--color-border)",
      }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {items.map((item, i) => (
        <button
          key={i}
          onClick={() => {
            item.onClick();
            onClose();
          }}
          className="w-full text-left px-3 py-2 text-sm flex items-center gap-2.5 transition-colors hover:bg-white/5"
          style={{ color: item.danger ? "var(--color-danger)" : "var(--color-text-primary)" }}
        >
          {item.icon}
          {item.label}
        </button>
      ))}
    </div>
  );
}
