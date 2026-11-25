classDiagram
    %% Classes principales
    class Session {
        -id : Uuid
        -token : String (session ou JWT : à confirmer)
        -createdAt : DateTime
        -expiresAt : DateTime
        +isValid() bool
        +revoke()
    }

    class User {
        -id : Uuid
        -username : String
        -email : String
        -passwordHash : String
        -displayName : String
        -avatarUrl : String
        -presenceStatus : PresenceStatus
        -createdAt : DateTime
        +updateProfile()
        +changePassword()
    }

    class Call {
        -id : Uuid
        -callType : CallType &#123;VOICE, VIDEO&#125;
        -status : CallStatus &#123;PENDING, ONGOING, ENDED, REJECTED, MISSED&#125;
        -startedAt : DateTime
        -endedAt : DateTime
        +accept()
        +reject()
        +hangUp()
        +duration() int
    }

    class QualityMetric {
        -timestamp : DateTime
        -latencyMs : f32
        -packetLoss : f32
        -jitterMs : f32
        (source : WebRTC getStats)
    }

    class Membership {
        -role : MemberRole &#123;MEMBER, ADMIN&#125;
        -joinedAt : DateTime
        -lastReadMessageId : Uuid
    }

    class Message {
        -id : Uuid
        -content : String
        -sentAt : DateTime
        -status : MessageStatus &#123;SENT, DELIVERED, READ&#125;
        +markAsRead()
    }

    class Attachment["Attachment (BONUS)"] {
        -id : Uuid
        -fileName : String
        -size : u64
        -mimeType : String
        -storageUrl : String
    }

    class Conversation {
        <<abstract>>
        -id : Uuid
        -createdAt : DateTime
        +isMember(user : User) bool
        +addMessage(message : Message)
        +history(since : DateTime) Vec~Message~
    }

    class PrivateConversation {
        (exactly 2 members)
    }

    class GroupConversation {
        -name : String
        +addMember(user : User)
        +removeMember(user : User)
    }

    %% Relations et cardinalités
    User "1" -- "0..*" Session : owns
    User "1" -- "0..*" Call : callee
    User "1" -- "0..*" Call : caller
    Call "1" *-- "0..*" QualityMetric : metrics

    User "1" -- "0..*" Membership
    Conversation "1" -- "0..*" Membership

    User "1" -- "0..*" Message : sender
    Conversation "1" *-- "0..*" Message : contains
    Message "1" .. "0..*" Attachment : attachment (bonus)

    Conversation <|-- PrivateConversation
    Conversation <|-- GroupConversation

    %% Note d'implémentation
    note for Conversation "Implémentation : structs Rust (backend) persistées dans PostgreSQL.\nRust n'a pas d'héritage : Conversation sera plutôt un enum ou un trait."

    %% Style spécifique pour la classe bonus en pointillés
    style Attachment fill:#f3f4f6,stroke:#6b7280,stroke-dasharray: 5 5
```[cite: 2]