import { type ReactNode, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { cn } from "../lib/format";

export function Modal({
  children,
  onClose,
  className,
  label,
}: {
  children: ReactNode;
  onClose: () => void;
  className?: string;
  label: string;
}) {
  const [closing, setClosing] = useState(false);
  const timer = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (timer.current) window.clearTimeout(timer.current);
    };
  }, []);

  function close() {
    if (closing) return;
    setClosing(true);
    timer.current = window.setTimeout(onClose, 180);
  }

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") close();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [closing]);

  return createPortal(
    <div
      className={cn("modal-backdrop", closing && "closing")}
      onMouseDown={close}
      role="dialog"
      aria-label={label}
    >
      <div
        className={cn("modal-panel", closing && "closing", className)}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <button className="modal-close" onClick={close} aria-label="Close">
          <X size={16} />
        </button>
        {children}
      </div>
    </div>,
    document.body,
  );
}
