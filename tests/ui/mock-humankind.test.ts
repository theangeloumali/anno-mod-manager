// The mock must refuse what the core refuses, or the UI is built against rules that don't exist.
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  createHumankindMock,
  humankindMods,
  makeHumankindSaveLoadable,
  restoreHumankindHistory,
  setHumankindModActive,
  writeHumankindOrder,
} from "../../src/data/mock-humankind.ts";
import { MISSING_MODULE } from "../../src/data/mock-humankind-seeds.ts";

const setup = () => {
  const hk = createHumankindMock();
  return { hk, mods: humankindMods(hk) };
};

test("writing an order that lists a missing module is refused as not-found", () => {
  const { hk, mods } = setup();
  assert.throws(
    () => writeHumankindOrder(hk, mods, ["prussia", MISSING_MODULE]),
    (error: { code: string; message: string }) =>
      error.code === "not-found" && error.message.includes(MISSING_MODULE),
  );
});

test("writing two Conversions is refused with the core's rule", () => {
  const { hk, mods } = setup();
  assert.throws(
    () => writeHumankindOrder(hk, mods, ["sigma pack", "the lost dynasties"]),
    /both Conversion modules/,
  );
});

test("a valid order is written with the base game first and mods follow it", () => {
  const { hk, mods } = setup();
  const written = writeHumankindOrder(hk, mods, ["sigma pack", "Prussia"]);
  assert.deepEqual(written, ["project mercury", "sigma pack", "prussia"]);
  assert.equal(
    mods.find((mod) => mod.id === "deepsea reloaded")?.active,
    false,
  );
});

test("switching on a Conversion replaces the one already on", () => {
  const { hk, mods } = setup();
  setHumankindModActive(hk, mods, "the lost dynasties", true);
  assert.equal(hk.order.includes("sigma pack"), false);
  assert.equal(hk.order[1], "the lost dynasties");
  assert.equal(mods.find((mod) => mod.id === "sigma pack")?.active, false);
});

test("restoring a history entry saves its old list as a profile", () => {
  const { hk } = setup();
  const name = restoreHumankindHistory(hk, 0);
  assert.equal(name, "Restored 2026-11-21 19-42-08");
  assert.ok(hk.profiles.some((profile) => profile.name === name));
});

test("a save with a missing mod restores what it can and leaves the registry alone", () => {
  const { hk, mods } = setup();
  const before = [...hk.order];
  const save = hk.saves.saves[0];
  assert.ok(save);
  const report = makeHumankindSaveLoadable(hk, mods, save.path);
  assert.deepEqual(report.restored, [{ name: "sigma pack", version: "3.1" }]);
  assert.deepEqual(
    report.missing.map((check) => check.name),
    [MISSING_MODULE],
  );
  assert.equal(report.registryWritten, false);
  assert.deepEqual(hk.order, before);
});
