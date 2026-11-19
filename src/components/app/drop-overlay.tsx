// Created by The Angelo Umali.
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { UploadIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useStartInstallPreview } from "./install-preview-sheet";

type DropHandlers = {
  setDragging: (dragging: boolean) => void;
  onDrop: (paths: string[]) => void;
};

const hasFiles = (event: DragEvent) =>
  event.dataTransfer?.types.includes("Files") ?? false;

/** Tauri's native drop handler swallows HTML5 file drops but hands over real paths. */
function listenTauriDrops({ setDragging, onDrop }: DropHandlers) {
  const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
    if (payload.type === "enter") setDragging(true);
    else if (payload.type === "leave") setDragging(false);
    else if (payload.type === "drop") {
      setDragging(false);
      if (payload.paths.length > 0) onDrop(payload.paths);
    }
  });
  return () => void unlisten.then((stop) => stop());
}

/** The browser only sees file names; dragenter/leave fire per child, so a depth counter tracks real exit. */
function listenBrowserDrops({ setDragging, onDrop }: DropHandlers) {
  let dragDepth = 0;
  const handleEnter = (event: DragEvent) => {
    if (!hasFiles(event)) return;
    event.preventDefault();
    dragDepth += 1;
    setDragging(true);
  };
  const handleOver = (event: DragEvent) => {
    if (hasFiles(event)) event.preventDefault();
  };
  const handleLeave = (event: DragEvent) => {
    if (!hasFiles(event)) return;
    dragDepth = Math.max(0, dragDepth - 1);
    if (dragDepth === 0) setDragging(false);
  };
  const handleDrop = (event: DragEvent) => {
    if (!hasFiles(event)) return;
    event.preventDefault();
    dragDepth = 0;
    setDragging(false);
    const fileNames = [...(event.dataTransfer?.files ?? [])].map(
      (file) => file.name,
    );
    if (fileNames.length > 0) onDrop(fileNames);
  };
  window.addEventListener("dragenter", handleEnter);
  window.addEventListener("dragover", handleOver);
  window.addEventListener("dragleave", handleLeave);
  window.addEventListener("drop", handleDrop);
  return () => {
    window.removeEventListener("dragenter", handleEnter);
    window.removeEventListener("dragover", handleOver);
    window.removeEventListener("dragleave", handleLeave);
    window.removeEventListener("drop", handleDrop);
  };
}

export function DropOverlay() {
  const [isDragging, setDragging] = useState(false);
  const startPreview = useStartInstallPreview();
  const startPreviewRef = useRef(startPreview);
  startPreviewRef.current = startPreview;

  useEffect(() => {
    const handlers: DropHandlers = {
      setDragging,
      onDrop: (paths) => void startPreviewRef.current(paths),
    };
    return isTauri()
      ? listenTauriDrops(handlers)
      : listenBrowserDrops(handlers);
  }, []);

  return (
    <>
      <p role="status" className="sr-only">
        {isDragging
          ? "Drop to install. Nothing is written until you confirm."
          : ""}
      </p>
      {isDragging && (
        <div
          aria-hidden="true"
          className="pointer-events-none fixed inset-0 z-50 grid place-items-center bg-(--backdrop) p-6"
        >
          <div className="flex w-full max-w-lg flex-col items-center gap-3 rounded-card border-2 border-dashed border-accent bg-bg-elevated px-8 py-12 text-center shadow-overlay">
            <UploadIcon className="size-10 text-accent" />
            <p className="font-display text-h3 font-bold">Drop to install</p>
            <p className="text-ui text-copy-muted">
              Zip, 7z or a mod folder. Atelier finds every mod inside, checks
              it, and shows you before anything is written.
            </p>
          </div>
        </div>
      )}
    </>
  );
}
