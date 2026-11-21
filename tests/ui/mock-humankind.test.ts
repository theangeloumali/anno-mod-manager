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
import { CoreError } from "../../src/data/core-error.ts";
import { MISSING_MODULE } from "../../src/data/mock-humankind-seeds.ts";
import { mockSource, resetMockData } from "../../src/data/mock-source.ts";

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
  assert.equal(name, "Restored 2026-11-21 19-42-08 UTC");
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

test("a save version only a Workshop copy could provide is refused before anything is restored", () => {
  const { hk, mods } = setup();
  const sigma = mods.find((mod) => mod.id === "sigma pack");
  assert.ok(sigma);
  sigma.location = "workshop";
  const save = hk.saves.saves[0];
  assert.ok(save);
  assert.throws(
    () => makeHumankindSaveLoadable(hk, mods, save.path),
    (error: unknown) =>
      error instanceof CoreError &&
      error.code === "read-only" &&
      error.message.includes("Steam Workshop folder"),
  );
  assert.equal(sigma.version, "3.2");
});

const hkOrder = async () =>
  (await mockSource.loadOrder("humankind")).entries.map((entry) => entry.modId);

test("uninstalling a listed mod drops it from Registry.xml's list", async () => {
  resetMockData();
  await mockSource.uninstallMod("humankind", "prussia");
  assert.equal((await hkOrder()).includes("prussia"), false);
});

test("uninstalling a mod that isn't listed leaves the list as it was", async () => {
  resetMockData();
  await mockSource.setModActive("humankind", "prussia", false);
  const before = await hkOrder();
  await mockSource.uninstallMod("humankind", "prussia");
  assert.deepEqual(await hkOrder(), before);
});

test("dropping a module that already has a Workshop copy is refused", async () => {
  resetMockData();
  const preview = await mockSource.previewInstall("humankind", [
    "C:\\Downloads\\deepsea-reloaded.zip",
  ]);
  const item = preview.items[0];
  assert.equal(item?.status, "invalid");
  assert.match(
    item?.checks[0]?.message ?? "",
    /Already installed in the Steam Workshop folder; remove that copy first/,
  );
});

test("dropping a module that already has a game-folder copy is refused", async () => {
  resetMockData();
  const preview = await mockSource.previewInstall("humankind", [
    "C:\\Downloads\\the-lost-dynasties.zip",
  ]);
  assert.equal(preview.items[0]?.status, "invalid");
  assert.match(
    preview.items[0]?.checks[0]?.message ?? "",
    /Already installed in the game folder; remove that copy first/,
  );
});
