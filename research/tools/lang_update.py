"""Textes des mises a jour (option et menu de l'ecran titre). Importe par gen_lang.py.
{v} = numero de version, {p} = pourcentage."""

KEYS = [
    "ui.updates", "ui.update.auto", "ui.update.ask", "ui.update.install", "ui.update.progress",
    "ui.update.restart", "ui.update.failed",
]

EXTRA = {
    "en": {
        "ui.updates": "Updates", "ui.update.auto": "Automatic", "ui.update.ask": "Ask me",
        "ui.update.install": "Update to v{v}", "ui.update.progress": "Downloading… {p}%",
        "ui.update.restart": "Restart to update", "ui.update.failed": "Update failed: web page",
    },
    "fr": {
        "ui.updates": "Mises à jour", "ui.update.auto": "Automatiques", "ui.update.ask": "Me demander",
        "ui.update.install": "Mettre à jour (v{v})", "ui.update.progress": "Téléchargement… {p} %",
        "ui.update.restart": "Redémarrer et installer", "ui.update.failed": "Échec : page de téléchargement",
    },
    "de": {
        "ui.updates": "Updates", "ui.update.auto": "Automatisch", "ui.update.ask": "Nachfragen",
        "ui.update.install": "Update auf v{v}", "ui.update.progress": "Download… {p} %",
        "ui.update.restart": "Neustart zum Aktualisieren", "ui.update.failed": "Fehler: Download-Seite",
    },
    "es": {
        "ui.updates": "Actualizaciones", "ui.update.auto": "Automáticas", "ui.update.ask": "Preguntar",
        "ui.update.install": "Actualizar a v{v}", "ui.update.progress": "Descargando… {p} %",
        "ui.update.restart": "Reiniciar y actualizar", "ui.update.failed": "Error: página de descargas",
    },
    "it": {
        "ui.updates": "Aggiornamenti", "ui.update.auto": "Automatici", "ui.update.ask": "Chiedi",
        "ui.update.install": "Aggiorna a v{v}", "ui.update.progress": "Download… {p}%",
        "ui.update.restart": "Riavvia e aggiorna", "ui.update.failed": "Errore: pagina di download",
    },
    "pt": {
        "ui.updates": "Atualizações", "ui.update.auto": "Automáticas", "ui.update.ask": "Perguntar",
        "ui.update.install": "Atualizar para v{v}", "ui.update.progress": "Baixando… {p}%",
        "ui.update.restart": "Reiniciar e atualizar", "ui.update.failed": "Falha: página de downloads",
    },
    "ja": {
        "ui.updates": "アップデート", "ui.update.auto": "じどう", "ui.update.ask": "かくにん する",
        "ui.update.install": "v{v}に アップデート", "ui.update.progress": "ダウンロードちゅう… {p}%",
        "ui.update.restart": "さいきどうして アップデート", "ui.update.failed": "しっぱい: ダウンロード ページ",
    },
}
