table! {
    giveaway (id) {
        id -> Int4,
        description -> Text,
        participants -> Jsonb,
        finished -> Bool,
        created_at -> Timestamptz,
    }
}

table! {
    giveaway_object (id) {
        id -> Int4,
        giveaway_id -> Int4,
        value -> Text,
        object_type -> Varchar,
        object_state -> Varchar,
    }
}

joinable!(giveaway_object -> giveaway (giveaway_id));

allow_tables_to_appear_in_same_query!(giveaway, giveaway_object,);
