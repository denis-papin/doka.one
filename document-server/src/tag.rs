
use std::collections::HashMap;
use anyhow::anyhow;
use commons_services::token_lib::SessionToken;
use rocket::{post};
use rocket_contrib::json::Json;
use commons_error::*;
use log::{error, info, debug};
use commons_pg::{CellValue, iso_to_date, iso_to_datetime, SQLChange, SQLConnection, SQLDataSet, SQLQueryBlock, SQLTransaction};
use commons_services::database_lib::open_transaction;
use commons_services::session_lib::fetch_entry_session;
use commons_services::x_request_id::{Follower, XRequestID};
use dkdto::error_codes::{INCORRECT_DEFAULT_BOOLEAN_VALUE, INCORRECT_DEFAULT_DATE_VALUE, INCORRECT_DEFAULT_DATETIME_VALUE, INCORRECT_DEFAULT_DOUBLE_VALUE, INCORRECT_DEFAULT_INTEGER_VALUE, INCORRECT_DEFAULT_STRING_LENGTH, INCORRECT_STRING_LENGTH, INCORRECT_TAG_TYPE, INTERNAL_DATABASE_ERROR, INTERNAL_TECHNICAL_ERROR, INVALID_TOKEN, STILL_IN_USE, SUCCESS};
use dkdto::{AddTagReply, AddTagRequest, GetTagReply, JsonErrorSet, TagElement};
use dkdto::error_replies::ErrorReply;
use doka_cli::request_client::TokenType;


pub(crate) struct TagDelegate {
    pub session_token: SessionToken,
    pub follower: Follower,
}

impl TagDelegate {
    pub fn new(session_token: SessionToken, x_request_id: XRequestID) -> Self {
        Self {
            session_token,
            follower: Follower {
                x_request_id: x_request_id.new_if_null(),
                token_type: TokenType::None,
            }
        }
    }

    ///
    /// ✨ Find all the existing tags by pages
    ///
    pub fn get_all_tag(mut self, start_page : Option<u32>, page_size : Option<u32>) -> Json<GetTagReply> {

        log_info!("🚀 Start get_all_tag api, follower=[{}]", &self.follower);

        // Check if the token is valid
        if !self.session_token.is_valid() {
            log_error!("Invalid session token=[{:?}], follower=[{}]", &self.session_token, &self.follower);
            return Json(GetTagReply::invalid_token_error_reply());
        }

        self.follower.token_type = TokenType::Sid(self.session_token.0.clone());

        // Read the session information
        let Ok(entry_session) = fetch_entry_session(&self.follower.token_type.value())
                                            .map_err(err_fwd!("💣 Session Manager failed, follower=[{}]", &self.follower)) else {
            return Json(GetTagReply::internal_technical_error_reply());
        };

        // Query the items
        let internal_database_error_reply = Json(GetTagReply{ tags: vec![], status : JsonErrorSet::from(INTERNAL_DATABASE_ERROR) });

        let mut r_cnx = SQLConnection::new();
        let r_trans = open_transaction(&mut r_cnx).map_err(err_fwd!("💣 Open transaction error, follower=[{}]", &self.follower));
        let Ok(mut trans) = r_trans else {
            return internal_database_error_reply;
        };

        let Ok(tags) = self.search_tag_by_id(&mut trans, None, start_page, page_size, &entry_session.customer_code )
                                    .map_err(err_fwd!("💣 Cannot find the tag by id, follower=[{}]", &self.follower)) else
        {
            return internal_database_error_reply;
        };

        if trans.commit().map_err(err_fwd!("💣 Commit failed")).is_err() {
            return internal_database_error_reply;
        }

        log_info!("🏁 End get_all_tag api, follower=[{}]", &self.follower);

        Json(GetTagReply{
            tags,
            status: JsonErrorSet::from(SUCCESS),
        })
    }


