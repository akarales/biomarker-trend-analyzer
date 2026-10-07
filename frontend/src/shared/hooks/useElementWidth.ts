import { useEffect, useRef, useState } from 'react';

/**
 * Track an element's content width (ResizeObserver, rAF-throttled) so SVG
 * charts can lay out in real pixels — text stays at its CSS size on phones
 * instead of shrinking with a scaled viewBox. Falls back to `fallback`
 * where ResizeObserver is missing (jsdom).
 */
export function useElementWidth<T extends HTMLElement>(fallback = 880) {
  const ref = useRef<T>(null);
  const [width, setWidth] = useState(fallback);
  useEffect(() => {
    const el = ref.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    let frame = 0;
    const observer = new ResizeObserver(([entry]) => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const w = Math.round(entry.contentRect.width);
        if (w > 0) setWidth(w);
      });
    });
    observer.observe(el);
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, []);
  return { ref, width };
}
