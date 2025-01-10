// @generated automatically by Diesel CLI.

diesel::table! {
    html_content (id) {
        id -> Nullable<Integer>,
        content -> Text,
    }
}

diesel::table! {
    html_metadata (id) {
        id -> Nullable<Integer>,
        file_path -> Text,
    }
}

diesel::joinable!(html_content -> html_metadata (id));

diesel::allow_tables_to_appear_in_same_query!(html_content, html_metadata,);
