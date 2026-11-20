// Created by The Angelo Umali.
import { useId, useState } from "react";
import { ChoiceSelect } from "@/components/app/choice-select";
import { FIELD_CLASS } from "@/components/app/field-class";
import { withToast } from "@/components/app/notify";
import { useCurrentGame } from "@/components/app/use-current-game";
import { Button } from "@/components/ui/button";
import { useAction } from "@/data/source";
import type { ReportKind } from "@/data/types";

const REPORT_KINDS: { value: ReportKind; label: string }[] = [
  { value: "not-working", label: "Doesn't work" },
  { value: "false-information", label: "Wrong information" },
  { value: "rude-content", label: "Rude content" },
  { value: "illegal-content", label: "Illegal content" },
  { value: "stolen-content", label: "Stolen content" },
  { value: "dmca", label: "Copyright (DMCA)" },
  { value: "generic", label: "Something else" },
];

export function ReportForm({
  modioId,
  onDone,
}: {
  modioId: number;
  onDone: () => void;
}) {
  const { gameId } = useCurrentGame();
  const summaryId = useId();
  const [kind, setKind] = useState<ReportKind>("not-working");
  const [summary, setSummary] = useState("");
  const report = useAction((source, text: string) =>
    source.reportMod(gameId, modioId, kind, text),
  );

  const submit = async () => {
    const sent = await withToast(
      report.mutateAsync([summary]),
      "Report sent to mod.io",
    );
    if (sent.ok) onDone();
  };

  return (
    <form
      className="flex flex-col gap-3 rounded-field border border-line bg-bg p-3"
      onSubmit={(event) => {
        event.preventDefault();
        void submit();
      }}
    >
      <ChoiceSelect
        label="What's wrong"
        value={kind}
        choices={REPORT_KINDS}
        onChange={setKind}
      />
      <div className="flex flex-col gap-1.5">
        <label
          htmlFor={summaryId}
          className="text-label font-medium text-copy-muted"
        >
          Details for mod.io's moderators
        </label>
        <textarea
          id={summaryId}
          rows={3}
          required
          value={summary}
          onChange={(event) => setSummary(event.target.value)}
          className={`${FIELD_CLASS} py-2`}
        />
      </div>
      <div className="flex justify-end gap-2">
        <Button size="sm" variant="quiet" type="button" onClick={onDone}>
          Cancel
        </Button>
        <Button size="sm" type="submit" disabled={report.isPending}>
          Send report
        </Button>
      </div>
    </form>
  );
}
