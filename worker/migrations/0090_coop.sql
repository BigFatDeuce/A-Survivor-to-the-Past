-- 0.9 : classement de la coop en ligne (une table, un classement par nombre de joueurs).
-- A lancer AVANT de deployer le worker 0.9 :
--   npx wrangler d1 execute astp-scores --remote --file worker/migrations/0090_coop.sql
ALTER TABLE scores ADD COLUMN players INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS scores_players ON scores (map, version, players, score DESC);
