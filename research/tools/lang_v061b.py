"""Textes de la version 0.6.1 (suite) : fiche du bestiaire, boss ajoutes (un par zone), personnages
ajoutes (Toon Link, Link d'Oracle of Ages, Zora des mers, Crewmate) et couleurs du Crewmate.
Importe par gen_lang.py. Japonais en kana uniquement (police du jeu) ; pas de guillemets doubles ni de
chevrons, pas de point-virgule."""

UI = ["ui.beast_open", "ui.beast_hint", "ui.beast_hp", "ui.beast_damage", "ui.beast_speed",
      "skin.pink", "skin.orange", "skin.yellow", "skin.cyan"]
ENEMIES = ["helmasaur_king", "blind", "vitreous", "arrghus", "moldorm", "trinexx"]
CHARS = ["toon_link", "link_ooa", "sea_zora", "crewmate"]

KEYS = UI + [f"enemy.{e}" for e in ENEMIES] + [f"char.{c}.{f}" for c in CHARS for f in ("name", "desc")]


def table(ui, enemies, chars):
    assert len(ui) == len(UI) and len(enemies) == len(ENEMIES) and len(chars) == len(CHARS)
    t = dict(zip(UI, ui))
    t.update({f"enemy.{e}": n for e, n in zip(ENEMIES, enemies)})
    for i, (n, d) in zip(CHARS, chars):
        t[f"char.{i}.name"] = n
        t[f"char.{i}.desc"] = d
    return t


