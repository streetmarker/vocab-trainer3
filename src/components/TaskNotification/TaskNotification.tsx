// src/components/TaskNotification/TaskNotification.tsx
//
// Toast-style notification card.
//
// Dismiss flow:
//   1. Slide out (320ms CSS transition)
//   2. Park window at -2000,-2000 (NOT hide — parking keeps JS alive so the
//      next notification reuses the warm React context without cold-start delay)
//   3. invoke() backend command
//
// This mirrors how popup.tsx works: the window is always alive off-screen.

import React, { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

const appWindow = getCurrentWebviewWindow();

const AUTO_CLOSE_MS = 10_000;
const SLIDE_OUT_MS  = 320;

type Props = {
  title:       string;
  description: string;
  wordId:      number;
  onDismiss:   () => void;
};

type CloseAction = "done" | "later" | "auto";

export function TaskNotification({ title, description, wordId, onDismiss }: Props) {
  const [phase, setPhase]       = useState<"in" | "idle" | "out">("in");
  const [progress, setProgress] = useState(100);

  const rafRef     = useRef<number>(0);
  const startRef   = useRef<number>(0);
  const elapsedRef = useRef<number>(0);
  const pausedRef  = useRef(false);
  const closedRef  = useRef(false);

  // ── Close ──────────────────────────────────────────────────────────────────
  const close = useCallback((action: CloseAction) => {
    if (closedRef.current) return;
    closedRef.current = true;
    cancelAnimationFrame(rafRef.current);
    setPhase("out");

    setTimeout(async () => {
      onDismiss(); // clear payload → component unmounts cleanly

      // hide() is the correct approach here. Rust's show_task_notification()
      // always calls show() before emitting the next event (300ms delay gives
      // WebView2 time to un-throttle and re-register the listener).
      // Parking at -2000,-2000 was unreliable on Windows — the window stayed
      // at its last visible position, creating a permanent transparent overlay.
      await appWindow.hide();

      if (action === "done")  await invoke("task_notification_done",  { wordId });
      if (action === "later") await invoke("task_notification_later", { wordId });
    }, SLIDE_OUT_MS);
  }, [wordId, onDismiss]);

  // ── rAF countdown ─────────────────────────────────────────────────────────
  const tick = useCallback(() => {
    if (closedRef.current) return;
    if (!pausedRef.current) {
      const total = elapsedRef.current + (performance.now() - startRef.current);
      const pct   = Math.max(0, 100 - (total / AUTO_CLOSE_MS) * 100);
      setProgress(pct);
      if (pct <= 0) { close("auto"); return; }
    }
    rafRef.current = requestAnimationFrame(tick);
  }, [close]);

  // ── Mount ─────────────────────────────────────────────────────────────────
  useEffect(() => {
    const t = setTimeout(() => setPhase("idle"), 16);
    startRef.current = performance.now();
    rafRef.current   = requestAnimationFrame(tick);
    return () => {
      clearTimeout(t);
      cancelAnimationFrame(rafRef.current);
    };
  }, [tick]);

  // ── Hover ─────────────────────────────────────────────────────────────────
  const handleMouseEnter = () => {
    if (closedRef.current) return;
    pausedRef.current   = true;
    elapsedRef.current += performance.now() - startRef.current;
  };
  const handleMouseLeave = () => {
    if (closedRef.current) return;
    pausedRef.current = false;
    startRef.current  = performance.now();
  };

  return (
    <div
      className={`tn-root tn-root--${phase}`}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <div className="tn-progress">
        <div className="tn-progress__fill" style={{ width: `${progress}%` }} />
      </div>
      <div className="tn-header">
        <span className="tn-icon" aria-hidden>📚</span>
        <span className="tn-title">{title}</span>
        <button className="tn-close" aria-label="Zamknij" onClick={() => close("later")}>×</button>
      </div>
      <div className="tn-desc">{description}</div>
      <div className="tn-actions">
        <button className="tn-btn tn-btn--later" onClick={() => close("later")}>Później</button>
        <button className="tn-btn tn-btn--ok"    onClick={() => close("done")}>Ok</button>
      </div>
    </div>
  );
}
