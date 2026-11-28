-- 0.8.1 : journal des envois pour la limite par heure et par jour (avant : comptee sur les parties gardees,
-- or une partie remplacee ou non gardee n'y reste pas).
-- npx wrangler d1 execute astp-scores --remote --file worker/migrations/0081_submits.sql
CREATE TABLE IF NOT EXISTS submits (
  ip TEXT NOT NULL,
  created INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS submits_ip ON submits (ip, created);
