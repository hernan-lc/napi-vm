import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import { RESIZER } from "../constants.ts";

export function useResizable(
  minRatio = RESIZER.minRatio,
  maxRatio = RESIZER.maxRatio,
) {
  const [ratio, setRatio] = useState(() => {
    const stored = localStorage.getItem(RESIZER.storageKey);
    const value = stored ? parseFloat(stored) : RESIZER.defaultRatio;
    return Number.isFinite(value) ? Math.min(maxRatio, Math.max(minRatio, value)) : RESIZER.defaultRatio;
  });
  const containerRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);
  const frame = useRef<number | null>(null);
  const pendingRatio = useRef<number | null>(null);

  const handlePointerDown = useCallback((event: PointerEvent) => {
    event.preventDefault();
    dragging.current = true;
    document.body.style.cursor = "row-resize";
    document.body.style.userSelect = "none";
  }, []);

  useEffect(() => {
    const handlePointerMove = (event: PointerEvent) => {
      if (!dragging.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const availableHeight = Math.max(1, rect.height - RESIZER.dividerHeight);
      const pointerY = Math.min(availableHeight, Math.max(0, event.clientY - rect.top));
      pendingRatio.current = Math.min(maxRatio, Math.max(minRatio, pointerY / availableHeight));

      if (frame.current === null) {
        frame.current = requestAnimationFrame(() => {
          if (pendingRatio.current !== null) setRatio(pendingRatio.current);
          frame.current = null;
        });
      }
    };

    const stopDragging = () => {
      if (!dragging.current) return;
      dragging.current = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };

    document.addEventListener("pointermove", handlePointerMove);
    document.addEventListener("pointerup", stopDragging);
    document.addEventListener("pointercancel", stopDragging);
    return () => {
      document.removeEventListener("pointermove", handlePointerMove);
      document.removeEventListener("pointerup", stopDragging);
      document.removeEventListener("pointercancel", stopDragging);
      if (frame.current !== null) cancelAnimationFrame(frame.current);
    };
  }, [minRatio, maxRatio]);

  useEffect(() => {
    localStorage.setItem(RESIZER.storageKey, String(ratio));
  }, [ratio]);

  return { ratio, containerRef, handlePointerDown };
}
