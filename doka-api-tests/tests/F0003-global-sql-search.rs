mod test_lib;

const TEST_TO_RUN: &[&str] = &[
    "t10_f0003_like_with_escapes_and_numeric",
];

#[cfg(test)]
mod f0003_global_sql_search_tests {
    use rand::Rng;

    use dkdto::api_error::ApiError;
    use dkdto::web_types::{AddItemRequest, AddTagValue, EnumTagValue};
    use doka_cli::request_client::{AdminServerClient, DocumentServerClient};

    use crate::TEST_TO_RUN;
    use crate::test_lib::{Lookup, get_login_request};

    /// TC-F0003-001 — Composite expression: `LIKE` with escape rules AND a
    /// numeric branch. End-to-end black-box validation through the
    /// `document-server` HTTP API that F0001 / F0002 / F0003 escaping is
    /// honoured all the way down to the generated SQL.
    ///
    /// Seeded items:
    ///   A: name = "50 % de l'élite ranking", age = 25  -> must match
    ///   B: name = "50X de l'élite ranking" , age = 25  -> must be excluded
    ///   C: name = "50 % de l'élite ranking", age = 15  -> must be excluded
    ///
    /// Filter: ( <tag_text> LIKE "50 #% de l'élite%" ) AND ( <tag_int> >= 18 )
    #[test]
    fn t10_f0003_like_with_escapes_and_numeric() -> Result<(), ApiError<'static>> {
        let lookup = Lookup::new("t10_f0003_like_with_escapes_and_numeric", TEST_TO_RUN);
        let props = lookup.props();

        let admin_server = AdminServerClient::new("localhost", 30060);
        let login_reply = admin_server.login(&get_login_request(&props))?;
        let document_server = DocumentServerClient::new("localhost", 30070);

        // Random tag names per run to avoid collisions on the shared customer.
        let tag_text = generate_random_tag();
        let tag_int = generate_random_tag();

        // Three items packing every escape axis into the text value:
        //   - a literal `%` (matched by the `#%` of the DFS filter)
            //   - a `'` inside `l'élite` (must be doubled in SQL by F0002 rule 1)
        //   - a trailing free sequence (caught by the trailing wildcard)
        let item_a = create_item(
            &document_server,
            &login_reply.session_id,
            "Item A (should match)",
            &tag_text,
            "50 % de l'élite ranking",
            &tag_int,
            25,
        )?;
        let item_b = create_item(
            &document_server,
            &login_reply.session_id,
            "Item B (no literal %)",
            &tag_text,
            "50X de l'élite ranking",
            &tag_int,
            25,
        )?;
        let item_c = create_item(
            &document_server,
            &login_reply.session_id,
            "Item C (age < 18)",
            &tag_text,
            "50 % de l'élite ranking",
            &tag_int,
            15,
        )?;

        let filter = format!(
            r#"({0} LIKE "50 #% de l'élite%") AND ({1} >= 18)"#,
            &tag_text, &tag_int
        );
        eprintln!("F0003 filter: {}", &filter);

        // Assertion 4 (implicit): the HTTP call must not error out.
        // If it does, the assembled SQL is malformed (typically because a
        // `'` was not doubled, or `ESCAPE '\'` ended up nested inside
        // `unaccent_lower(...)`).
        let reply = document_server.search_item(Some(&filter), &login_reply.session_id)?;

        eprintln!("F0003 reply items: {}", reply.items.len());
        for it in &reply.items {
            eprintln!("  - item_id={}", it.item_id);
        }

        // Assertion 1: item A is returned.
        // Covers: `'` doubling + `#%` -> `\%` + bare `%` wildcard + ESCAPE
        // clause sitting outside `unaccent_lower(...)`.
        assert!(
            reply.items.iter().any(|it| it.item_id == item_a.item_id),
            "expected item A (literal '%', age 25) to match filter [{}]",
            &filter
        );

        // Assertion 2: item B is excluded.
        // Covers: `#%` is enforced as literal `%`, not as a wildcard.
        assert!(
            reply.items.iter().all(|it| it.item_id != item_b.item_id),
            "expected item B (no literal '%') to be excluded — \\% must be enforced"
        );

        // Assertion 3: item C is excluded.
        // Covers: numeric branch honoured; no ESCAPE leak into the int sub-query.
        assert!(
            reply.items.iter().all(|it| it.item_id != item_c.item_id),
            "expected item C (age=15) to be excluded by the numeric branch"
        );

        lookup.close();
        Ok(())
    }

    /// Helper: create an item with one text tag and one int tag.
    fn create_item(
        document_server: &DocumentServerClient,
        session_id: &str,
        item_name: &str,
        tag_text: &str,
        tag_text_value: &str,
        tag_int: &str,
        tag_int_value: i64,
    ) -> Result<dkdto::web_types::AddItemReply, ApiError<'static>> {
        let text_property = AddTagValue {
            tag_id: None,
            tag_name: Some(tag_text.to_string()),
            value: EnumTagValue::Text(Some(tag_text_value.to_string())),
        };
        let int_property = AddTagValue {
            tag_id: None,
            tag_name: Some(tag_int.to_string()),
            value: EnumTagValue::Integer(Some(tag_int_value)),
        };
        let request = AddItemRequest {
            name: item_name.to_string(),
            file_ref: None,
            properties: Some(vec![text_property, int_property]),
        };
        document_server.create_item(&request, session_id)
    }

    fn generate_random_tag() -> String {
        let mut rng = rand::thread_rng();
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
        let random_string: String = (0..5).map(|_| chars[rng.gen_range(0..chars.len())]).collect();
        format!("tag_{}", random_string)
    }
}
