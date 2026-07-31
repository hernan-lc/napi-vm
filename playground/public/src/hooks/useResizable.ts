import { useCallback, useEffect, useRef, useState } from "preact/hooks";

const STORAGE_KEY = "napi-vm-panel-ratio";

export function useResizable(minRatio = 0.2, maxRatio = 0.8) {
  const [ratio, setRatio] = useState(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    const v = stored ? parseFloat(stored) : 0.55;
    return Math.min(maxRatio, Math.max(minRatio, v));
  });
  const containerRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);

  const handleMousedown = useCallback((e: MouseEvent) => {
    e.preventDefault();
    dragging.current = true;
    document.body.style.cursor = "row-resize";
    document.body.style.userSelect = "none";
  }, []);

  useEffect(() => {
    const handleMousemove = (e: MouseEvent) => {
      if (!dragging.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const y = e.clientY - rect.top;
      const r = y / rect.height;
      const clamped = Math.min(maxRatio, Math.max(minRatio, r));
      setRatio(clamped);
    };

    const handleMouseup = () => {
      if (!dragging.current) return;
      dragging.current = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };

    document.addEventListener("mousemove", handleMousemove);
    document.addEventListener("mouseup", handleMouseup);
    return () => {
      document.removeEventListener("mousemove", handleMousemove);
      document.removeEventListener("mouseup", handleMouseup);
    };
  }, [minRatio, maxRatio]);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, String(ratio));
  }, [ratio]);

  return { ratio, containerRef, handleMousedown };
}
