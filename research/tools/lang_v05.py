"""Textes de la version 0.5 : nouveaux personnages (Sonic, Zelda, Link NES), descriptions avec defauts,
rencontres (champions errants) et leurs recompenses, nouvelles reliques. Importe par gen_lang.py.
Japonais en kana uniquement (police du jeu) ; pas de guillemets doubles ni de chevrons, pas de point-virgule."""

UI = ["ui.encounter_spawned", "ui.encounter_fled", "ui.encounter_won", "ui.reward.title", "ui.reward.blessing",
      "ui.reward.blessing_desc", "ui.reward.boost", "ui.reward.boost_desc", "ui.reward.relic", "ui.reward.relic_desc",
      "ui.relic_hint_encounter"]
CHARS = ["link", "link_minish", "link_capless", "sonic", "zelda", "link_nes"]
RELICS = ["magic_mirror", "big_key", "titan_mitt", "red_mail", "bottle", "shovel", "small_key", "golden_sword",
          "fire_shield"]

KEYS = (UI + [f"char.{c}.{f}" for c in CHARS[3:] for f in ("name", "desc")]
        + [f"relic.{r}.name" for r in RELICS])


def table(ui, chars, relics):
    assert len(ui) == len(UI) and len(chars) == len(CHARS) and len(relics) == len(RELICS)
    t = dict(zip(UI, ui))
    for i, (n, d) in zip(CHARS, chars):
        t[f"char.{i}.name"] = n
        t[f"char.{i}.desc"] = d
    for i, n in zip(RELICS, relics):
        t[f"relic.{i}.name"] = n
    return t


