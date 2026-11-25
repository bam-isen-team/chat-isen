flowchart TB
    %% Définition des styles (couleurs et bordures fidèles au schéma)
    classDef bluePkg fill:#dbeafe,stroke:#3b82f6,stroke-width:1.5px,color:#1e293b
    classDef whitePkg fill:#ffffff,stroke:#4b5563,stroke-width:1.5px,color:#1e293b
    classDef grayBonus fill:#f3f4f6,stroke:#6b7280,stroke-width:1.5px,stroke-dasharray: 5 5,color:#374151
    classDef greenPkg fill:#dcfce7,stroke:#22c55e,stroke-width:1.5px,color:#1e293b

    %% Package client externe
    ClientWeb["<b>client-web</b><hr/>Preact + CSS + JavaScript<br/>Interface (chat, appels)<br/>Client WebSocket<br/>Client WebRTC (getUserMedia, getStats)<br/>Tests : vitest"]:::bluePkg

    %% Conteneur principal du serveur
    subgraph Server ["Serveur ChatISEN (Rust, tests rstest)"]
        direction TB
        Realtime["<b>realtime</b><hr/>Passerelle WebSocket<br/>Diffusion des événements<br/>Signalisation WebRTC (offer / answer / ICE)"]:::whitePkg
        Attachments["<b>attachments</b><hr/>(BONUS)<br/>Attachment<br/>Stockage et partage sécurisés"]:::grayBonus
        Messaging["<b>messaging</b><hr/>Conversation,<br/>PrivateConversation,<br/>GroupConversation,<br/>Membership,<br/>Message"]:::whitePkg
        Calls["<b>calls</b><hr/>Call, QualityMetric<br/>Cycle de vie des appels 1-à-1"]:::whitePkg
        Monitoring["<b>monitoring</b><hr/>Métriques, logs, alertes<br/>Collecte des QualityMetric<br/>(Prometheus / Grafana ? à confirmer)"]:::greenPkg
        Auth["<b>auth</b><hr/>User, Session<br/>Inscription, connexion, profils<br/>Contrôle d'accès<br/>(sessions ou JWT : à confirmer)"]:::whitePkg
        Conferences["<b>conferences</b><hr/>(BONUS)<br/>Conférences multi-participants<br/>via un SFU (à confirmer)"]:::grayBonus
        Persistence["<b>persistence</b><hr/>Base de données PostgreSQL<br/>Dépôts d'accès aux données (Rust)<br/>Pub/Sub (Redis ? à confirmer)"]:::whitePkg
    end

    %% Packages externes (droite)
    DataAI["<b>data-ai</b><hr/>Analyse d'usage et de qualité<br/>Détection d'anomalies"]:::greenPkg
    Infra["<b>infrastructure</b><hr/>Hébergement : GCP ou VPS (à confirmer)<br/>CI/CD : GitHub Actions<br/>Conteneurs Docker (à confirmer)<br/>Reverse proxy HTTPS<br/>STUN / TURN (coturn ? à confirmer)"]:::greenPkg

    %% Dépendances et flux
    ClientWeb -.->|"WebSocket"| Realtime
    ClientWeb -.->|"REST"| Auth

    Realtime -.->|"«use»"| Messaging
    Realtime -.->|"«use»"| Calls

    Messaging -.->|"«use»"| Attachments
    Messaging -.->|"«use»"| Auth
    Messaging -.->|"«use»"| Persistence

    Calls -.->|"«use»"| Auth
    Calls -.->|"«use»"| Persistence
    Calls -.->|"«use»"| Monitoring

    Conferences -.->|"«use»"| Calls
    Auth -.->|"«use»"| Persistence
    DataAI -.->|"«use»"| Monitoring

    %% Style du conteneur principal
    style Server fill:#ffffff,stroke:#374151,stroke-width:1.5px,color:#1e293b
```[cite: 3]

*(Note : Mermaid ne disposant pas d'un type `packageDiagram` natif, ce code utilise un `flowchart` avec un sous-graphe `subgraph` et des séparateurs `<hr/>` pour reproduire fidèlement les onglets de packages, les bordures en pointillés et les dépendances `«use»`).*