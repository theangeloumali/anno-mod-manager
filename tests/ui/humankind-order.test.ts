// Run: node --test tests/ui/*.test.ts (Node strips the types; no test dependency).
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  enableModule,
  isMap,
  joinOrder,
  moduleKind,
  moveExtension,
  moveExtensionInOrder,
  orderProblem,
  splitOrder,
  withExtensions,
} from "../../src/data/humankind-order.ts";

const KINDS: Record<string, string> = {
  deepsea: "Standalone",
  sigma: "Conversion",
  dynasties: "Conversion",
  prussia: "Extension",
  nukes: "",
};
const kindOf = (name: string) => moduleKind(KINDS[name] ?? "");

test("a blank or unknown category counts as an Extension", () => {
  assert.equal(moduleKind(""), "Extension");
  assert.equal(moduleKind("Standalone"), "Standalone");
  assert.equal(kindOf("not-installed"), "Extension");
});

test("splitOrder groups the fixed slots apart from the Extensions", () => {
  const groups = splitOrder(["deepsea", "sigma", "prussia", "nukes"], kindOf);
  assert.deepEqual(groups, {
    standalone: "deepsea",
    conversion: "sigma",
    extensions: ["prussia", "nukes"],
  });
  assert.deepEqual(joinOrder(groups), ["deepsea", "sigma", "prussia", "nukes"]);
});

test("enabling a Conversion replaces the one already on, in its slot", () => {
  const order = enableModule(
    ["deepsea", "sigma", "prussia"],
    "dynasties",
    kindOf,
  );
  assert.deepEqual(order, ["deepsea", "dynasties", "prussia"]);
});

test("enabling an Extension appends it", () => {
  assert.deepEqual(enableModule(["sigma"], "nukes", kindOf), [
    "sigma",
    "nukes",
  ]);
});

test("orderProblem names the first broken rule", () => {
  assert.equal(orderProblem(["deepsea", "sigma", "prussia"], kindOf), null);
  assert.match(
    orderProblem(["prussia", "deepsea"], kindOf) ?? "",
    /Standalone module 'deepsea' must come right after the base game/,
  );
  assert.match(
    orderProblem(["sigma", "dynasties"], kindOf) ?? "",
    /both Conversion modules/,
  );
});

test("moving an extension shifts the others and leaves the order valid", () => {
  assert.deepEqual(moveExtension(["a", "b", "c"], 0, 2), ["b", "c", "a"]);
  assert.deepEqual(moveExtension(["a", "b", "c"], 2, 0), ["c", "a", "b"]);
});

test("a move past either end is refused as null", () => {
  assert.equal(moveExtension(["a", "b"], 0, -1), null);
  assert.equal(moveExtension(["a", "b"], 1, 2), null);
});

test("rewriting the Extensions leaves a second Conversion where it was", () => {
  const order = ["sigma", "dynasties", "prussia", "nukes"];
  assert.deepEqual(withExtensions(order, ["nukes", "prussia"], kindOf), [
    "sigma",
    "dynasties",
    "nukes",
    "prussia",
  ]);
});

test("an Extension move is refused while two Standalones are listed, so neither is dropped", () => {
  const order = ["deepsea", "abyss", "prussia", "nukes"];
  const kinds = (name: string) =>
    name === "abyss" ? moduleKind("Standalone") : kindOf(name);
  assert.equal(moveExtensionInOrder(order, 0, 1, kinds), null);
});

test("an Extension move keeps a module that isn't installed in the list", () => {
  const order = ["sigma", "prussia", "historical leaders", "nukes"];
  assert.deepEqual(moveExtensionInOrder(order, 0, 2, kindOf), [
    "sigma",
    "historical leaders",
    "nukes",
    "prussia",
  ]);
});

test("a map is told apart from a module by its category", () => {
  assert.equal(isMap({ category: "Map" }), true);
  assert.equal(isMap({ category: "Extension" }), false);
  assert.equal(isMap({ category: "" }), false);
});
