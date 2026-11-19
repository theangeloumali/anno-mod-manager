// Created by The Angelo Umali.
import type { Issue } from "@/data/types";

/** An incompatible pair belongs to both mods, so both rows and both detail panels show it. */
export const issuesForMod = (issues: Issue[], modId: string) =>
  issues.filter(
    (issue) =>
      issue.modId === modId ||
      (issue.kind === "incompatible" && issue.relatedModId === modId),
  );
