// @generated automatically by Diesel CLI.

diesel::table! {
    file_content (rowid) {
        rowid -> Integer,
        id -> Integer,
        content -> Text,
    }
}

diesel::table! {
    metadata (id) {
        id -> Integer,
        file_path -> Text,
    }
}

diesel::joinable!(file_content -> metadata (id));

diesel::allow_tables_to_appear_in_same_query!(
    file_content,
    metadata,
);
