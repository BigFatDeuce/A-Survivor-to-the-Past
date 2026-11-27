"""Textes de la version 0.6.5 : personnages du A Link to the Past Randomizer (univers, boutique,
auteurs des sprites, couleurs). Importe par gen_lang.py. Une ligne par cle : fr, en, de, es, it, pt,
ja. Japonais en kana uniquement (police du jeu) ; pas de guillemets doubles ni de chevrons.
Les noms et descriptions des personnages sont generes : lang_zspr.py (gen_zspr_chars.py)."""
import lang_zspr

CODES = ["fr", "en", "de", "es", "it", "pt", "ja"]

ROWS = {
    # ---- univers (pages de l'ecran des personnages) ----
    "group.zelda": ["Zelda", "Zelda", "Zelda", "Zelda", "Zelda", "Zelda", "ゼルダ"],
    "group.nintendo": ["Nintendo", "Nintendo", "Nintendo", "Nintendo", "Nintendo", "Nintendo", "ニンテンドー"],
    "group.rpg": ["RPG", "RPG", "RPG", "RPG", "RPG", "RPG", "RPG"],
    "group.games": ["Jeux vidéo", "Video games", "Videospiele", "Videojuegos", "Videogiochi", "Videogames", "ゲーム"],
    "group.screen": ["Séries et films", "TV and film", "Serien und Filme", "Series y cine", "Serie e film", "Séries e filmes",
                     "アニメと えいが"],
    "group.community": ["Communauté", "Community", "Community", "Comunidad", "Comunità", "Comunidade", "コミュニティ"],
    # ---- boutique ----
    "ui.tab.characters": ["Personnages", "Characters", "Figuren", "Personajes", "Personaggi", "Personagens", "キャラ"],
    "ui.unlock_shop": ["En vente à la boutique : {n} rubis rouges.", "For sale in the shop: {n} red rupees.",
                       "Im Laden erhältlich: {n} rote Rubine.", "A la venta en la tienda: {n} rupias rojas.",
                       "In vendita al negozio: {n} rupie rosse.", "À venda na loja: {n} rupias vermelhas.",
                       "ショップで うっている: あかい ルピー {n}"],
    "ui.char_buy": ["A : l'acheter.", "A: buy.", "A: kaufen.", "A: comprar.", "A: comprare.", "A: comprar.", "A: かう"],
    "ui.char_bought": ["À choisir dans l'écran des personnages.", "Pick it in character select.",
                       "In der Figurenwahl auswählen.", "Elígelo al elegir personaje.", "Sceglilo nella scelta personaggio.",
                       "Escolha na seleção de personagem.", "キャラせんたくで えらべる"],
    "ui.char_credit": ["Sprite : {s}.", "Sprite: {s}.", "Sprite: {s}.", "Sprite: {s}.", "Sprite: {s}.", "Sprite: {s}.",
                       "スプライト: {s}"],
    # ---- couleurs des palettes (cottes de mailles du Randomizer) ----
    "skin.white": ["Blanc", "White", "Weiß", "Blanco", "Bianco", "Branco", "しろ"],
    "skin.black": ["Noir", "Black", "Schwarz", "Negro", "Nero", "Preto", "くろ"],
    "skin.gray": ["Gris", "Gray", "Grau", "Gris", "Grigio", "Cinza", "はいいろ"],
    "skin.brown": ["Brun", "Brown", "Braun", "Marrón", "Marrone", "Marrom", "ちゃいろ"],
    # forme du Monde des Tenebres (le lapin de Link ; chaque sprite du Randomizer a la sienne)
    "skin.dark": ["Monde des Ténèbres", "Dark World", "Schattenwelt", "Mundo Oscuro", "Mondo Oscuro", "Mundo Sombrio", "やみの せかい"],
    "skin.p1": ["Tenue 1", "Outfit 1", "Outfit 1", "Traje 1", "Abito 1", "Traje 1", "ふく 1"],
    "skin.p2": ["Tenue 2", "Outfit 2", "Outfit 2", "Traje 2", "Abito 2", "Traje 2", "ふく 2"],
    "skin.p3": ["Tenue 3", "Outfit 3", "Outfit 3", "Traje 3", "Abito 3", "Traje 3", "ふく 3"],
}
ROWS.update(lang_zspr.ROWS)

KEYS = list(ROWS)
EXTRA = {code: {k: v[i] for k, v in ROWS.items()} for i, code in enumerate(CODES)}
assert all(len(v) == len(CODES) for v in ROWS.values())
