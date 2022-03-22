#![feature(proc_macro_hygiene, decl_macro)]

use std::path::Path;
use std::process::exit;
use std::collections::HashMap;
use log::{info, error};
use rocket::*;
use rocket_contrib::json::Json;
use rocket::http::RawStr;
use rocket_contrib::templates::Template;
use rocket::config::Environment;

use dkconfig::conf_reader::{read_config};
use dkconfig::properties::{get_prop_pg_connect_string, get_prop_value, set_prop_values};

use commons_error::*;
use commons_pg::{SQLConnection, SQLChange, CellValue, SQLQueryBlock, SQLDataSet, SQLTransaction, init_db_pool};
use commons_services::database_lib::open_transaction;
use commons_services::read_cek_and_store;
use commons_services::token_lib::SecurityToken;
use dkcrypto::dk_crypto::DkEncrypt;
use dkdto::{AddKeyReply, AddKeyRequest, CustomerKeyReply, EntryReply, JsonErrorSet};
use dkdto::error_codes::{CUSTOMER_KEY_ALREADY_EXISTS, INTERNAL_DATABASE_ERROR, INTERNAL_TECHNICAL_ERROR, INVALID_REQUEST, INVALID_TOKEN, SUCCESS};


// Read the list of users from the DB
fn read_entries( customer_code : Option<&str> ) -> CustomerKeyReply {

    let internal_database_error_reply = CustomerKeyReply{ keys: HashMap::new(), status: JsonErrorSet::from(INTERNAL_DATABASE_ERROR) };

    let mut r_cnx = SQLConnection::new();
    let mut trans = match open_transaction(&mut r_cnx).map_err(err_fwd!("Open transaction error")) {
        Ok(x) => { x },
        Err(_) => { return internal_database_error_reply; },
    };

    let entries = match search_key_by_customer_code(&mut trans, customer_code) {
        Ok(ds) => { ds },
        Err(_) => { return internal_database_error_reply; },
    };

    if trans.commit().map_err(err_fwd!("Commit failed")).is_err() {
        return internal_database_error_reply;
    }
    CustomerKeyReply{ keys: entries, status: JsonErrorSet::from(SUCCESS) }

}


#[get("/key/<customer_code>")]
fn read_key(customer_code: &RawStr, security_token: SecurityToken) -> Json<CustomerKeyReply> {

    // ** Check if the token is valid
    if ! security_token.is_valid() {
        log_error!("Invalid security token {:?}", &security_token);
        return Json(CustomerKeyReply { keys: HashMap::new(), status: JsonErrorSet::from(INVALID_TOKEN) } )
    }
    let token = security_token.take_value();

    log_info!("🚀 Start read_key api, token_id=[{:?}]", token);

    let customer_code = match customer_code.percent_decode().map_err(err_fwd!("Invalid input parameter [{}]", customer_code) ) {
        Ok(s) => s.to_string(),
        Err(_) => {
            return Json(CustomerKeyReply {  keys: HashMap::new(), status: JsonErrorSet::from(INVALID_REQUEST) } )
        }
    };

    // customer key to return.
    let customer_key_reply = read_entries(Some(&customer_code));

    dbg!(&customer_key_reply);

    log_info!("🏁 End read_key api, token=[{:?}]", token);

    Json(customer_key_reply)

}


///
///
///
#[get("/key")]
fn key_list(security_token: SecurityToken) -> Json<CustomerKeyReply> {
    // ** Check if the token is valid
    if ! security_token.is_valid() {
        log_error!("Invalid security token {:?}", &security_token);
        return Json(CustomerKeyReply { keys: HashMap::new(), status: JsonErrorSet::from(INVALID_TOKEN) } )
    }
    let token = security_token.take_value();

    log_info!("🚀 Start key api, token_id=[{:?}]", &token);

    // List of customer keys to return.
    let customer_key_reply = read_entries(None);

    log_info!("🏁 End key api, token=[{:?}]", &token);

    Json(customer_key_reply)

}



