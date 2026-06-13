# Greffe

Dictée vocale **100 % locale** : maintenez un raccourci clavier, parlez, le texte
s'écrit dans l'input de l'application active. Une petite bulle animée s'affiche
en bas de l'écran pendant que vous parlez.

- **Tauri 2 + Svelte 5 + Deno** — fonctionne sur macOS, Windows et Linux.
- **Modèles locaux interchangeables**, un par usage : **Parakeet TDT 0.6B v3**
  (recommandé — ONNX, ~25× temps réel en CPU, 25 langues), **Whisper Large v3
  Turbo** (précision max, ~100 langues, GPU/Metal) et **Whisper Small** (léger,
  machines modestes). Téléchargement et changement depuis les réglages.
- **Multilingue** : détection automatique de la langue ou langue forcée.

## Fonctionnement

1. Maintenez le raccourci (par défaut **F9**).
2. La bulle apparaît en bas au centre et s'anime avec votre voix ; la
   transcription partielle s'y affiche en direct.
3. Le texte est tapé **en direct** dans le champ focalisé (mode « En direct »),
   puis réconcilié au relâchement avec la transcription finale. Le mode
   « À la fin » tape tout d'un bloc au relâchement.

## Développement

```bash
deno install          # dépendances frontend
deno task tauri dev   # lance l'app (compile le backend Rust)
```

Build de production : `deno task tauri build`.

## Permissions

- **macOS** : autoriser le **micro** (demandé au premier enregistrement) et
  l'**Accessibilité** (Réglages Système → Confidentialité → Accessibilité),
  nécessaire pour taper le texte dans les autres apps. L'app vous le propose.
- **Linux** : la frappe simulée requiert `libxdo` (X11). Sous Wayland, le
  support dépend du compositeur.
- **Windows** : aucune permission particulière.

## Notes

- Les modèles sont stockés dans le dossier de données de l'app
  (`~/Library/Application Support/com.leomartin.greffe/models` sur macOS).
- Si votre raccourci contient un modificateur (Cmd/Ctrl/Alt), préférez le mode
  d'insertion « À la fin » : taper du texte pendant que le modificateur est
  physiquement enfoncé peut déclencher des raccourcis dans l'app cible. Une
  touche seule type F9 ne pose aucun problème en mode « En direct ».
- Whisper ne « streame » pas nativement : les partiels sont décodés sur une
  fenêtre glissante (~8 s max) toutes les ~1 s ; aux pauses de parole, le texte
  décodé est « engagé » et la fenêtre repart, ce qui garde une latence
  constante même sur les longues dictées. Seule la partie stable entre deux
  décodages est tapée en direct, le décodage final corrige la fin.
