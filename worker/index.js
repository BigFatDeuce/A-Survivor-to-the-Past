// Classement en ligne d'A Survivor to the Past (Cloudflare Worker + base D1).
//
//   GET  /api/scores?map=<stage>&version=<x.y.z|vide>  -> top 100 du stage pour cette version
//                                                         (vide : la derniere version du jeu)
//   POST /api/scores  (JSON)                            -> publie un score, renvoie son rang
//
// Le reste du site est servi tel quel (fichiers statiques de site/). Aucune donnee personnelle :
// le pseudo choisi par le joueur, sa partie, et une empreinte salee de l'adresse IP (limite
// d'envois), effacee au bout de 30 jours.
//
// Anti-triche par vraisemblance : le score est recalcule a partir de la partie, et les parties
// impossibles (temps, ennemis vaincus, niveau, boss, multiplicateur) sont refusees.
// Suppression manuelle d'un score :
//   npx wrangler d1 execute astp-scores --remote --command "DELETE FROM scores WHERE id = 123"

const JSON_HEADERS = {
  "Content-Type": "application/json; charset=utf-8",
  "X-Content-Type-Options": "nosniff",
  "Referrer-Policy": "no-referrer",
};

// Caracteres autorises dans un pseudo (les memes que la grille de saisie du jeu).
const NAME_RE = /^[A-Za-z0-9\-.!? '+:]{1,12}$/;
const ID_RE = /^[a-z0-9_]{1,32}$/;
const VERSION_RE = /^\d{1,3}\.\d{1,3}\.\d{1,3}$/;

// Mots refuses dans les pseudos (recherches sans espaces ni ponctuation, chiffres usuels remplaces).
// (pas de mots courts contenus dans des noms ordinaires : « grape », « computer », « peacock »...)
const BLOCKED = [
  "nigger", "nigga", "faggot", "fagot", "retard", "nazi", "hitler", "rapist", "pedophil",
  "cunt", "whore", "slut", "bitch", "fuck", "shit", "dick", "pussy", "penis", "vagina", "porn", "kkk",
  "salope", "connard", "connasse", "encule", "batard", "bougnoul", "bicot", "youpin", "negre",
  "tapette", "merde", "couille", "niquer", "putain",
  "mierda", "joder", "cabron", "maricon", "pendejo",
  "scheisse", "fotze", "wichser", "schwuchtel", "hurensohn",
  "cazzo", "stronzo", "troia", "frocio", "vaffanculo",
  "caralho", "porra", "viado", "buceta",
];

// Limites d'envoi par adresse.
const PER_HOUR = 12;
const PER_DAY = 40;
// Parties gardees par pseudo et par classement.
const KEEP_PER_NAME = 3;

function json(body, status = 200, extra = {}) {
  return new Response(JSON.stringify(body), { status, headers: { ...JSON_HEADERS, ...extra } });
}

function refuse(error, status = 400) {
  return json({ ok: false, error }, status);
}

function parseVersion(v) {
  return v.split(".").map((n) => parseInt(n, 10));
}

function cmpVersion(a, b) {
  const x = parseVersion(a);
  const y = parseVersion(b);
  for (let i = 0; i < 3; i++) {
    if (x[i] !== y[i]) return x[i] - y[i];
  }
  return 0;
}

/// Versions visibles : a partir de la 1.0, les scores des 0.x disparaissent.
function versionAllowed(v, latest) {
  if (!VERSION_RE.test(v) || cmpVersion(v, latest) > 0) return false;
  return !(parseVersion(latest)[0] >= 1 && parseVersion(v)[0] === 0);
}

function stages(env) {
  return (env.STAGES || "").split(",").map((s) => s.trim()).filter(Boolean);
}

function blockedName(name) {
  const flat = name
    .toLowerCase()
    .replace(/0/g, "o").replace(/1/g, "i").replace(/3/g, "e").replace(/4/g, "a").replace(/5/g, "s").replace(/7/g, "t")
    .replace(/[^a-z]/g, "");
  return BLOCKED.some((w) => flat.includes(w));
}

async function ipHash(request, env) {
  const ip = request.headers.get("CF-Connecting-IP") || "0.0.0.0";
  const data = new TextEncoder().encode(`${env.SALT || "astp"}:${ip}`);
  const digest = await crypto.subtle.digest("SHA-256", data);
  return [...new Uint8Array(digest).slice(0, 12)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/// Score de la partie, calcule comme dans le jeu (profile::score).
function expectedScore(s) {
  const base = s.kills * 10 + Math.floor(Math.max(0, s.time)) * 10 + s.bosses * 1000 + (s.won ? 5000 : 0);
  return Math.round(base * Math.max(0, s.mult));
}

/// Partie vraisemblable ? Renvoie un code d'erreur, ou null.
function check(s, env) {
  const num = (v) => typeof v === "number" && Number.isFinite(v);
  const int = (v) => Number.isInteger(v);
  if (typeof s !== "object" || s === null) return "bad_request";
  if (typeof s.map !== "string" || !ID_RE.test(s.map)) return "bad_request";
  const known = stages(env);
  if (known.length && !known.includes(s.map)) return "bad_request";
  if (typeof s.version !== "string" || !VERSION_RE.test(s.version)) return "bad_request";
  if (cmpVersion(s.version, env.LATEST) > 0) return "bad_request";
  if (!versionAllowed(s.version, env.LATEST)) return "old_version";
  if (typeof s.name !== "string") return "bad_name";
  const name = s.name.trim();
  if (!NAME_RE.test(name) || blockedName(name)) return "bad_name";
  if (typeof s.character !== "string" || !ID_RE.test(s.character)) return "bad_request";
  if (!int(s.difficulty) || s.difficulty < 0 || s.difficulty > 3) return "bad_request";
  if (!num(s.time) || !int(s.kills) || !int(s.level) || !int(s.bosses) || !int(s.score) || !num(s.mult)) return "bad_request";
  if (typeof s.won !== "boolean") return "bad_request";
  // limites larges : une partie dure au plus 2 h ; 60 ennemis par seconde en moyenne au plus
  if (s.time < 1 || s.time > 7200) return "implausible";
  if (s.kills < 0 || s.kills > s.time * 60) return "implausible";
  if (s.level < 1 || s.level > 5 + s.time / 2) return "implausible";
  if (s.bosses < 0 || s.bosses > 20 || s.bosses > 1 + s.time / 60) return "implausible";
  if (s.won && (s.bosses < 1 || s.time < 300)) return "implausible";
  // difficulte (0,75 a 1,8) x (1 + Sceaux, 1,25 au plus) ; marge pour l'avenir
  if (s.mult < 0.5 || s.mult > 6) return "implausible";
  const want = expectedScore(s);
  if (Math.abs(s.score - want) > Math.max(2, want * 1e-6)) return "implausible";
  return null;
}

async function board(request, env, url) {
  const map = url.searchParams.get("map") || "";
  let version = url.searchParams.get("version") || "";
  if (!ID_RE.test(map)) return refuse("bad_request");
  const latest = env.LATEST;
  if (!version) version = latest;
  if (!versionAllowed(version, latest)) return refuse("bad_request");
  // lecture mise en cache une minute (dans chaque centre de donnees)
  const cache = caches.default;
  const key = new Request(`https://cache.local/scores?map=${map}&version=${version}&latest=${latest}`);
  const hit = await cache.match(key);
  if (hit) return hit;
  const rows = await env.DB.prepare(
    `SELECT name, score, character, time, kills, won, difficulty FROM (
       SELECT *, ROW_NUMBER() OVER (PARTITION BY lower(name) ORDER BY score DESC, created ASC) AS rn
       FROM scores WHERE map = ?1 AND version = ?2
     ) WHERE rn = 1 ORDER BY score DESC, created ASC LIMIT 100`
  ).bind(map, version).all();
  const vers = await env.DB.prepare(`SELECT DISTINCT version FROM scores WHERE map = ?1`).bind(map).all();
  const versions = [...new Set([latest, ...vers.results.map((r) => r.version)])]
    .filter((v) => versionAllowed(v, latest))
    .sort((a, b) => cmpVersion(b, a));
  const scores = rows.results.map((r, i) => ({
    rank: i + 1,
    name: r.name,
    score: r.score,
    character: r.character,
    time: r.time,
    kills: r.kills,
    won: !!r.won,
    difficulty: r.difficulty,
  }));
  const res = json({ map, version, versions, scores }, 200, { "Cache-Control": "public, max-age=60" });
  await cache.put(key, res.clone());
  return res;
}

async function submit(request, env) {
  if (!(request.headers.get("Content-Type") || "").startsWith("application/json")) return refuse("bad_request", 415);
  const text = await request.text();
  if (text.length > 2048) return refuse("bad_request", 413);
  let s;
  try {
    s = JSON.parse(text);
  } catch {
    return refuse("bad_request");
  }
  const err = check(s, env);
  if (err) return refuse(err, err === "old_version" ? 409 : 400);
  const name = s.name.trim();
  const ip = await ipHash(request, env);
  const now = Math.floor(Date.now() / 1000);
  const db = env.DB;
  const recent = await db
    .prepare(`SELECT SUM(created > ?2) AS hour, COUNT(*) AS day FROM scores WHERE ip = ?1 AND created > ?3`)
    .bind(ip, now - 3600, now - 86400)
    .first();
  if ((recent?.hour || 0) >= PER_HOUR || (recent?.day || 0) >= PER_DAY) return refuse("rate_limited", 429);
  const dup = await db
    .prepare(`SELECT id FROM scores WHERE map = ?1 AND version = ?2 AND lower(name) = lower(?3) AND score = ?4 AND kills = ?5 AND abs(time - ?6) < 0.01`)
    .bind(s.map, s.version, name, s.score, s.kills, s.time)
    .first();
  if (dup) return refuse("duplicate", 409);
  await db.batch([
    db
      .prepare(
        `INSERT INTO scores (map, version, name, score, time, kills, level, bosses, won, character, difficulty, mult, ip, created)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)`
      )
      .bind(s.map, s.version, name, s.score, s.time, s.kills, s.level, s.bosses, s.won ? 1 : 0, s.character, s.difficulty, s.mult, ip, now),
    // seules les meilleures parties de chaque pseudo restent
    db
      .prepare(
        `DELETE FROM scores WHERE id IN (
           SELECT id FROM (
             SELECT id, ROW_NUMBER() OVER (ORDER BY score DESC, created ASC) AS rn
             FROM scores WHERE map = ?1 AND version = ?2 AND lower(name) = lower(?3)
           ) WHERE rn > ?4)`
      )
      .bind(s.map, s.version, name, KEEP_PER_NAME),
    // empreintes d'adresse effacees au bout de 30 jours
    db.prepare(`UPDATE scores SET ip = '' WHERE ip != '' AND created < ?1`).bind(now - 30 * 86400),
  ]);
  const better = await db
    .prepare(
      `SELECT COUNT(*) AS n FROM (
         SELECT lower(name) AS k, MAX(score) AS best FROM scores WHERE map = ?1 AND version = ?2 GROUP BY lower(name)
       ) WHERE best > ?3`
    )
    .bind(s.map, s.version, s.score)
    .first();
  await caches.default.delete(new Request(`https://cache.local/scores?map=${s.map}&version=${s.version}&latest=${env.LATEST}`));
  return json({ ok: true, rank: (better?.n || 0) + 1 });
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.pathname !== "/api/scores") {
      if (url.pathname.startsWith("/api/")) return refuse("not_found", 404);
      return env.ASSETS.fetch(request);
    }
    try {
      if (request.method === "GET") return await board(request, env, url);
      if (request.method === "POST") return await submit(request, env);
      return refuse("method", 405);
    } catch (e) {
      console.error(e);
      return refuse("server", 500);
    }
  },
};

// Pour les essais (node --test).
export { check, expectedScore, versionAllowed, blockedName, cmpVersion };
