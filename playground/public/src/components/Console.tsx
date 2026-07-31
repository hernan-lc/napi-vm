import type { ConsoleLine } from "../hooks/useVm.ts";
import type { Translations } from "../i18n/translations.ts";

interface ConsoleProps {
  lines: ConsoleLine[];
  t: Translations;
}

export function Console({ lines, t }: ConsoleProps) {
  if (lines.length === 0) {
    return (
      <div class="console empty">
        <div class="placeholder">{t.emptyConsole}</div>
      </div>
    );
  }

  return (
    <div class="console">
      {lines.map((line) => (
        <div key={line.id} class={"line " + line.cls} dangerouslySetInnerHTML={{ __html: line.html }} />
      ))}
    </div>
  );
}
