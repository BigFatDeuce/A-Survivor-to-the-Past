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
  created INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS scores_board ON scores (map, version, score DESC);
CREATE INDEX IF NOT EXISTS scores_name ON scores (map, version, name);
CREATE INDEX IF NOT EXISTS scores_ip ON scores (ip, created);
