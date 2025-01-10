// @generated automatically by Diesel CLI.

diesel::table! {
    html_metadata (id) {
        id -> Nullable<Integer>,
        file_path -> Text,
        html_content -> Text,
    }
}
