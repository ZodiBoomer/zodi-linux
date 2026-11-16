# Zodi Linux

Dépôt consacré à l'environnement Linux du projet Zodi AI et à son intégration avec le PC de l'utilisateur.

## Périmètre
- Documentation d'installation et de configuration Linux.
- Scripts de démarrage et d'arrêt contrôlés.
- Services système nécessaires à Zodi AI.
- Gestion des dépendances et vérifications de santé.
- Procédures de sauvegarde, diagnostic et retour arrière.

## Principes
- Ne pas modifier le démarrage, le partitionnement ou les données du PC sans validation explicite.
- Utiliser des permissions minimales et documenter toute élévation de privilèges.
- Prévoir une désinstallation et un retour arrière.
- Ne jamais commiter de clés, mots de passe, jetons ou fichiers d'environnement.

## État initial
Dépôt documentaire initialisé. Aucun script système n'est installé ni exécuté par ce dépôt à ce stade.

## Dépendances du projet
Le moteur central est dans https://github.com/ZodiBoomer/zodi-ai. Les utilitaires partagés sont dans https://github.com/ZodiBoomer/zodi-tools.
