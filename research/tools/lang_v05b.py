"""Textes de la version 0.5 (suite) : personnages de AdditionalChars (Link Four Swords et ses couleurs,
Link's Awakening, Link adulte, Link Mojo, Maskass) et Shadow Link. Importe par gen_lang.py.
Japonais en kana uniquement (police du jeu) ; pas de guillemets doubles ni de chevrons, pas de point-virgule."""

UI = ["ui.char_color", "ui.unlock_slain", "skin.green", "skin.red", "skin.blue", "skin.purple"]
CHARS = ["link_fsa", "link_awakening", "link_adult", "deku_link", "shy_guy", "shadow_link"]

KEYS = UI + [f"char.{c}.{f}" for c in CHARS for f in ("name", "desc")]


def table(ui, chars):
    assert len(ui) == len(UI) and len(chars) == len(CHARS)
    t = dict(zip(UI, ui))
    for i, (n, d) in zip(CHARS, chars):
        t[f"char.{i}.name"] = n
        t[f"char.{i}.desc"] = d
    return t


EXTRA = {
    "fr": table(
        ["Couleur", "Vainquez {s} pour le débloquer.", "Vert", "Rouge", "Bleu", "Violet"],
        [("Link (Four Swords)", "Un coup d'épée de plus, mais chaque coup frappe moins fort. Quatre couleurs au choix."),
         ("Link (Link's Awakening)", "Chanceux, attire les objets de loin, mais moins de dégâts. Part avec la poudre magique."),
         ("Link adulte", "Plus de dégâts et de zone, mais plus lent et son dash revient moins vite. Part avec le marteau."),
         ("Link Mojo", "Léger, effets plus longs, attire les objets. Mais fragile et moins puissant. Part avec les abeilles."),
         ("Maskass", "Très chanceux, mais apprend moins vite et frappe un peu moins fort. Part avec la baguette de glace."),
         ("Shadow Link", "L'ombre de Link : très puissant et rapide, mais malchanceux et apprend moins vite. Part avec l'épée.")],
    ),
    "en": table(
        ["Color", "Defeat {s} to unlock.", "Green", "Red", "Blue", "Purple"],
        [("Four Swords Link", "One more sword strike, but each one hits softer. Four colors to choose from."),
         ("Link's Awakening Link", "Lucky and pulls items from afar, but deals less damage. Starts with magic powder."),
         ("Adult Link", "More damage and area, but slower, and his dash comes back slower. Starts with the hammer."),
         ("Deku Link", "Light, longer effects, pulls items. But frail and weaker. Starts with the bees."),
         ("Shy Guy", "Very lucky, but learns slower and hits a bit softer. Starts with the Ice Rod."),
         ("Shadow Link", "Link's shadow: very strong and fast, but unlucky and learns slower. Starts with the sword.")],
    ),
    "de": table(
        ["Farbe", "Besiege {s} zum Freischalten.", "Grün", "Rot", "Blau", "Lila"],
        [("Four-Swords-Link", "Ein Schwertschlag mehr, aber jeder trifft schwächer. Vier Farben zur Wahl."),
         ("Link (Link's Awakening)", "Glücklich und zieht Gegenstände von weit her an, aber weniger Schaden. Startet mit Zauberpulver."),
         ("Erwachsener Link", "Mehr Schaden und Fläche, aber langsamer, und sein Sprint lädt langsamer. Startet mit dem Hammer."),
         ("Deku-Link", "Leicht, längere Effekte, zieht Gegenstände an. Aber zerbrechlich und schwächer. Startet mit Bienen."),
         ("Shy Guy", "Sehr glücklich, aber lernt langsamer und trifft etwas schwächer. Startet mit dem Eisstab."),
         ("Shadow Link", "Links Schatten: sehr stark und schnell, aber glücklos und lernt langsamer. Startet mit dem Schwert.")],
    ),
    "es": table(
        ["Color", "Derrota a {s} para desbloquearlo.", "Verde", "Rojo", "Azul", "Morado"],
        [("Link (Four Swords)", "Un espadazo más, pero cada uno golpea más flojo. Cuatro colores a elegir."),
         ("Link (Link's Awakening)", "Afortunado y atrae objetos de lejos, pero hace menos daño. Empieza con los polvos mágicos."),
         ("Link adulto", "Más daño y área, pero más lento y su carrera tarda más en volver. Empieza con el martillo."),
         ("Link Deku", "Ligero, efectos más largos, atrae objetos. Pero frágil y más débil. Empieza con las abejas."),
         ("Shy Guy", "Muy afortunado, pero aprende más despacio y golpea algo menos. Empieza con la vara de hielo."),
         ("Shadow Link", "La sombra de Link: muy fuerte y rápido, pero sin suerte y aprende más despacio. Empieza con la espada.")],
    ),
    "it": table(
        ["Colore", "Sconfiggi {s} per sbloccarlo.", "Verde", "Rosso", "Blu", "Viola"],
        [("Link (Four Swords)", "Un fendente in più, ma ogni colpo è più debole. Quattro colori a scelta."),
         ("Link (Link's Awakening)", "Fortunato e attira gli oggetti da lontano, ma fa meno danni. Parte con la polvere magica."),
         ("Link adulto", "Più danni e area, ma più lento e lo scatto torna più tardi. Parte con il martello."),
         ("Link Deku", "Leggero, effetti più lunghi, attira gli oggetti. Ma fragile e più debole. Parte con le api."),
         ("Tipo Timido", "Fortunatissimo, ma impara più piano e colpisce un po' meno. Parte con la verga di ghiaccio."),
         ("Shadow Link", "L'ombra di Link: fortissimo e veloce, ma sfortunato e impara più piano. Parte con la spada.")],
    ),
    "pt": table(
        ["Cor", "Derrote {s} para desbloquear.", "Verde", "Vermelho", "Azul", "Roxo"],
        [("Link (Four Swords)", "Um golpe de espada a mais, mas cada um acerta mais fraco. Quatro cores à escolha."),
         ("Link (Link's Awakening)", "Sortudo e atrai itens de longe, mas causa menos dano. Começa com o pó mágico."),
         ("Link adulto", "Mais dano e área, mas mais lento, e a investida demora a voltar. Começa com o martelo."),
         ("Link Deku", "Leve, efeitos mais longos, atrai itens. Mas frágil e mais fraco. Começa com as abelhas."),
         ("Shy Guy", "Muito sortudo, mas aprende mais devagar e bate um pouco menos. Começa com a vara de gelo."),
         ("Shadow Link", "A sombra de Link: muito forte e rápido, mas azarado e aprende mais devagar. Começa com a espada.")],
    ),
    "ja": table(
        ["カラー", "{s}を たおすと つかえる", "みどり", "あか", "あお", "むらさき"],
        [("4つの けんの リンク", "けんの こうげきが 1かい ふえるが いちげきが よわい。 カラーを 4しょくから えらべる。"),
         ("ゆめをみる しまの リンク", "ラッキーで とおくの アイテムも ひきよせるが ダメージが ひくい。 まほうの こなで はじまる。"),
         ("おとなの リンク", "ダメージと はんいが あがるが おそくて ダッシュの じゅうてんも おそい。 ハンマーで はじまる。"),
         ("デクリンク", "かるくて こうかが ながく アイテムを ひきよせる。 でも もろくて ちからが よわい。 ハチで はじまる。"),
         ("ヘイホー", "とても ラッキーだが まなぶのが おそく すこし よわい。 こおりの ロッドで はじまる。"),
         ("シャドウリンク", "リンクの かげ。 とても つよくて はやいが ラッキーが ひくく まなぶのが おそい。 けんで はじまる。")],
    ),
}