fn search_key_by_customer_code(mut trans : &mut SQLTransaction, customer_code : Option<&str>) -> anyhow::Result<HashMap<String, EntryReply>> {
    let p_customer_code = CellValue::from_opt_str(customer_code);

    let mut params = HashMap::new();
    params.insert("p_customer_code".to_owned(), p_customer_code);

    let query = SQLQueryBlock {
        sql_query : r"SELECT id, customer_code, ciphered_key FROM keymanager.customer_keys
                    WHERE customer_code = :p_customer_code OR :p_customer_code IS NULL ".to_string(),
        start : 0,
        length : None,
        params,
    };

    let mut sql_result : SQLDataSet =  query.execute(&mut trans).map_err(err_fwd!("Query failed, [{}]", &query.sql_query))?;

    let mut entries= HashMap::new();
    while sql_result.next() {
        let id : i64 = sql_result.get_int("id").unwrap_or(0i64);
        let customer_code: String = sql_result.get_string("customer_code").unwrap_or("".to_owned());
        let ciphered_key: String = sql_result.get_string("ciphered_key").unwrap_or("".to_owned());

        let key_info = EntryReply {
            key_id : id,
            customer_code,
            ciphered_key,
            active: true,
        };

        let _ = &entries.insert(key_info.customer_code.clone(), key_info);

    }

    Ok(entries)
}


#[post("/key", format = "application/json", data = "<customer>")]
fn add_key(customer: Json<AddKeyRequest>, security_token: SecurityToken) -> Json<AddKeyReply> {

    dbg!(&customer);

    // Check if the trace_id is valid
    if !security_token.is_valid() {
        return Json(AddKeyReply {
            success: false,
            status: JsonErrorSet::from(INVALID_TOKEN),
        });
    }
    let token = security_token.take_value();

    log_info!("🚀 Start add_key api, token_id={}", &token);

    let internal_database_error_reply = Json(AddKeyReply{ success : false, status: JsonErrorSet::from(INTERNAL_DATABASE_ERROR) });

    let mut r_cnx = SQLConnection::new();
    let mut trans = match open_transaction(&mut r_cnx).map_err(err_fwd!("Open transaction error")) {
        Ok(x) => { x },
        Err(_) => { return internal_database_error_reply; },
    };

    // Verify if the customer code exists in the system
    let entries  = match search_key_by_customer_code(&mut trans, Some(&customer.customer_code)) {
        Ok(x) => {x}
        Err(_) => {
            return internal_database_error_reply;
        }
    };

    if entries.contains_key(&customer.customer_code) {
        return Json(AddKeyReply{ success : false, status: JsonErrorSet::from(CUSTOMER_KEY_ALREADY_EXISTS) });
    }

    let cek = get_prop_value("cek");
    dbg!(&cek);

    let new_customer_key = DkEncrypt::generate_random_key();
    dbg!(&new_customer_key);

    let internal_error_reply = Json(AddKeyReply{ success : false, status: JsonErrorSet::from(INTERNAL_TECHNICAL_ERROR) });

    let enc_password = match DkEncrypt::encrypt_str(&new_customer_key, &cek) {
        Ok(v) => { v },
        Err(_) => { return internal_error_reply; },
    };

    let success = true;
    let sql_insert = r#"INSERT INTO keymanager.customer_keys(
                            customer_code, ciphered_key)
                            VALUES (:p_customer_code, :p_ciphered_key)"#;


    let mut params : HashMap<String, CellValue> = HashMap::new();
    params.insert("p_customer_code".to_owned(), CellValue::from_raw_string(customer.customer_code.to_owned()));
    params.insert("p_ciphered_key".to_owned(), CellValue::from_raw_string(enc_password));

    let query = SQLChange {
        sql_query :  sql_insert.to_string(),
        params,
        sequence_name : "keymanager.customer_keys_id_seq".to_string(),
    };

    // TODO Handles the failure error !!!
    let _ = query.insert(&mut trans);

    if trans.commit().map_err(err_fwd!("Commit failed")).is_err() {
        return internal_database_error_reply;
    }

    if success {
        log_info!("😎 Customer key added with success");
    }

    let ret = AddKeyReply {
        success,
        status: JsonErrorSet::from(SUCCESS),
    };
    log_info!("🏁 End dd_key, token_id = {}, success={}", &token, success);
    Json(ret)
}