    /// Search items by id
    /// If no item id provided, return all existing items
    fn search_tag_by_id(&self, mut trans : &mut SQLTransaction, tag_id: Option<i64>,
                        start_page : Option<u32>, page_size : Option<u32>,
                        customer_code : &str) -> anyhow::Result<Vec<TagElement>> {

        let p_tag_id = CellValue::Int(tag_id);

        let mut params = HashMap::new();
        params.insert("p_tag_id".to_owned(), p_tag_id);

        let sql_query = format!(r"SELECT id, name, type, string_tag_length, default_value
                                    FROM cs_{}.tag_definition
                                    WHERE ( id = :p_tag_id OR :p_tag_id IS NULL )
                                    ORDER BY name ", customer_code );

        let query = SQLQueryBlock {
            sql_query,
            start : start_page.unwrap_or(0) * page_size.unwrap_or(0),
            length : page_size,
            params,
        };

        let mut sql_result : SQLDataSet =  query.execute(&mut trans)
                    .map_err(err_fwd!("Query failed, sql=[{}], follower=[{}]", &query.sql_query, &self.follower))?;

        let mut tags = vec![];
        while sql_result.next() {
            let id : i64 = sql_result.get_int("id").ok_or(anyhow!("Wrong id"))?;
            let name : String = sql_result.get_string("name").ok_or(anyhow!("Wrong name"))?;
            let tag_type= sql_result.get_string("type").ok_or(anyhow!("Wrong tag_type"))?;
            // optional
            let string_tag_length = sql_result.get_int_32("string_tag_length");
            let default_value= sql_result.get_string("default_value");

            log_debug!("Found tag, tag id=[{}], tag_name=[{}], follower=[{}]", id, &name, &self.follower);

            let item = TagElement {
                tag_id: id,
                name,
                tag_type,
                string_tag_length,
                default_value,
            };
            let _ = &tags.push(item);
        }

        Ok(tags)
    }


    ///
    /// ✨ Create a new tag
    ///
    pub fn delete_tag(mut self, tag_id: i64) -> Json<JsonErrorSet> {

        log_info!("🚀 Start delete_tag api, follower={}", &self.follower);

        // Check if the token is valid
        if !self.session_token.is_valid() {
            return Json(
                JsonErrorSet::from(INVALID_TOKEN),
            );
        }
        self.follower.token_type = TokenType::Sid(self.session_token.0.clone());

        // Read the session information
        let Ok(entry_session) = fetch_entry_session(&self.follower.token_type.value()).map_err(err_fwd!("💣 Session Manager failed, follower={}", &self.follower)) else {
            return Json(JsonErrorSet::from(INTERNAL_TECHNICAL_ERROR));
        };

        let customer_code = entry_session.customer_code.as_str();

        // Open the transaction

        let internal_database_error_reply = Json(
            JsonErrorSet::from(INTERNAL_DATABASE_ERROR),
        );

        let mut r_cnx = SQLConnection::new();
        let r_trans = open_transaction(&mut r_cnx).map_err(err_fwd!("💣 Open transaction error, follower={}", &self.follower));
        let Ok(mut trans) = r_trans  else {
            return internal_database_error_reply;
        };

        // Check if the tag definition is used somewhere

        if self.check_tag_usage(&mut trans, tag_id, customer_code).is_err() {
            return Json(
                JsonErrorSet::from(STILL_IN_USE),
            );
        }

        // Delete the tag definition

        let sql_query = format!( r"DELETE FROM cs_{}.tag_definition
	                                WHERE id = :p_tag_id", customer_code );

        let mut params = HashMap::new();
        params.insert("p_tag_id".to_string(), CellValue::from_raw_int(tag_id));

        let sql_delete = SQLChange {
            sql_query,
            params,
            sequence_name: "".to_string()
        };

        let Ok(_tag_id) = sql_delete.delete(&mut trans)
                    .map_err(err_fwd!("💣 Tag delete failed, tag_id=[{}], follower=[{}]", tag_id, &self.follower)) else {
            return internal_database_error_reply;
        };

        if trans.commit().map_err(err_fwd!("💣 Commit failed, follower={}", &self.follower)).is_err() {
            return internal_database_error_reply;
        }

        log_info!("🏁 End delete_tag api, follower=[{}]", &self.follower);

        Json(
            JsonErrorSet::from(SUCCESS),
        )

    }


    fn check_tag_usage(&self, trans : &mut SQLTransaction, tag_id: i64, customer_code : &str) -> anyhow::Result<()> {

        let sql_query = format!( r"SELECT 1 FROM cs_{}.tag_value
	                                WHERE tag_id = :p_tag_id", customer_code );

        let mut params = HashMap::new();
        params.insert("p_tag_id".to_owned(), CellValue::from_raw_int(tag_id));

        let sql = SQLQueryBlock {
            sql_query,
            start: 0,
            length: Some(1),
            params,
        };

        let dataset = sql.execute( trans).map_err(tr_fwd!())?;

        if dataset.len() > 0 {
            return Err(anyhow::anyhow!("Tag still in use, follower=[{}]", &self.follower));
        }

        Ok(())
    }


}


///
/// Return a None if all inputs are correct
///
fn check_input_values(add_tag_request: &AddTagRequest)-> Option<AddTagReply> {

    // Check the input values ( ie tag_type, length limit, default_value type, etc )
    match add_tag_request.tag_type.to_lowercase().as_str() {
        "string" => {
            // The string_length between 0 and 10_000_000
            if let Some(length ) = add_tag_request.string_tag_length {
                if length > 10_000_000 || length < 0 {
                    return Some(AddTagReply {
                        tag_id: 0,
                        status:  JsonErrorSet::from(INCORRECT_STRING_LENGTH),
                    })
                }
                if let Some(default_string) = &add_tag_request.default_value {
                    if default_string.len() > length as usize {
                        return Some(AddTagReply {
                            tag_id: 0,
                            status:  JsonErrorSet::from(INCORRECT_DEFAULT_STRING_LENGTH),
                        })
                    }
                }
            }
        },
        "bool" => {
            if let Some(v) = &add_tag_request.default_value {
                if v != "true" && v != "false" {
                    return Some(AddTagReply {
                        tag_id: 0,
                        status:  JsonErrorSet::from(INCORRECT_DEFAULT_BOOLEAN_VALUE),
                    })
                }
            }
        },
        "integer" => {
            if let Some(v) = &add_tag_request.default_value {
                if v.parse::<i64>().is_err() {
                    return Some(AddTagReply {
                        tag_id: 0,
                        status:  JsonErrorSet::from(INCORRECT_DEFAULT_INTEGER_VALUE),
                    })
                }
            }
        },
        "double" => {
            if let Some(d) = &add_tag_request.default_value {
                if d.parse::<f64>().is_err() {
                    return Some(AddTagReply {
                        tag_id: 0,
                        status:  JsonErrorSet::from(INCORRECT_DEFAULT_DOUBLE_VALUE),
                    })
                }
            }
        },
        "date" => {
            if let Some(d_str) = &add_tag_request.default_value {
                // Check if the default is a valid date  ISO8601 1977-04-22
                if iso_to_date(d_str).is_err() {
                    return Some(AddTagReply {
                        tag_id: 0,
                        status:  JsonErrorSet::from(INCORRECT_DEFAULT_DATE_VALUE),
                    })
                }
            }
        },
        "datetime" => {
            if let Some(dt_str) = &add_tag_request.default_value {
                // Check if the default is a valid datetime ISO8601 "1977-04-22T06:00:00Z"
                if iso_to_datetime(dt_str).is_err() {
                    return Some(AddTagReply {
                        tag_id: 0,
                        status:  JsonErrorSet::from(INCORRECT_DEFAULT_DATETIME_VALUE),
                    })
                }
            }
        },
        _ => {
            return Some(AddTagReply {
                tag_id: 0,
                status:  JsonErrorSet::from(INCORRECT_TAG_TYPE),
            })
        },
    };

    None
}






fn add_tag_delegate(add_tag_request: Json<AddTagRequest>, session_token: SessionToken) -> Json<AddTagReply> {
    // Check if the token is valid
    if !session_token.is_valid() {
        return Json(AddTagReply {
            tag_id: 0,
            status: JsonErrorSet::from(INVALID_TOKEN),
        });
    }
    let sid = session_token.take_value();

    log_info!("🚀 Start add_tag api, sid={}", &sid);

    let internal_database_error_reply = Json(AddTagReply {
        tag_id: 0,
        status: JsonErrorSet::from(INTERNAL_DATABASE_ERROR),
    });

    let _internal_technical_error = Json(AddTagReply {
        tag_id: 0,
        status: JsonErrorSet::from(INTERNAL_TECHNICAL_ERROR),
    });

    // Read the session information
    let entry_session = match fetch_entry_session(&sid).map_err(err_fwd!("Session Manager failed")) {
        Ok(x) => x,
        Err(_) => {
            return Json(AddTagReply {
                tag_id: 0,
                status: JsonErrorSet::from(INTERNAL_TECHNICAL_ERROR),
            });
        }
    };

    let customer_code = entry_session.customer_code.as_str();

    if let Some(err) = check_input_values(&add_tag_request) {
        return Json(err);
    }

    // Open the transaction
    let mut r_cnx = SQLConnection::new();
    let r_trans = open_transaction(&mut r_cnx).map_err(err_fwd!("Open transaction error"));
    let Ok(mut trans) = r_trans else {
         return internal_database_error_reply;
    };

    let sql_query = format!( r"INSERT INTO cs_{}.tag_definition(name, string_tag_length, default_value, type)
	VALUES (:p_name, :p_string_tag_length , :p_default_value, :p_type)", customer_code );

    let sequence_name = format!( "cs_{}.tag_definition_id_seq", customer_code );

    let length = CellValue::Int32(add_tag_request.string_tag_length);
    let default_value = CellValue::from_opt_str(add_tag_request.default_value.as_deref());
    let mut params = HashMap::new();
    params.insert("p_name".to_string(), CellValue::from_raw_string(add_tag_request.name.clone()));
    params.insert("p_type".to_string(), CellValue::from_raw_string(add_tag_request.tag_type.clone()));
    params.insert("p_string_tag_length".to_string(), length);
    params.insert("p_default_value".to_string(), default_value);


    let sql_insert = SQLChange {
        sql_query,
        params,
        sequence_name,
    };

    let Ok(tag_id) = sql_insert.insert(&mut trans).map_err(err_fwd!("Insertion of a new item failed")) else {
        return internal_database_error_reply;
    };

    if trans.commit().map_err(err_fwd!("Commit failed")).is_err() {
        return internal_database_error_reply;
    }

    Json(AddTagReply {
        tag_id,
        status: JsonErrorSet::from(SUCCESS),
    })
}

///
/// Create a new tag
///
#[post("/tag", format = "application/json", data = "<add_tag_request>")]
pub (crate) fn add_tag(add_tag_request: Json<AddTagRequest>, session_token: SessionToken) -> Json<AddTagReply> {
    add_tag_delegate(add_tag_request, session_token)
}


#[cfg(test)]
mod test {

    use chrono::{Datelike, DateTime, NaiveDateTime, Timelike, Utc};
    use commons_pg::{iso_to_date, iso_to_datetime};

    #[test]
    fn is_valid_datetime_test() {

        assert!(iso_to_datetime("1977-04-22T06:12:04Z").is_ok());
        assert!(iso_to_datetime("1977-04-22T23:12:04Z").is_ok());
        assert!(iso_to_datetime("1977-04-22T23:12:04+01:00").is_ok());
        assert!(iso_to_datetime("0000-12-04T22:12:04Z").is_ok());

        assert!(iso_to_datetime("1977-04-22T26:12:04Z").is_err());
        assert!(iso_to_datetime("1977-0422T22:12:04Z").is_err());
        assert!(iso_to_datetime("1977-04T22:12:04Z").is_err());
        assert!(iso_to_datetime("1977-04-22T22:12:04+01").is_err());
        assert!(iso_to_datetime("1977-13-04T22:12:04Z").is_err());
    }

    #[test]
    fn is_valid_date_test() {

        assert!(iso_to_date("1977-04-22").is_ok());
        assert!(iso_to_date("2000-02-29").is_ok());

        assert!(iso_to_date("1977-13-26").is_err());
        assert!(iso_to_date("1977-02-32").is_err());
        assert!(iso_to_date("1977-02-29").is_err());
        assert!(iso_to_date("1977-02").is_err());
    }

    #[test]
    fn convert_iso8601_str_to_datetime() {
        let r_dt = DateTime::parse_from_rfc3339("1977-04-22T06:12:04Z");

        match r_dt {
            Ok(dt) => {
                assert_eq!(1977, dt.year());
                assert_eq!(4, dt.month());
                assert_eq!(22, dt.day());
                assert_eq!(6, dt.hour());
                assert_eq!(12, dt.minute());
                assert_eq!(4, dt.second());
                assert_eq!("+00:00", dt.timezone().to_string());
            }
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn convert_iso8601_str_to_date() {
        let r_dt = DateTime::parse_from_rfc3339("1977-04-22T00:00:00Z");

        match r_dt {
            Ok(dt) => {
                let d = dt.date();
                assert_eq!(1977,  d.year());
                assert_eq!(4, dt.month());
                assert_eq!(22, dt.day());
            }
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn convert_datetime_to_iso8601_string() {

        let dt = Utc::now();

        let s = dt.to_rfc3339();

        dbg!(s);


    }

}