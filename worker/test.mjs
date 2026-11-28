// Essais du classement : node --test worker/test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { check, multAllowed, expectedScore, versionAllowed, blockedName } from "./index.js";

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
test("multiplicateur impossible pour la difficulte refuse", () => {
  const e = { ...env, MAX_MULT: "4.05", DIFF_MULTS: "0.75,1,1.35,1.8", SEAL_BONUSES: "0.25,0.25,0.25,0.2,0.3" };
  // Normal sans Sceau, Facile avec deux Sceaux, Heros avec tous (f32 du jeu)
  assert.equal(check(run({ difficulty: 1, mult: 1 }), e), null);
  assert.equal(multAllowed(0.75 * 1.5, 0, e), true);
  assert.equal(check(run({ difficulty: 3, mult: 4.050000190734863 }), e), null);
  // multiplicateur du Heros annonce en Facile
  assert.equal(check(run({ difficulty: 0, mult: 1.8 }), e), "implausible");
  assert.equal(check(run({ difficulty: 1, mult: 1.37 }), e), "implausible");
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

// 0.9 : chaque ennemi vaut 10 x son poids de vie (difficulte, Sceau de la Force)
const env9 = {
  LATEST: "0.9.0", STAGES: "hyrule_field,lost_woods", MAX_MULT: "4.05", DIFF_MULTS: "0.75,1,1.35,1.8",
  SEAL_BONUSES: "0.25,0.25,0.25,0.2,0.3", DIFF_HP: "0.7,1.0,1.4,2.0", SEAL_HP: "1.5,1,1,1,1",
};
test("poids de vie : partie 0.9 acceptee, calcul entier comme le jeu", () => {
  assert.equal(check(run({ version: "0.9.0", difficulty: 2, mult: 1.35, kw: 1.4 }), env9), null);
  // Heros + Sceau de la Force, valeurs f32 du jeu : 2 x 1,5 = 2,9999998
  assert.equal(check(run({ version: "0.9.0", difficulty: 3, mult: 2.25, kw: 2.9999998 }), env9), null);
  assert.equal(expectedScore({ kills: 101, time: 0, bosses: 0, won: false, mult: 1, kw: 2.9999998 }), 3030);
  assert.equal(expectedScore({ kills: 100, time: 0, bosses: 0, won: false, mult: 1, kw: 0.7 }), 700);
});
test("poids de vie : absent apres la 0.8.0 refuse, accepte avant", () => {
  assert.equal(check(run({ version: "0.9.0", difficulty: 1, mult: 1 }), env9), "bad_request");
  assert.equal(check(run({ version: "0.8.0", difficulty: 1, mult: 1 }), env9), null);
});
test("poids de vie impossible refuse", () => {
  // Facile qui annonce la vie du Heros
  assert.equal(check(run({ version: "0.9.0", difficulty: 0, mult: 0.75, kw: 2 }), env9), "implausible");
  // Sceau de la Force dans le poids mais pas dans le multiplicateur
  assert.equal(check(run({ version: "0.9.0", difficulty: 1, mult: 1, kw: 1.5 }), env9), "implausible");
  // meme Sceau compte des deux cotes : accepte
  assert.equal(check(run({ version: "0.9.0", difficulty: 1, mult: 1.25, kw: 1.5 }), env9), null);
});

// ---------------------------------------------------------------------------------------------
// Salle de coop (worker/room.js) avec des sockets factices

import { Room } from "./room.js";

class FakeSocket {
  constructor() {
    this.sent = [];
    this.readyState = 1;
    this.att = null;
  }
  send(m) {
    this.sent.push(m);
  }
  close() {
    this.readyState = 3;
  }
  serializeAttachment(a) {
    this.att = structuredClone(a);
  }
  deserializeAttachment() {
    return this.att;
  }
}

function fakeRoom() {
  const socks = [];
  const store = new Map();
  const ctx = {
    getWebSockets: () => socks,
    acceptWebSocket: (ws) => socks.push(ws),
    storage: {
      get: async (k) => store.get(k),
      put: async (k, v) => store.set(k, v),
      delete: async (k) => store.delete(k),
      deleteAll: async () => store.clear(),
    },
  };
  globalThis.WebSocketPair = class {
    constructor() {
      this.a = new FakeSocket();
      this.b = new FakeSocket();
      return { 0: this.a, 1: this.b };
    }
  };
  const room = new Room(ctx, {});
  // la reponse 101 n'existe pas hors de Cloudflare : la socket cote salle suffit
  const join = async (create, v = "0.9.0") => {
    const before = socks.length;
    try {
      await room.fetch(new Request(`https://x/api/coop/ABCD?room=ABCD&v=${v}&create=${create ? 1 : 0}`));
    } catch {
      // Response 101 refusee par node : sans importance
    }
    return socks.length > before ? socks[socks.length - 1] : null;
  };
  return { room, socks, join };
}

const parse = (m) => [m.slice(0, m.indexOf(" ")), JSON.parse(m.slice(m.indexOf(" ") + 1))];

test("salle : places, relais, depart", async () => {
  const { room, join } = fakeRoom();
  const a = await join(true);
  assert.deepEqual(parse(a.sent[0]), ["S", { t: "hello", slot: 0, room: "ABCD", peers: [] }]);
  const b = await join(false);
  assert.deepEqual(parse(b.sent[0])[1].peers, [0]);
  assert.equal(parse(b.sent[0])[1].slot, 1);
  assert.deepEqual(parse(a.sent[1]), ["S", { t: "join", s: 1 }]);
  await room.webSocketMessage(b, '{"t":"in","k":5,"i":[1,2]}');
  assert.equal(a.sent[2], '1 {"t":"in","k":5,"i":[1,2]}');
  assert.equal(b.sent.length, 1, "pas de renvoi a l'expediteur");
  // la partie est lancee : plus d'arrivee
  await room.webSocketMessage(a, '{"t":"start","seed":1}');
  const c = await join(false);
  assert.deepEqual(parse(c.sent[0])[1], { t: "error", e: "started" });
  await room.webSocketMessage(a, '{"t":"lobby"}');
  // b part : a est prevenu, sa place se libere
  b.readyState = 3;
  await room.webSocketClose(b, 1000);
  assert.deepEqual(parse(a.sent[a.sent.length - 1]), ["S", { t: "leave", s: 1 }]);
});

test("salle : refus (code inconnu, autre version, pleine)", async () => {
  let r = fakeRoom();
  const x = await r.join(false);
  assert.deepEqual(parse(x.sent[0])[1], { t: "error", e: "no_room" });
  r = fakeRoom();
  await r.join(true);
  const v = await r.join(false, "0.8.0");
  assert.deepEqual(parse(v.sent[0])[1], { t: "error", e: "version" });
  r = fakeRoom();
  for (let k = 0; k < 4; k++) await r.join(k === 0);
  // les sockets refusees restent dans la liste factice : on ne garde que les 4 premieres
  r.socks.splice(4);
  const full = await r.join(false);
  assert.deepEqual(parse(full.sent[0])[1], { t: "error", e: "full" });
});

test("salle : un joueur qui inonde est deconnecte", async () => {
  const { room, join } = fakeRoom();
  const a = await join(true);
  const b = await join(false);
  for (let k = 0; k < 130; k++) await room.webSocketMessage(b, '{"t":"in","k":1,"i":[0]}');
  assert.equal(b.readyState, 3);
  assert.ok(a.sent.some((m) => m === 'S {"t":"leave","s":1}'));
});

test("coop : equipe acceptee, un classement par nombre de joueurs", () => {
  const team = { players: 3, team: ["LINK", "Zelda", "Impa"], character: "link,zelda,link", kills: 2400 * 2 };
  assert.equal(check(run(team), env), null);
  // une equipe tue plus : la limite suit le nombre de joueurs
  assert.equal(check(run({ ...team, kills: 600 * 61 * 2 }), env), null);
  assert.equal(check(run({ ...team, kills: 600 * 61 * 3 }), env), "implausible");
});
test("coop : equipe mal formee refusee", () => {
  assert.equal(check(run({ players: 2, team: ["LINK"], character: "link,link" }), env), "bad_name");
  assert.equal(check(run({ players: 2, team: ["LINK", "FuCk3r"], character: "link,link" }), env), "bad_name");
  assert.equal(check(run({ players: 2, team: ["LINK", "Zelda"], character: "link" }), env), "bad_request");
  assert.equal(check(run({ players: 5, team: ["A", "B", "C", "D", "E"], character: "a,b,c,d,e" }), env), "bad_request");
  // solo : un seul personnage
  assert.equal(check(run({ character: "link,zelda" }), env), "bad_request");
});
