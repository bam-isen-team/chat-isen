flowchart TB
    %% Définition des styles (couleurs fidèles au schéma)
    classDef blueBox fill:#dbeafe,stroke:#3b82f6,stroke-width:1.5px,color:#1e293b
    classDef grayBonus fill:#f3f4f6,stroke:#6b7280,stroke-width:1.5px,stroke-dasharray: 5 5,color:#374151
    classDef orangeBox fill:#ffedd5,stroke:#f97316,stroke-width:1.5px,color:#1e293b
    classDef greenBox fill:#dcfce7,stroke:#22c55e,stroke-width:1.5px,color:#1e293b

    %% Nœuds principaux
    UserA["<b>Utilisateur A</b><br/>Client web Preact<br/>(navigateur)<br/>UI chat + appels WebRTC"]:::blueBox
    UserB["<b>Utilisateur B</b><br/>Client web Preact<br/>(navigateur)<br/>UI chat + appels WebRTC"]:::blueBox

    SFU["BONUS : SFU<br/>conférences<br/>multi-participants<br/>(mediasoup / LiveKit ?)"]:::grayBonus
    Proxy["Reverse proxy HTTPS /<br/>WSS (TLS)<br/>(Nginx ou Traefik : à<br/>confirmer)"]:::blueBox
    Stun["STUN / TURN<br/>(coturn ? à confirmer)<br/>traversée NAT des appels"]:::orangeBox

    API["API REST +<br/>Authentification (Rust)<br/>inscription, connexion,<br/>sessions<br/>(sessions ou JWT : à<br/>confirmer)"]:::blueBox
    Realtime["Serveur temps réel (Rust,<br/>WebSocket)<br/>messagerie privée /<br/>groupe<br/>+ signalisation WebRTC"]:::blueBox
    Supervision["Supervision de base<br/>métriques, logs, alertes<br/>(Prometheus / Grafana ? à<br/>confirmer)"]:::greenBox

    Storage["BONUS : stockage de<br/>fichiers<br/>partage de fichiers<br/>sécurisé"]:::grayBonus
    DB["Base de données<br/>PostgreSQL<br/>utilisateurs,<br/>conversations, historique"]:::blueBox
    PubSub["Pub/Sub (Redis ? à<br/>confirmer)<br/>si plusieurs instances<br/>temps réel"]:::blueBox
    DataAI["Data & IA (investigation)<br/>qualité d'appel (getStats),<br/>usage<br/>détection d'anomalies"]:::greenBox

    Deploy["Déploiement reproductible<br/>CI/CD GitHub Actions (tests vitest + rstest)<br/>Hébergement : GCP ou VPS (à confirmer)<br/>Conteneurs Docker (à confirmer)"]:::greenBox

    %% Connexions et flux
    UserA <-->|"Média WebRTC pair-à-pair (SRTP)"| UserB
    UserA -.->|"bonus"| SFU
    UserA -->|"HTTPS / WSS"| Proxy
    UserB -->|"HTTPS / WSS"| Proxy
    UserB -.->|"ICE / relais"| Stun

    Proxy -->|"REST"| API
    Proxy -->|"WebSocket"| Realtime

    API -.->|"bonus"| Storage
    API -->|"comptes"| DB
    Realtime -->|"historique"| DB
    Realtime -.->|"pub/sub"| PubSub

    Realtime -.->|"métriques / logs (tous les services)"| Supervision
    Supervision -->|"données"| DataAI

    %% Style personnalisé des flèches spécifiques (orange et vert)
    linkStyle 0,4 stroke:#f97316,stroke-width:1.5px,stroke-dasharray: 5 5
    linkStyle 11,12 stroke:#22c55e,stroke-width:1.5px