// Created by The Angelo Umali.
import { DownloadIcon, RefreshCwIcon, RotateCwIcon, XIcon } from "lucide-react";
import { formatBytes } from "@/components/app/format";
import { withToast } from "@/components/app/notify";
import { PageHeader } from "@/components/app/page-header";
import { EmptyState, Panel } from "@/components/app/panel";
import { StateBadge, type StateTone } from "@/components/app/state-badge";
import { useCurrentGame } from "@/components/app/use-current-game";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { useAction, useDownloads } from "@/data/source";
import type { DownloadJob, DownloadStatus } from "@/data/types";

const STATUS_BADGE: Record<DownloadStatus, { tone: StateTone; label: string }> =
  {
    queued: { tone: "neutral", label: "Queued" },
    downloading: { tone: "info", label: "Downloading" },
    verifying: { tone: "info", label: "Verifying" },
    done: { tone: "ok", label: "Installed" },
    failed: { tone: "error", label: "Failed" },
  };

const PERCENT = 100;

function JobRow({ job }: { job: DownloadJob }) {
  const retry = useAction((source) => source.retryDownload(job.id));
  const cancel = useAction((source) => source.cancelDownload(job.id));
  const badge = STATUS_BADGE[job.status];
  const percent = Math.round(job.progress * PERCENT);
  const isRunning =
    job.status === "downloading" ||
    job.status === "queued" ||
    job.status === "verifying";

  return (
    <li className="flex flex-col gap-2 py-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          <p className="truncate text-ui font-semibold">{job.name}</p>
          <p className="font-mono text-label text-copy-dim">
            {formatBytes(job.sizeBytes * job.progress)} of{" "}
            {formatBytes(job.sizeBytes)}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <StateBadge tone={badge.tone}>{badge.label}</StateBadge>
          {job.status === "failed" && (
            <Button
              size="sm"
              onClick={() =>
                withToast(retry.mutateAsync([]), `Retrying ${job.name}`)
              }
            >
              <RotateCwIcon aria-hidden="true" />
              Retry
            </Button>
          )}
          {isRunning && (
            <Button
              size="icon"
              variant="quiet"
              aria-label={`Cancel ${job.name}`}
              onClick={() =>
                withToast(cancel.mutateAsync([]), `Cancelled ${job.name}`)
              }
            >
              <XIcon aria-hidden="true" />
            </Button>
          )}
        </div>
      </div>
      {job.status !== "done" && (
        <Progress
          value={percent}
          aria-label={`${job.name} ${percent}%`}
          className="[&_[data-slot=progress-indicator]]:bg-accent [&_[data-slot=progress-track]]:bg-surface"
        />
      )}
      {job.error && (
        <p role="alert" className="text-ui text-danger">
          {job.error}
        </p>
      )}
    </li>
  );
}

export function DownloadsScreen() {
  const { gameId, isWriteLocked } = useCurrentGame();
  const { data: jobs = [] } = useDownloads();
  const updateAll = useAction((source) => source.updateAll(gameId));
  const sections = [
    {
      title: "In progress",
      jobs: jobs.filter(
        (job) =>
          job.status === "queued" ||
          job.status === "downloading" ||
          job.status === "verifying",
      ),
    },
    { title: "Failed", jobs: jobs.filter((job) => job.status === "failed") },
    { title: "Done", jobs: jobs.filter((job) => job.status === "done") },
  ].filter((section) => section.jobs.length > 0);

  return (
    <div className="flex flex-col gap-5">
      <PageHeader
        title="Downloads"
        description="mod.io downloads and updates. Each file is verified before it's installed, and a failed download can be retried from the start."
        actions={
          <Button
            disabled={isWriteLocked || updateAll.isPending}
            onClick={() =>
              withToast(
                updateAll.mutateAsync([]),
                "Checking every mod for updates",
              )
            }
          >
            <RefreshCwIcon aria-hidden="true" />
            Update all
          </Button>
        }
      />
      {sections.length === 0 ? (
        <EmptyState icon={<DownloadIcon />} title="Nothing downloading">
          Subscribe to a mod on mod.io, or run Update all, and its progress
          shows here.
        </EmptyState>
      ) : (
        sections.map((section) => (
          <Panel
            key={section.title}
            title={`${section.title} (${section.jobs.length})`}
          >
            <ul className="divide-y divide-line">
              {section.jobs.map((job) => (
                <JobRow key={job.id} job={job} />
              ))}
            </ul>
          </Panel>
        ))
      )}
    </div>
  );
}
