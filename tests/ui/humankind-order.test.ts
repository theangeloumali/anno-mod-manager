// Run: node --test tests/ui/*.test.ts (Node strips the types; no test dependency).
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  enableModule,
  joinOrder,
  moduleKind,
  orderProblem,
  splitOrder,
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
