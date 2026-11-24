"""Textes de l'ecran Commandes (reattribution des touches). Importe par gen_lang.py.
Japonais en kana uniquement (police du jeu)."""

ACTS = ["up", "down", "left", "right", "attack", "dash", "toggle_auto", "pause"]
KEYNAMES = ["space", "escape", "enter", "tab", "backspace", "up", "down", "left", "right", "shift", "ctrl",
            "alt", "mouse_left", "mouse_right", "mouse_middle", "dpad_up", "dpad_down", "dpad_left", "dpad_right"]
UI = ["ui.controls", "ui.tab.keyboard", "ui.tab.gamepad", "ui.press_key", "ui.press_button", "ui.rebind_hint",
      "ui.reset_keys", "ui.reset_keys_desc", "ui.keys_reset"]

KEYS = UI + [f"act.{a}" for a in ACTS] + [f"key.{k}" for k in KEYNAMES]


def table(ui, acts, keys):
    assert len(ui) == len(UI) and len(acts) == len(ACTS) and len(keys) == len(KEYNAMES)
    t = dict(zip(UI, ui))
    t.update({f"act.{a}": v for a, v in zip(ACTS, acts)})
    t.update({f"key.{k}": v for k, v in zip(KEYNAMES, keys)})
    return t


EXTRA = {
    "en": table(
        ["Controls", "Keyboard and mouse", "Gamepad", "Press a key or a mouse button... (Escape: cancel)",
         "Press a gamepad button... (Escape: cancel)", "Choose an action, then press its new key.",
         "Reset to default", "Puts back the original keys and buttons for every action.", "Default controls restored."],
        ["Move up", "Move down", "Move left", "Move right", "Attack", "Dash", "Auto aim on / off", "Pause"],
        ["Space", "Escape", "Enter", "Tab", "Backspace", "Up arrow", "Down arrow", "Left arrow", "Right arrow",
         "Shift", "Ctrl", "Alt", "Left click", "Right click", "Middle click", "D-pad up", "D-pad down",
         "D-pad left", "D-pad right"],
    ),
    "fr": table(
        ["Commandes", "Clavier et souris", "Manette", "Appuyez sur une touche ou un bouton de souris... (Échap : annuler)",
         "Appuyez sur un bouton de la manette... (Échap : annuler)", "Choisissez une action, puis appuyez sur sa nouvelle touche.",
         "Touches par défaut", "Remet les touches et boutons d'origine pour toutes les actions.", "Commandes d'origine rétablies."],
        ["Haut", "Bas", "Gauche", "Droite", "Attaque", "Esquive", "Visée auto oui / non", "Pause"],
        ["Espace", "Échap", "Entrée", "Tab", "Retour arrière", "Flèche haut", "Flèche bas", "Flèche gauche",
         "Flèche droite", "Maj", "Ctrl", "Alt", "Clic gauche", "Clic droit", "Clic molette", "Croix haut",
         "Croix bas", "Croix gauche", "Croix droite"],
    ),
    "de": table(
        ["Steuerung", "Tastatur und Maus", "Controller", "Drücke eine Taste oder Maustaste... (Esc: abbrechen)",
         "Drücke eine Controller-Taste... (Esc: abbrechen)", "Wähle eine Aktion und drücke dann ihre neue Taste.",
         "Standard", "Stellt die ursprünglichen Tasten für alle Aktionen wieder her.", "Standardsteuerung wiederhergestellt."],
        ["Hoch", "Runter", "Links", "Rechts", "Angriff", "Ausweichen", "Autozielen an / aus", "Pause"],
        ["Leertaste", "Esc", "Eingabe", "Tab", "Rücktaste", "Pfeil hoch", "Pfeil runter", "Pfeil links",
         "Pfeil rechts", "Umschalt", "Strg", "Alt", "Linksklick", "Rechtsklick", "Mittelklick", "Steuerkreuz hoch",
         "Steuerkreuz runter", "Steuerkreuz links", "Steuerkreuz rechts"],
    ),
    "es": table(
        ["Controles", "Teclado y ratón", "Mando", "Pulsa una tecla o un botón del ratón... (Esc: cancelar)",
         "Pulsa un botón del mando... (Esc: cancelar)", "Elige una acción y pulsa su nueva tecla.",
         "Por defecto", "Restaura las teclas y botones originales de todas las acciones.", "Controles originales restaurados."],
        ["Arriba", "Abajo", "Izquierda", "Derecha", "Ataque", "Esquivar", "Apuntado auto sí / no", "Pausa"],
        ["Espacio", "Esc", "Intro", "Tab", "Retroceso", "Flecha arriba", "Flecha abajo", "Flecha izquierda",
         "Flecha derecha", "Mayús", "Ctrl", "Alt", "Clic izquierdo", "Clic derecho", "Clic central", "Cruceta arriba",
         "Cruceta abajo", "Cruceta izquierda", "Cruceta derecha"],
    ),
    "it": table(
        ["Comandi", "Tastiera e mouse", "Controller", "Premi un tasto o un pulsante del mouse... (Esc: annulla)",
         "Premi un pulsante del controller... (Esc: annulla)", "Scegli un'azione, poi premi il nuovo tasto.",
         "Predefiniti", "Ripristina i tasti e i pulsanti originali di tutte le azioni.", "Comandi originali ripristinati."],
        ["Su", "Giù", "Sinistra", "Destra", "Attacco", "Schivata", "Mira automatica sì / no", "Pausa"],
        ["Spazio", "Esc", "Invio", "Tab", "Backspace", "Freccia su", "Freccia giù", "Freccia sinistra",
         "Freccia destra", "Maiusc", "Ctrl", "Alt", "Clic sinistro", "Clic destro", "Clic centrale", "Croce su",
         "Croce giù", "Croce sinistra", "Croce destra"],
    ),
    "pt": table(
        ["Controles", "Teclado e mouse", "Controle", "Aperte uma tecla ou um botão do mouse... (Esc: cancelar)",
         "Aperte um botão do controle... (Esc: cancelar)", "Escolha uma ação e aperte a nova tecla.",
         "Padrão", "Restaura as teclas e botões originais de todas as ações.", "Controles originais restaurados."],
        ["Cima", "Baixo", "Esquerda", "Direita", "Ataque", "Esquiva", "Mira automática sim / não", "Pausa"],
        ["Espaço", "Esc", "Enter", "Tab", "Backspace", "Seta para cima", "Seta para baixo", "Seta para a esquerda",
         "Seta para a direita", "Shift", "Ctrl", "Alt", "Clique esquerdo", "Clique direito", "Clique do meio",
         "Direcional cima", "Direcional baixo", "Direcional esquerda", "Direcional direita"],
    ),
    "ja": table(
        ["そうさ せってい", "キーボードと マウス", "コントローラー", "キーか マウスボタンを おして ください... (Esc: キャンセル)",
         "コントローラーの ボタンを おして ください... (Esc: キャンセル)", "アクションを えらんで あたらしい キーを おす。",
         "しょきせってい", "すべての アクションを さいしょの キーに もどす。", "さいしょの そうさに もどしました。"],
        ["うえ", "した", "ひだり", "みぎ", "こうげき", "ダッシュ", "オートエイム オン / オフ", "ポーズ"],
        ["スペース", "Esc", "エンター", "Tab", "バックスペース", "うえ キー", "した キー", "ひだり キー", "みぎ キー",
         "シフト", "Ctrl", "Alt", "ひだりクリック", "みぎクリック", "なかクリック", "じゅうじ うえ", "じゅうじ した",
         "じゅうじ ひだり", "じゅうじ みぎ"],
    ),
}
