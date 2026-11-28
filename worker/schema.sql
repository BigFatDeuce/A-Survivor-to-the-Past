-- Base du classement en ligne (Cloudflare D1).
-- Creation : npx wrangler d1 execute astp-scores --remote --file worker/schema.sql
CREATE TABLE IF NOT EXISTS scores (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  map TEXT NOT NULL,
  version TEXT NOT NULL,
  name TEXT NOT NULL,
  score INTEGER NOT NULL,
  time REAL NOT NULL,
  kills INTEGER NOT NULL,
  level INTEGER NOT NULL,
  bosses INTEGER NOT NULL,
  won INTEGER NOT NULL,
  character TEXT NOT NULL,
  difficulty INTEGER NOT NULL,
  mult REAL NOT NULL,
  -- empreinte salee de l'adresse IP (limite d'envois), videe au bout de 30 jours
  ip TEXT NOT NULL,
  created INTEGER NOT NULL,
  -- 0.6.3 : titre offert a la Grande Fee (0 : aucun) ; base existante : worker/migrations/0063_title.sql
  title INTEGER NOT NULL DEFAULT 0,
  -- 0.9 : joueurs de la partie (1 : solo, 2 a 4 : coop) ; base existante : worker/migrations/0090_coop.sql
  players INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS scores_board ON scores (map, version, score DESC);
CREATE INDEX IF NOT EXISTS scores_players ON scores (map, version, players, score DESC);
CREATE INDEX IF NOT EXISTS scores_name ON scores (map, version, name);
CREATE INDEX IF NOT EXISTS scores_ip ON scores (ip, created);
-- 0.8.1 : journal des envois (limite par heure et par jour), purge au bout d'un jour ;
-- base existante : worker/migrations/0081_submits.sql
CREATE TABLE IF NOT EXISTS submits (
  ip TEXT NOT NULL,
  created INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS submits_ip ON submits (ip, created);
