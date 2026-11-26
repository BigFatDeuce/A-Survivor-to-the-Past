// Essais du classement : node --test worker/test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { check, expectedScore, versionAllowed, blockedName } from "./index.js";

const env = { LATEST: "0.6.0", STAGES: "hyrule_field,lost_woods" };
const base = {
  map: "hyrule_field", version: "0.6.0", name: "LINK", time: 600.5, kills: 2400, level: 42, bosses: 2,
  won: true, character: "link", difficulty: 1, mult: 1.35,
};
const run = (o = {}) => {
  const s = { ...base, ...o };
  s.score = o.score ?? expectedScore(s);
  return s;
};

test("partie normale acceptee", () => assert.equal(check(run(), env), null));
test("score falsifie refuse", () => assert.equal(check(run({ score: 99999999 }), env), "implausible"));
test("score du jeu (f32) accepte", () => {
  // le jeu calcule avec un multiplicateur f32 : 1.35f32 = 1.35000002384...
  const s = run();
  s.score = Math.round((2400 * 10 + 600 * 10 + 2000 + 5000) * 1.3500000238418579);
  assert.equal(check(s, env), null);
});
test("multiplicateur au-dela du maximum du jeu refuse", () => {
  const e = { ...env, MAX_MULT: "4.05" };
  assert.equal(check(run({ mult: 4.05 }), e), null);
  // le jeu envoie un f32 : 4.05f32 = 4.0500001907...
  assert.equal(check(run({ mult: 4.050000190734863 }), e), null);
  assert.equal(check(run({ mult: 5.5 }), e), "implausible");
});
test("trop d'ennemis refuse", () => assert.equal(check(run({ kills: 600 * 61 }), env), "implausible"));
test("victoire trop rapide refusee", () => assert.equal(check(run({ time: 120 }), env), "implausible"));
test("pseudo invalide refuse", () => {
  assert.equal(check(run({ name: "" }), env), "bad_name");
  assert.equal(check(run({ name: "ABCDEFGHIJKLM" }), env), "bad_name");
  assert.equal(check(run({ name: "<script>" }), env), "bad_name");
  assert.equal(check(run({ name: "FuCk3r" }), env), "bad_name");
});
test("noms ordinaires acceptes", () => {
  for (const n of ["Grape", "Computer", "Peacock", "Conor", "Essex", "Unique", "Torpedo", "Montenegro"]) {
    assert.equal(blockedName(n), false, n);
  }
});
test("titre de la Grande Fee", () => {
  assert.equal(check(run({ title: 3 }), env), null);
  assert.equal(check(run({ title: 4 }), env), "bad_request");
  assert.equal(check(run({ title: "or" }), env), "bad_request");
});
test("stage inconnu refuse", () => assert.equal(check(run({ map: "arena_test" }), env), "bad_request"));
test("versions", () => {
  assert.equal(versionAllowed("0.6.0", "0.6.0"), true);
  assert.equal(versionAllowed("0.7.0", "0.6.0"), false);
  assert.equal(versionAllowed("0.5.1", "0.6.0"), true);
  assert.equal(versionAllowed("0.9.9", "1.0.0"), false);
  assert.equal(versionAllowed("1.0.0", "1.0.0"), true);
  assert.equal(check(run({ version: "0.9.0" }), { ...env, LATEST: "1.0.0" }), "old_version");
});
