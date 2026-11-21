// Created by The Angelo Umali.
import { OctagonXIcon } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";

const ALERT_CLASS = "rounded-field border-danger/60 bg-bg-elevated text-copy";

export function MissingModulesAlert({
  missing,
  onRemove,
}: {
  missing: string[];
  onRemove: (module: string) => void;
}) {
  const isOne = missing.length === 1;
  return (
    <Alert className={ALERT_CLASS}>
      <OctagonXIcon aria-hidden="true" className="text-danger" />
      <AlertTitle className="font-semibold">
        The list names {isOne ? "a module that isn't" : "modules that aren't"}{" "}
        installed
      </AlertTitle>
      <AlertDescription className="text-copy">
        <p>
          At launch Humankind would reset the whole list to the base game.
          Remove {isOne ? "it" : "them"}, then save.
        </p>
        <ul className="mt-2 flex flex-col gap-2">
          {missing.map((module) => (
            <li key={module} className="flex flex-wrap items-center gap-2">
              <span className="font-mono text-label wrap-anywhere">
                {module}
              </span>
              <Button size="sm" onClick={() => onRemove(module)}>
                Remove from list
              </Button>
            </li>
          ))}
        </ul>
      </AlertDescription>
    </Alert>
  );
}

export function OrderProblemAlert({ problem }: { problem: string }) {
  return (
    <Alert className={ALERT_CLASS}>
      <OctagonXIcon aria-hidden="true" className="text-danger" />
      <AlertTitle className="font-semibold">
        This order breaks a rule
      </AlertTitle>
      <AlertDescription className="text-copy">{problem}</AlertDescription>
    </Alert>
  );
}
