import type { ConsoleLine } from "../hooks/useVm.ts";

interface ConsoleProps {
  lines: ConsoleLine[];
}

export function Console({ lines }: ConsoleProps) {
  if (lines.length === 0) {
    return (
      <div class="console empty">
        <div class="placeholder">Run code to see output here</div>
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
