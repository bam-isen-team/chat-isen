flowchart LR
    %% Définition des styles
    classDef actor fill:#ffffff,stroke:#374151,stroke-width:1.5px,color:#1e293b
    classDef usecase fill:#ffffff,stroke:#374151,stroke-width:1.5px,color:#1e293b
    classDef bonus fill:#f3f4f6,stroke:#6b7280,stroke-width:1.5px,stroke-dasharray: 5 5,color:#374151

    %% Acteurs de gauche
    Visiteur(["👤 Visiteur"]):::actor
    UserLeft(["👤 Utilisateur connecté"]):::actor

    %% Conteneur du système
    subgraph System ["Système ChatISEN"]
        direction TB
        
        %% Colonne gauche des cas d'utilisation
        UC_Register(["S'inscrire"]):::usecase
        UC_Auth(["Se connecter / se déconnecter"]):::usecase
        UC_Profile(["Gérer son profil"]):::usecase
        UC_PrivChat(["Démarrer une conversation privée"]):::usecase
        UC_Group(["Créer un groupe et gérer ses membres"]):::usecase
        UC_SendMsg(["Envoyer un message"]):::usecase
        UC_RecvMsg(["Recevoir les messages en temps réel"]):::usecase
        UC_History(["Consulter l'historique"]):::usecase

        %% Cas d'utilisation centraux (include / extend)
        UC_CheckAccess(["Vérifier l'accès à la conversation"]):::usecase
        UC_Mute(["Couper micro / caméra"]):::usecase

        %% Colonne droite des cas d'utilisation
        UC_VoiceCall(["Passer un appel vocal 1-à-1"]):::usecase
        UC_VideoCall(["Passer un appel vidéo 1-à-1"]):::usecase
        UC_AnswerCall(["Répondre à / refuser un appel"]):::usecase
        UC_Conf(["BONUS : rejoindre une conférence"]):::bonus
        UC_File(["BONUS : partager un fichier"]):::bonus
        UC_Monitor(["Superviser la plateforme (métriques, logs, alertes)"]):::usecase
    end

    %% Acteurs de droite
    UserRight(["👤 Utilisateur connecté"]):::actor
    Ops(["👤 Équipe d'exploitation"]):::actor

    %% Relations Acteurs <-> Cas d'utilisation (Gauche)
    Visiteur --- UC_Register
    Visiteur --- UC_Auth

    UserLeft --- UC_Auth
    UserLeft --- UC_Profile
    UserLeft --- UC_PrivChat
    UserLeft --- UC_Group
    UserLeft --- UC_SendMsg
    UserLeft --- UC_RecvMsg
    UserLeft --- UC_History

    %% Relations internes («include» et «extend»)
    UC_SendMsg -.->|"«include»"| UC_CheckAccess
    UC_History -.->|"«include»"| UC_CheckAccess

    UC_Mute -.->|"«extend»"| UC_VoiceCall
    UC_Mute -.->|"«extend»"| UC_VideoCall

    %% Relations Cas d'utilisation <-> Acteurs (Droite)
    UC_VoiceCall --- UserRight
    UC_VideoCall --- UserRight
    UC_AnswerCall --- UserRight
    UC_Conf --- UserRight
    UC_File --- UserRight

    UC_Monitor --- Ops

    %% Style du cadre système
    style System fill:#ffffff,stroke:#374151,stroke-width:1.5px,color:#1e293b
```[cite: 4]

*(Note : Mermaid ne propose pas encore de diagramme de cas d'utilisation UML natif. Ce code utilise un `flowchart` gauche-droite (`LR`) avec des nœuds arrondis `(["..."])` pour reproduire fidèlement les ovales, les relations `«include»` / `«extend»`, les éléments bonus en pointillés et la symétrie des acteurs)*[cite: 4].