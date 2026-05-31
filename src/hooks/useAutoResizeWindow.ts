import { useEffect, type RefObject } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { PhysicalSize, PhysicalPosition } from "@tauri-apps/api/dpi";

/**
 * Hook to automatically resize the Tauri window to match the DOM element's size.
 * Anchors the window to the bottom-right of its current position (grows up/left).
 * Uses Physical pixels to avoid ambiguity with system-level text scaling (zoom).
 */
export function useAutoResizeWindow(ref: RefObject<HTMLElement | null>) {
  useEffect(() => {
    const element = ref.current;
    if (!element) return;

    const appWindow = getCurrentWebviewWindow();

    const resizeObserver = new ResizeObserver(async () => {
      // Use getBoundingClientRect for sub-pixel precision
      const rect = element.getBoundingClientRect();
      
      // Convert CSS pixels to Physical pixels using devicePixelRatio.
      // devicePixelRatio accounts for both DPI scaling AND "Make text bigger" zoom.
      const dpr = window.devicePixelRatio;
      const targetWidthPhys = Math.round(rect.width * dpr);
      const targetHeightPhys = Math.round(rect.height * dpr);

      if (targetWidthPhys === 0 || targetHeightPhys === 0) return;

      try {
        // Get current window state in physical pixels
        const currentSizePhys = await appWindow.innerSize(); 
        const currentPosPhys = await appWindow.outerPosition(); 

        // Calculate bottom-right anchor in physical pixels
        const bottomYPhys = currentPosPhys.y + currentSizePhys.height;
        const rightXPhys = currentPosPhys.x + currentSizePhys.width;

        const newYPhys = bottomYPhys - targetHeightPhys;
        const newXPhys = rightXPhys - targetWidthPhys;

        // Perform atomic-like update: Size then Position
        // On Windows, setSize can be slightly slower than position, so we wait.
        await appWindow.setSize(new PhysicalSize(targetWidthPhys, targetHeightPhys));
        await appWindow.setPosition(new PhysicalPosition(newXPhys, newYPhys));
      } catch (err) {
        console.error("Failed to resize window:", err);
      }
    });

    resizeObserver.observe(element);

    return () => {
      resizeObserver.disconnect();
    };
  }, [ref]);
}
