// Created by The Angelo Umali.
import { useId } from "react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { SELECT_TRIGGER_CLASS } from "./field-class";

interface Choice<TValue extends string> {
  value: TValue;
  label: string;
}

/** shadcn Select with a real label; `hideLabel` keeps it for screen readers only. */
export function ChoiceSelect<TValue extends string>({
  label,
  value,
  choices,
  onChange,
  isDisabled = false,
  hideLabel = false,
  className,
}: {
  label: string;
  value: TValue;
  choices: Choice<TValue>[];
  onChange: (value: TValue) => void;
  isDisabled?: boolean;
  hideLabel?: boolean;
  className?: string;
}) {
  const labelId = useId();
  return (
    <div className={cn("flex min-w-0 flex-col gap-1.5", className)}>
      <span
        id={labelId}
        className={cn(
          "text-label font-medium text-copy-muted",
          hideLabel && "sr-only",
        )}
      >
        {label}
      </span>
      <Select
        items={choices}
        value={value}
        disabled={isDisabled}
        onValueChange={(nextValue) => {
          const match = choices.find((choice) => choice.value === nextValue);
          if (match) onChange(match.value);
        }}
      >
        <SelectTrigger
          aria-labelledby={labelId}
          className={SELECT_TRIGGER_CLASS}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {choices.map((choice) => (
            <SelectItem key={choice.value} value={choice.value}>
              {choice.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
