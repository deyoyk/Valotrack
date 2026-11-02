import { useEffect, useMemo, useRef, useState } from "react";
import { CornerDownLeft, Search, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { cn } from "../lib/format";

export type CommandEntry = {
  id: string;
  label: string;
  group: string;
  hint?: string;
  icon: LucideIcon;
  run: () => void;
};

export function CommandPalette({
  open,
  entries,
  onClose,
}: {
  open: boolean;
  entries: CommandEntry[];
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [rendered, setRendered] = useState(open);
  const [closing, setClosing] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (open) {
      setRendered(true);
      setClosing(false);
      return;
    }
    if (!rendered) return;
    setClosing(true);
    const timer = window.setTimeout(() => {
      setRendered(false);
      setClosing(false);
    }, 150);
    return () => window.clearTimeout(timer);
  }, [open, rendered]);

  const rows = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return entries;
    return entries.filter((entry) =>
      `${entry.label} ${entry.group} ${entry.hint ?? ""}`.toLowerCase().includes(needle),
    );
  }, [entries, query]);

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setActive(0);
    const timer = window.setTimeout(() => inputRef.current?.focus(), 30);
    return () => window.clearTimeout(timer);
  }, [open]);

  useEffect(() => {
    setActive(0);
  }, [query]);

  useEffect(() => {
    if (!open) return;
    const current = listRef.current?.querySelector<HTMLElement>(".command-item.active");
    current?.scrollIntoView({ block: "nearest" });
  }, [active, open]);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, open]);

  if (!rendered) return null;

  function select(index: number) {
    const entry = rows[index];
    if (!entry) return;
    entry.run();
    onClose();
  }

  const groups = rows.reduce<Record<string, CommandEntry[]>>((acc, entry) => {
    acc[entry.group] = [...(acc[entry.group] ?? []), entry];
    return acc;
  }, {});

  return (
    <div
      className={cn("command-overlay", closing && "closing")}
      onMouseDown={onClose}
    >
      <div
        className={cn("command-panel", closing && "closing")}
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="command-search">
          <Search size={19} />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setActive((value) => Math.min(rows.length - 1, value + 1));
              }
              if (event.key === "ArrowUp") {
                event.preventDefault();
                setActive((value) => Math.max(0, value - 1));
              }
              if (event.key === "Enter") {
                event.preventDefault();
                select(active);
              }
            }}
            placeholder="Search pages, tools and actions"
            aria-label="Search commands"
          />
          <kbd>ESC</kbd>
          <button className="icon-button" onClick={onClose} aria-label="Close command palette">
            <X size={15} />
          </button>
        </div>

        <div className="command-list" ref={listRef}>
          {Object.entries(groups).map(([group, items]) => (
            <div key={group}>
              <div className="command-group-label">{group}</div>
              {items.map((entry) => {
                const Icon = entry.icon;
                const index = rows.indexOf(entry);
                return (
                  <button
                    key={entry.id}
                    className={cn("command-item", index === active && "active")}
                    onMouseEnter={() => setActive(index)}
                    onClick={() => select(index)}
                  >
                    <Icon size={18} />
                    <span>{entry.label}</span>
                    {entry.hint && <em>{entry.hint}</em>}
                    {index === active && <CornerDownLeft size={14} />}
                  </button>
                );
              })}
            </div>
          ))}
          {!rows.length && (
            <div className="command-empty">Nothing matches “{query}”. Try another word.</div>
          )}
        </div>

        <div className="command-foot">
          <span>
            <kbd>↑</kbd> <kbd>↓</kbd> navigate
          </span>
          <span>
            <kbd>Enter</kbd> open
          </span>
          <span>
            <kbd>Ctrl</kbd> + <kbd>K</kbd> anywhere
          </span>
        </div>
      </div>
    </div>
  );
}
