// Created by The Angelo Umali.
import { RotateCcwIcon } from "lucide-react";
import { useId, useState } from "react";
import { ChoiceSelect } from "@/components/app/choice-select";
import { FIELD_CLASS, SWITCH_CLASS } from "@/components/app/field-class";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import type { ModOption } from "@/data/types";

type OptionValue = ModOption["value"];

/** One control per ModOption type; `null` resets to the mod's default. */
export function OptionField({
  option,
  isDisabled,
  onChange,
}: {
  option: ModOption;
  isDisabled: boolean;
  onChange: (value: OptionValue | null) => void;
}) {
  const labelId = useId();
  const hintId = useId();
  const [draft, setDraft] = useState<OptionValue>(option.value);
  const isDefault = option.value === option.defaultValue;

  const label = (
    <span id={labelId} className="text-ui font-medium text-copy">
      {option.label}
    </span>
  );

  const controls: Record<ModOption["type"], React.ReactNode> = {
    toggle: (
      <Switch
        className={SWITCH_CLASS}
        checked={option.value === true}
        disabled={isDisabled}
        aria-labelledby={labelId}
        aria-describedby={option.hint ? hintId : undefined}
        onCheckedChange={(checked) => onChange(checked)}
      />
    ),
    enum: (
      <ChoiceSelect
        label={option.label}
        hideLabel
        className="w-full sm:w-56"
        value={String(option.value)}
        isDisabled={isDisabled}
        choices={(option.values ?? []).map((value) => ({
          value,
          label: value,
        }))}
        onChange={onChange}
      />
    ),
    slider: (
      <div className="flex w-full items-center gap-3 sm:w-64">
        <Slider
          className="flex-1 [&_[data-slot=slider-range]]:bg-accent"
          aria-labelledby={labelId}
          min={option.min}
          max={option.max}
          step={option.step}
          disabled={isDisabled}
          value={[Number(draft)]}
          onValueChange={(value) =>
            setDraft(Array.isArray(value) ? (value[0] ?? 0) : value)
          }
          onValueCommitted={(value) =>
            onChange(Array.isArray(value) ? (value[0] ?? 0) : value)
          }
        />
        <output
          aria-live="polite"
          className="w-10 text-right font-mono text-label tabular-nums"
        >
          {Number(draft)}
        </output>
      </div>
    ),
    text: (
      <input
        aria-labelledby={labelId}
        className={`${FIELD_CLASS} sm:w-56`}
        disabled={isDisabled}
        value={String(draft)}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={() => draft !== option.value && onChange(draft)}
      />
    ),
    color: (
      <div className="flex items-center gap-2">
        <input
          type="color"
          aria-labelledby={labelId}
          disabled={isDisabled}
          value={String(draft)}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={() => draft !== option.value && onChange(draft)}
          className="h-10 w-14 cursor-pointer rounded-field border border-line-strong bg-bg p-1 disabled:cursor-not-allowed"
        />
        <span className="font-mono text-label text-copy-muted">
          {String(draft)}
        </span>
      </div>
    ),
  };

  return (
    <div className="flex flex-col gap-2 py-3 sm:flex-row sm:items-center sm:justify-between">
      <div className="min-w-0">
        {label}
        {option.hint && (
          <p id={hintId} className="text-label text-copy-muted">
            {option.hint}
          </p>
        )}
        <p className="font-mono text-label text-copy-dim">{option.key}</p>
      </div>
      <div className="flex items-center gap-2">
        {controls[option.type]}
        <Button
          size="icon"
          variant="quiet"
          disabled={isDisabled || isDefault}
          aria-label={`Reset ${option.label} to ${String(option.defaultValue)}`}
          onClick={() => onChange(null)}
        >
          <RotateCcwIcon aria-hidden="true" />
        </Button>
      </div>
    </div>
  );
}
