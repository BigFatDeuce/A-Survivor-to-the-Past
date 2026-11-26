-- 0.6.3 : titre offert a la Grande Fee, affiche au classement (pseudo en couleur).
-- A executer UNE fois sur la base en ligne AVANT de deployer le Worker 0.6.3 :
--   npx wrangler d1 execute astp-scores --remote --file worker/migrations/0063_title.sql
ALTER TABLE scores ADD COLUMN title INTEGER NOT NULL DEFAULT 0;
