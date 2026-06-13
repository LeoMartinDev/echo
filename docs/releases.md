# Releases et mises a jour automatiques

Le workflow GitHub Actions `.github/workflows/release.yml` publie une release a
chaque push sur `main` ou `master`, ainsi qu'en lancement manuel.

## Plateformes construites

- macOS Apple Silicon (`aarch64-apple-darwin`)
- Linux x64 (`ubuntu-22.04`)
- Linux arm64 (`ubuntu-22.04-arm`)
- Windows x64 (`windows-latest`)

macOS Intel est volontairement exclu.

## Secrets GitHub requis

Generer une paire de cles updater Tauri :

```bash
deno task tauri signer generate -w ~/.tauri/echo.key
```

Ajouter ensuite ces secrets dans GitHub:

- `TAURI_SIGNING_PRIVATE_KEY`: contenu de la cle privee generee
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: mot de passe de la cle, ou vide si aucun
- `TAURI_UPDATER_PUBLIC_KEY`: cle publique affichee par la commande

La cle publique est injectee dans `tauri.conf.json` uniquement pendant le build
CI. La cle privee sert a signer les bundles updater et ne doit jamais etre
versionnee.

## Versioning date

Pour permettre une vraie nouvelle release a chaque push, la CI calcule une seule
version UTC SemVer-compatible au debut du workflow, puis tous les builds de la
matrix reutilisent cette meme version:

```text
YYYY.M.DDHHMMSS
```

Par exemple, un build du 13 juin 2026 a 09:45:30 UTC produit
`2026.6.13094530`.

L'updater Tauri compare ces versions SemVer. Sans increment de version, une app
deja installee ne verrait pas de mise a jour.

## Fonctionnement updater

En build release, l'app verifie au demarrage:

```text
https://github.com/LeoMartinDev/echo/releases/latest/download/latest.json
```

Si une version plus recente existe, elle est telechargee, installee, puis l'app
redemarre automatiquement.