///
///
///
fn main() {

    const PROGRAM_NAME: &str = "Key Manager";

    println!("😎 Init {}", PROGRAM_NAME);

    const PROJECT_CODE: &str = "key-manager";
    const VAR_NAME: &str = "DOKA_ENV";

    // Read the application config's file
    println!("😎 Config file using PROJECT_CODE={} VAR_NAME={}", PROJECT_CODE, VAR_NAME);

    let props = read_config(PROJECT_CODE, VAR_NAME);

    dbg!(&props);
    set_prop_values(props);

    let port = get_prop_value("server.port").parse::<u16>().unwrap();
    dbg!(port);

    let log_config: String = get_prop_value("log4rs.config");

    let log_config_path = Path::new(&log_config);

    // Read the global properties
    println!("😎 Read log properties from {:?}", &log_config_path);

    match log4rs::init_file(&log_config_path, Default::default()) {
        Err(e) => {
            eprintln!("{:?} {:?}", &log_config_path, e);
            exit(-59);
        }
        Ok(_) => {}
    }

    // Read the CEK
    log_info!("😎 Read Common Edible Key");
    read_cek_and_store();

    // Init DB pool
    let (connect_string, db_pool_size) = match get_prop_pg_connect_string()
        .map_err(err_fwd!("Cannot read the database connection information")) {
        Ok(x) => x,
        Err(e) => {
            log_error!("{:?}", e);
            exit(-64);
        }
    };

    init_db_pool(&connect_string, db_pool_size);

    log_info!("🚀 Start {}", PROGRAM_NAME);

    let new_prop = get_prop_value("cek");
    dbg!(&new_prop);

    let mut my_config = Config::new(Environment::Production);
    my_config.set_port(port);

    let base_url = format!("/{}", PROJECT_CODE);

    let _ = rocket::custom(my_config)
        .mount(&base_url, routes![key_list, add_key, read_key])
        .attach(Template::fairing())
        .launch();

    log_info!("🏁 End {}", PROGRAM_NAME);
}

#[cfg(test)]
mod test {
    use dkdto::AddKeyRequest;
    use dkdto::AddKeyReply;

    #[test]
    fn http_post_add_key() -> anyhow::Result<()> {
        let customer_code = "denis.zzzzzzz".to_string();
        let token= "j6nk2GaKdfLl3nTPbfWW0C_Tj-MFLrJVS2zdxiIKMZpxNOQGnMwFgiE4C9_cSScqshQvWrZDiPyAVYYwB8zCLRBzd3UUXpwLpK-LMnpqVIs".to_string();

        let new_post = AddKeyRequest {
            customer_code,
        };

        let reply: AddKeyReply = reqwest::blocking::Client::new()
            .post("http://localhost:30040/key-manager/key")
            .header("token", token.clone())
            .json(&new_post)
            .send()?.json()?;


        // let reply : AddKeyReply = reqwest::Client::new()
        //     .post("http://localhost:30040/key-manager/key")
        //     .header("token", token.clone())
        //     .json(&new_post)
        //     .send()
        //     .await?
        //     .json()
        //     .await?;

        dbg!(&reply);

        // println!("{:#?}", new_post);

        Ok(())

        // let rocket = rocket::ignite();
        // let client = Client::new(rocket).expect("valid rocket");
        //
        // let msg = format!("{{    \"customer_code\":   \"denis.\"{}       }}", customer_code);
        //
        // let _response = client.post("/key-manager/key")
        //     .header(Header::new("token_id", token.clone()))
        //     .header(ContentType::JSON)
        //     .remote("127.0.0.1:30040".parse().unwrap())
        //     .body(&msg)
        //     .dispatch();
    }
}