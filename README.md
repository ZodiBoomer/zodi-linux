# Zodi AI pour Linux

Client de bureau **natif GTK4 + Libadwaita**, écrit en Rust. Il communique directement avec l'API Zodi AI en HTTPS : il n'ouvre pas le site Web et ne dépend d'aucun compte ou service Google.

## Fonctionnalités initiales

- Création de compte et connexion par adresse e-mail + mot de passe.
- Jeton natif Bearer révocable, délivré par le backend Zodi.
- Chargement des conversations sauvegardées.
- Création d'une nouvelle discussion et envoi de messages à Zodi.
- Déconnexion avec révocation du jeton.

Le mot de passe est transmis uniquement en HTTPS à l'API Zodi ; il n'est pas enregistré par l'application. Le jeton reste en mémoire pendant la session Linux et est effacé à la fermeture. Le stockage sécurisé persistant du jeton sera ajouté dans une amélioration ultérieure.

## Prérequis de compilation

Ubuntu 24.04 ou distribution récente avec GTK4 et Libadwaita :

```bash
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev
```

Installer Rust stable, puis compiler :

```bash
cargo check
cargo build --release
```

## Installation Debian

Après compilation :

```bash
cargo install cargo-deb
cargo deb --no-build
sudo apt install ./target/debian/zodi-ai-linux_0.1.0_amd64.deb
```

## Configuration

Par défaut, le client utilise `https://zodi-ai-core.vercel.app`. Pour tester un backend différent :

```bash
ZODI_API_URL=https://ton-backend.example zodi-ai
```

Aucun secret serveur ni clé fournisseur IA n'est intégré au binaire.

## Backend requis

Le backend doit exposer :

- `POST /api/mobile/auth/register`
- `POST /api/mobile/auth/login`
- `POST /api/mobile/auth/logout`
- `GET/POST /api/conversations`
- `GET /api/conversations/:id`
- `POST /api/chat`

Les endpoints protégés acceptent `Authorization: Bearer <token>`.

## Tests et limites

La CI vérifie la compilation GTK4/Libadwaita et construit le paquet Debian. Le test réel de connexion nécessite un backend Preview avec les migrations appliquées et des identifiants de test. Le jeton n'est pas encore conservé entre deux lancements.