EXTRA = {
    "fr": table(
        ["A ou clic : voir sa fiche.", "Gauche/Droite : ennemi suivant. A : fermer.", "PV", "Dégâts (coeurs)", "Vitesse",
         "Rose", "Orange", "Jaune", "Cyan"],
        ["Roi Helmasaur", "Blind", "Vitreous", "Arrghus", "Moldorm", "Trinexx"],
        [("Toon Link", "Rapide et son dash revient plus vite, mais fragile. Part avec le grappin."),
         ("Link (Oracle of Ages)", "Frappe plus large et ses effets durent plus longtemps, mais ses projectiles sont plus lents. Part avec la baguette de feu."),
         ("Zora des mers", "Écailles épaisses : armure et 7 coeurs, mais lent. Part avec la canne de Byrna."),
         ("Crewmate", "Très chanceux et attire les objets, mais frappe un peu moins fort. Part avec les bombes.")],
    ),
    "en": table(
        ["A or click: open its card.", "Left/Right: next enemy. A: close.", "HP", "Damage (hearts)", "Speed",
         "Pink", "Orange", "Yellow", "Cyan"],
        ["Helmasaur King", "Blind", "Vitreous", "Arrghus", "Moldorm", "Trinexx"],
        [("Toon Link", "Fast, and his dash comes back sooner, but frail. Starts with the Hookshot."),
         ("Oracle of Ages Link", "Wider attacks and longer effects, but slower projectiles. Starts with the Fire Rod."),
         ("Sea Zora", "Thick scales: armor and 7 hearts, but slow. Starts with the Cane of Byrna."),
         ("Crewmate", "Very lucky and pulls items, but hits a bit softer. Starts with bombs.")],
    ),
    "de": table(
        ["A oder Klick: Steckbrief öffnen.", "Links/Rechts: nächster Gegner. A: schließen.", "KP", "Schaden (Herzen)", "Tempo",
         "Rosa", "Orange", "Gelb", "Cyan"],
        ["Helmasaurus King", "Blind", "Vitreous", "Arrghus", "Moldorm", "Trinexx"],
        [("Toon-Link", "Schnell, und sein Sprint lädt schneller, aber zerbrechlich. Startet mit dem Enterhaken."),
         ("Link (Oracle of Ages)", "Größere Angriffe und längere Effekte, aber langsamere Geschosse. Startet mit dem Feuerstab."),
         ("Meeres-Zora", "Dicke Schuppen: Rüstung und 7 Herzen, aber langsam. Startet mit dem Byrna-Stab."),
         ("Crewmate", "Sehr glücklich und zieht Gegenstände an, aber trifft etwas schwächer. Startet mit Bomben.")],
    ),
    "es": table(
        ["A o clic: ver su ficha.", "Izquierda/Derecha: siguiente enemigo. A: cerrar.", "PV", "Daño (corazones)", "Velocidad",
         "Rosa", "Naranja", "Amarillo", "Cian"],
        ["Rey Helmasaurio", "Blind", "Vitreous", "Arrghus", "Moldorm", "Trinexx"],
        [("Toon Link", "Rápido y su carrera vuelve antes, pero frágil. Empieza con el gancho."),
         ("Link (Oracle of Ages)", "Ataques más amplios y efectos más largos, pero proyectiles más lentos. Empieza con la vara de fuego."),
         ("Zora marino", "Escamas gruesas: armadura y 7 corazones, pero lento. Empieza con el cetro de Byrna."),
         ("Crewmate", "Muy afortunado y atrae objetos, pero golpea algo menos. Empieza con las bombas.")],
    ),
    "it": table(
        ["A o clic: apri la scheda.", "Sinistra/Destra: nemico successivo. A: chiudi.", "PS", "Danni (cuori)", "Velocità",
         "Rosa", "Arancione", "Giallo", "Ciano"],
        ["Re Helmasaur", "Blind", "Vitreous", "Arrghus", "Moldorm", "Trinexx"],
        [("Toon Link", "Veloce e il suo scatto torna prima, ma fragile. Parte con il rampino."),
         ("Link (Oracle of Ages)", "Attacchi più ampi ed effetti più lunghi, ma proiettili più lenti. Parte con la verga di fuoco."),
         ("Zora marino", "Squame spesse: armatura e 7 cuori, ma lento. Parte con il bastone di Byrna."),
         ("Crewmate", "Molto fortunato e attira gli oggetti, ma colpisce un po' meno. Parte con le bombe.")],
    ),
    "pt": table(
        ["A ou clique: ver a ficha.", "Esquerda/Direita: próximo inimigo. A: fechar.", "PV", "Dano (corações)", "Velocidade",
         "Rosa", "Laranja", "Amarelo", "Ciano"],
        ["Rei Helmasaur", "Blind", "Vitreous", "Arrghus", "Moldorm", "Trinexx"],
        [("Toon Link", "Rápido e sua corrida volta antes, mas frágil. Começa com o gancho."),
         ("Link (Oracle of Ages)", "Ataques mais amplos e efeitos mais longos, mas projéteis mais lentos. Começa com o bastão de fogo."),
         ("Zora marinho", "Escamas grossas: armadura e 7 corações, mas lento. Começa com o cetro de Byrna."),
         ("Crewmate", "Muito sortudo e atrai itens, mas bate um pouco mais fraco. Começa com as bombas.")],
    ),
    "ja": table(
        ["Aか クリックで くわしく みる。", "ひだり/みぎ：つぎの てき。 A：とじる。", "たいりょく", "こうげき(ハート)", "はやさ",
         "ピンク", "オレンジ", "きいろ", "みずいろ"],
        ["ヘルマキング", "ブラインド", "ビトレアス", "アーガス", "モルドーム", "トライノックス"],
        [("トゥーンリンク", "はやくて ダッシュの じゅうてんも はやいが もろい。 フックショットで はじまる。"),
         ("じくうの しょうの リンク", "こうげきの はんいが ひろく こうかも ながいが たまが おそい。 ファイアロッドで はじまる。"),
         ("うみの ゾーラ", "あつい うろこで まもりと ハート7こ。 でも おそい。 バイラのつえで はじまる。"),
         ("クルーメイト", "とても ラッキーで アイテムを ひきよせるが すこし よわい。 バクダンで はじまる。")],
    ),
}
