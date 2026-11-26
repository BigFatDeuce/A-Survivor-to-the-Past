"""Textes de la version 0.6.2, fin : fusion mise en scene, ecran des deblocages de fin de partie, armes
debloquees par les mini-boss (annonce en jeu, indice dans le recueil). Importe par gen_lang.py.
Japonais en kana uniquement (police du jeu) ; pas de guillemets doubles ni de chevrons."""

KEYS = ["ui.unlocks_title", "ui.unlock_kind_char", "ui.unlock_kind_weapon", "ui.unlocks_chars_note",
        "ui.unlocks_weapons_note", "ui.weapon_unlocked", "ui.weapon_unlocked_sub", "ui.weapon_locked_hint",
        "ui.evo_fusion", "ui.fusions_title"]


def table(values):
    assert len(values) == len(KEYS)
    return dict(zip(KEYS, values))


EXTRA = {
    "fr": table(["Déblocages !", "Personnage", "Arme",
                 "Nouveaux personnages jouables : choisissez-les avant la partie.",
                 "Ces armes peuvent maintenant apparaître dans toutes les parties.",
                 "Nouvelle arme débloquée !", "Elle peut maintenant apparaître dans toutes les parties.",
                 "Vainquez {a} pour la débloquer.", "Fusion : {a} + {b}", "Fusions possibles"]),
    "en": table(["Unlocked!", "Character", "Weapon",
                 "New playable characters: pick them before a run.",
                 "These weapons can now appear in every run.",
                 "New weapon unlocked!", "It can now appear in every run.",
                 "Defeat {a} to unlock it.", "Fusion: {a} + {b}", "Possible fusions"]),
    "de": table(["Freigeschaltet!", "Figur", "Waffe",
                 "Neue spielbare Figuren: wähle sie vor dem Lauf.",
                 "Diese Waffen können jetzt in jedem Lauf erscheinen.",
                 "Neue Waffe freigeschaltet!", "Sie kann jetzt in jedem Lauf erscheinen.",
                 "Besiege {a}, um sie freizuschalten.", "Fusion: {a} + {b}", "Mögliche Fusionen"]),
    "es": table(["¡Desbloqueado!", "Personaje", "Arma",
                 "Nuevos personajes jugables: elígelos antes de la partida.",
                 "Estas armas ya pueden aparecer en todas las partidas.",
                 "¡Nueva arma desbloqueada!", "Ya puede aparecer en todas las partidas.",
                 "Derrota a {a} para desbloquearla.", "Fusión: {a} + {b}", "Fusiones posibles"]),
    "it": table(["Sbloccati!", "Personaggio", "Arma",
                 "Nuovi personaggi giocabili: sceglili prima della partita.",
                 "Queste armi ora possono comparire in tutte le partite.",
                 "Nuova arma sbloccata!", "Ora può comparire in tutte le partite.",
                 "Sconfiggi {a} per sbloccarla.", "Fusione: {a} + {b}", "Fusioni possibili"]),
    "pt": table(["Desbloqueado!", "Personagem", "Arma",
                 "Novos personagens jogáveis: escolha-os antes da partida.",
                 "Estas armas agora podem aparecer em todas as partidas.",
                 "Nova arma desbloqueada!", "Agora pode aparecer em todas as partidas.",
                 "Derrote {a} para desbloqueá-la.", "Fusão: {a} + {b}", "Fusões possíveis"]),
    "ja": table(["かいほう!", "キャラクター", "ぶき",
                 "あたらしい キャラクターが つかえる。 プレイの まえに えらぼう。",
                 "これらの ぶきは どの プレイでも でるように なった。",
                 "あたらしい ぶきを かいほう!", "どの プレイでも でるように なった。",
                 "{a}を たおすと かいほう", "がったい {a} + {b}", "できる がったい"]),
}