EXTRA = {
    "fr": table(
        ["Un champion errant vous défie ! Suivez la flèche.", "Le champion est reparti.", "Champion vaincu !",
         "Butin du champion", "Bénédiction", "Bonus pour le reste de la partie.", "Spécial",
         "Amélioration spéciale, gardée quand l'arme évolue.", "Relique",
         "Active tout de suite. Gardée pour toujours si vous gagnez la partie.",
         "Offerte par les champions errants. Gardée si vous gagnez la partie."],
        [("Link", "Le héros d'Hyrule. Plus de dégâts et d'armure, mais peu de chance. Part avec l'épée."),
         ("Link (Minish)", "Rapide et chanceux, mais fragile et moins puissant. Part avec le boomerang."),
         ("Link sans bonnet", "Robuste et puissant, mais lent et ses armes rechargent moins vite. Part avec les bombes."),
         ("Sonic", "Ultra rapide, dash presque sans attente, épée plus vive. Mais très fragile et frappe moins large."),
         ("Zelda", "Sage : effets plus longs, plus d'expérience, recharge rapide. Lente et moins puissante. Part avec la canne de Somaria."),
         ("Link (NES)", "Le premier héros : une flèche de plus, plus rapides. Mais moins de dégâts et de zone. Part avec l'arc.")],
        ["Miroir magique", "Grande clé", "Gant de Titan", "Mailles rouges", "Flacon", "Pelle", "Petite clé",
         "Épée d'or", "Bouclier de feu"],
    ),
    "en": table(
        ["A wandering champion challenges you! Follow the arrow.", "The champion is gone.", "Champion defeated!",
         "Champion's loot", "Blessing", "Bonus for the rest of the run.", "Special",
         "Special upgrade, kept when the weapon evolves.", "Relic",
         "Active right away. Kept forever if you win the run.",
         "Given by wandering champions. Kept if you win the run."],
        [("Link", "The hero of Hyrule. More damage and armor, but little luck. Starts with the sword."),
         ("Minish Link", "Fast and lucky, but frail and weaker. Starts with the boomerang."),
         ("Capless Link", "Sturdy and strong, but slow, and his weapons recharge slower. Starts with bombs."),
         ("Sonic", "Super fast, near-instant dash, quicker sword. But very frail and hits a smaller area."),
         ("Zelda", "Wise: longer effects, more experience, quick recharge. Slow and weaker. Starts with the Cane of Somaria."),
         ("NES Link", "The first hero: one more arrow, faster shots. But less damage and area. Starts with the bow.")],
        ["Magic Mirror", "Big Key", "Titan's Mitt", "Red Mail", "Bottle", "Shovel", "Small Key", "Golden Sword",
         "Fire Shield"],
    ),
    "de": table(
        ["Ein wandernder Champion fordert dich heraus! Folge dem Pfeil.", "Der Champion ist fort.",
         "Champion besiegt!", "Beute des Champions", "Segen", "Bonus für den Rest des Laufs.", "Spezial",
         "Spezialverbesserung, bleibt bei der Entwicklung der Waffe.", "Relikt",
         "Sofort aktiv. Für immer behalten, wenn du den Lauf gewinnst.",
         "Von wandernden Champions. Bleibt, wenn du den Lauf gewinnst."],
        [("Link", "Der Held Hyrules. Mehr Schaden und Rüstung, aber wenig Glück. Startet mit dem Schwert."),
         ("Minish-Link", "Schnell und glücklich, aber zerbrechlich und schwächer. Startet mit dem Bumerang."),
         ("Link ohne Mütze", "Robust und stark, aber langsam, und seine Waffen laden langsamer. Startet mit Bomben."),
         ("Sonic", "Superschnell, Sprint fast ohne Wartezeit, flinkeres Schwert. Aber sehr zerbrechlich, kleinere Fläche."),
         ("Zelda", "Weise: längere Effekte, mehr Erfahrung, schnelles Aufladen. Langsam und schwächer. Startet mit dem Somaria-Stab."),
         ("NES-Link", "Der erste Held: ein Pfeil mehr, schnellere Schüsse. Aber weniger Schaden und Fläche. Startet mit dem Bogen.")],
        ["Zauberspiegel", "Großer Schlüssel", "Titanhandschuh", "Rotes Kettenhemd", "Flasche", "Schaufel",
         "Kleiner Schlüssel", "Goldschwert", "Feuerschild"],
    ),
    "es": table(
        ["¡Un campeón errante te desafía! Sigue la flecha.", "El campeón se ha ido.", "¡Campeón derrotado!",
         "Botín del campeón", "Bendición", "Bonificación para el resto de la partida.", "Especial",
         "Mejora especial, se conserva al evolucionar el arma.", "Reliquia",
         "Activa al instante. Tuya para siempre si ganas la partida.",
         "La dan los campeones errantes. Se conserva si ganas la partida."],
        [("Link", "El héroe de Hyrule. Más daño y armadura, pero poca suerte. Empieza con la espada."),
         ("Link (Minish)", "Rápido y con suerte, pero frágil y más débil. Empieza con el bumerán."),
         ("Link sin gorro", "Robusto y fuerte, pero lento y sus armas recargan más despacio. Empieza con bombas."),
         ("Sonic", "Ultrarrápido, carrera casi sin espera, espada más ágil. Pero muy frágil y golpea menos área."),
         ("Zelda", "Sabia: efectos más largos, más experiencia, recarga rápida. Lenta y más débil. Empieza con el bastón de Somaria."),
         ("Link (NES)", "El primer héroe: una flecha más, disparos más rápidos. Pero menos daño y área. Empieza con el arco.")],
        ["Espejo mágico", "Llave grande", "Guante de titán", "Malla roja", "Botella", "Pala", "Llave pequeña",
         "Espada dorada", "Escudo de fuego"],
    ),
    "it": table(
        ["Un campione errante ti sfida! Segui la freccia.", "Il campione se n'è andato.", "Campione sconfitto!",
         "Bottino del campione", "Benedizione", "Bonus per il resto della partita.", "Speciale",
         "Potenziamento speciale, resta quando l'arma evolve.", "Reliquia",
         "Attiva subito. Tua per sempre se vinci la partita.",
         "La danno i campioni erranti. Resta se vinci la partita."],
        [("Link", "L'eroe di Hyrule. Più danni e armatura, ma poca fortuna. Parte con la spada."),
         ("Link (Minish)", "Veloce e fortunato, ma fragile e più debole. Parte con il boomerang."),
         ("Link senza berretto", "Robusto e forte, ma lento e le sue armi si ricaricano più piano. Parte con le bombe."),
         ("Sonic", "Velocissimo, scatto quasi senza attesa, spada più rapida. Ma fragilissimo e colpisce un'area minore."),
         ("Zelda", "Saggia: effetti più lunghi, più esperienza, ricarica rapida. Lenta e più debole. Parte con il bastone di Somaria."),
         ("Link (NES)", "Il primo eroe: una freccia in più, colpi più veloci. Ma meno danni e area. Parte con l'arco.")],
        ["Specchio magico", "Chiave grande", "Guanto del titano", "Cotta rossa", "Ampolla", "Pala", "Chiave piccola",
         "Spada d'oro", "Scudo di fuoco"],
    ),
    "pt": table(
        ["Um campeão errante desafia você! Siga a seta.", "O campeão foi embora.", "Campeão derrotado!",
         "Espólio do campeão", "Bênção", "Bônus para o resto da partida.", "Especial",
         "Melhoria especial, mantida quando a arma evolui.", "Relíquia",
         "Ativa na hora. Sua para sempre se você vencer a partida.",
         "Dada pelos campeões errantes. Mantida se você vencer a partida."],
        [("Link", "O herói de Hyrule. Mais dano e armadura, mas pouca sorte. Começa com a espada."),
         ("Link (Minish)", "Rápido e sortudo, mas frágil e mais fraco. Começa com o bumerangue."),
         ("Link sem gorro", "Robusto e forte, mas lento, e suas armas recarregam mais devagar. Começa com bombas."),
         ("Sonic", "Ultrarrápido, investida quase sem espera, espada mais ágil. Mas muito frágil e acerta área menor."),
         ("Zelda", "Sábia: efeitos mais longos, mais experiência, recarga rápida. Lenta e mais fraca. Começa com o bastão de Somaria."),
         ("Link (NES)", "O primeiro herói: uma flecha a mais, tiros mais rápidos. Mas menos dano e área. Começa com o arco.")],
        ["Espelho mágico", "Chave grande", "Luva de titã", "Malha vermelha", "Garrafa", "Pá", "Chave pequena",
         "Espada dourada", "Escudo de fogo"],
    ),
    "ja": table(
        ["さまよう チャンピオンが あらわれた! やじるしを おえ!", "チャンピオンは さっていった。", "チャンピオンを たおした!",
         "チャンピオンの たからもの", "しゅくふく", "この ゲームの あいだ つよくなる。", "とくべつ",
         "とくべつな きょうか。 ぶきが しんかしても のこる。", "レリック",
         "すぐに きく。 ゲームに かつと ずっと てに はいる。",
         "さまよう チャンピオンが くれる。 ゲームに かつと のこる。"],
        [("リンク", "ハイラルの ゆうしゃ。 ダメージと ぼうぎょが たかいが ラッキーが ひくい。 けんで はじまる。"),
         ("ミニッシュの リンク", "はやくて ラッキー。 でも もろくて ちからが よわい。 ブーメランで はじまる。"),
         ("ぼうしの ない リンク", "がんじょうで ちからもち。 でも おそくて ぶきの じゅうてんも おそい。 バクダンで はじまる。"),
         ("ソニック", "ものすごく はやい。 ダッシュも けんも すばやい。 でも とても もろくて はんいが せまい。"),
         ("ゼルダ", "かしこい: こうかが ながく、 けいけんちも ふえる。 おそくて ちからが よわい。 ソマリアの つえで はじまる。"),
         ("ファミコンの リンク", "さいしょの ゆうしゃ: やが 1ぽん おおく、 はやい。 でも ダメージと はんいが へる。 ゆみで はじまる。")],
        ["まほうの かがみ", "おおきな カギ", "タイタンの ミット", "あかい ふく", "あきビン", "シャベル", "ちいさな カギ",
         "ゴールドの けん", "ほのおの たて"],
    ),
}
