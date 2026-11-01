export type PromptOptions = {
  title: string;
  label?: string;
  defaultValue?: string;
  placeholder?: string;
  confirmLabel?: string;
};

export function promptText({
  title,
  label,
  defaultValue = "",
  placeholder,
  confirmLabel = "Save",
}: PromptOptions): Promise<string | null> {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "prompt-overlay";

    const card = document.createElement("div");
    card.className = "prompt-card";

    const heading = document.createElement("strong");
    heading.textContent = title;

    const field = document.createElement("label");
    field.className = "prompt-field";
    if (label) {
      const caption = document.createElement("span");
      caption.textContent = label;
      field.appendChild(caption);
    }

    const input = document.createElement("input");
    input.value = defaultValue;
    input.placeholder = placeholder ?? "";
    field.appendChild(input);

    const actions = document.createElement("div");
    actions.className = "prompt-actions";

    const cancel = document.createElement("button");
    cancel.className = "button";
    cancel.type = "button";
    cancel.textContent = "Cancel";

    const confirm = document.createElement("button");
    confirm.className = "button primary";
    confirm.type = "button";
    confirm.textContent = confirmLabel;

    actions.append(cancel, confirm);
    card.append(heading, field, actions);
    overlay.appendChild(card);
    document.body.appendChild(overlay);

    let closed = false;
    const close = (value: string | null) => {
      if (closed) return;
      closed = true;
      overlay.remove();
      resolve(value);
    };

    cancel.addEventListener("click", () => close(null));
    confirm.addEventListener("click", () => close(input.value.trim() || null));
    overlay.addEventListener("pointerdown", (event) => {
      if (event.target === overlay) close(null);
    });
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        close(input.value.trim() || null);
      }
      if (event.key === "Escape") {
        event.preventDefault();
        close(null);
      }
    });

    requestAnimationFrame(() => {
      input.focus();
      input.select();
    });
  });
}
