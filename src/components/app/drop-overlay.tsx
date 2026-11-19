// Created by The Angelo Umali.
import { UploadIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useStartInstallPreview } from "./install-preview-sheet";

const hasFiles = (event: DragEvent) =>
  event.dataTransfer?.types.includes("Files") ?? false;

/** Listens on the whole window; dragenter/leave fire per child, so a depth counter tracks real exit. */
export function DropOverlay() {
  const [isDragging, setDragging] = useState(false);
  const startPreview = useStartInstallPreview();
  const startPreviewRef = useRef(startPreview);
  startPreviewRef.current = startPreview;

  useEffect(() => {
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
      if (fileNames.length > 0) void startPreviewRef.current(fileNames);
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
              Zip, 7z, rar or a mod folder. Atelier finds every mod inside,
              checks it, and shows you before anything is written.
            </p>
          </div>
        </div>
      )}
    </>
  );
}
