#![feature(proc_macro_hygiene, decl_macro)]


mod session;

use std::path::Path;
use std::process::exit;
use log::*;
use rocket::*;
use rocket_contrib::json::Json;
use dkconfig::conf_reader::{read_config};


use rocket::http::RawStr;
use commons_error::*;
use rocket_contrib::templates::Template;

use rocket::config::Environment;
use commons_pg::{init_db_pool};
use commons_services::property_name::{COMMON_EDIBLE_KEY_PROPERTY, LOG_CONFIG_FILE_PROPERTY, SERVER_PORT_PROPERTY};
use commons_services::read_cek_and_store;
use commons_services::token_lib::SecurityToken;
use commons_services::x_request_id::{XRequestID};
use dkconfig::properties::{get_prop_pg_connect_string, get_prop_value, set_prop_values};

use dkdto::{OpenSessionReply, OpenSessionRequest, SessionReply};

use crate::session::SessionDelegate;

///
/// 🔑 Find a session from its sid
///
#[get("/session/<session_id>")]
fn read_session(session_id: &RawStr, security_token: SecurityToken, x_request_id: XRequestID) -> Json<SessionReply> {
    let mut delegate = SessionDelegate::new(security_token, x_request_id);
    delegate.read_session(session_id)
}



///
/// 🔑 Open a new session for the group and user
/// It's usually called by the Login end point using the session_id as a security_token
///
#[post("/session", format = "application/json", data = "<session_request>")]
fn open_session(session_request: Json<OpenSessionRequest>, security_token: SecurityToken, x_request_id: XRequestID) -> Json<OpenSessionReply> {
    let mut delegate = SessionDelegate::new(security_token, x_request_id);
    delegate.open_session(session_request)
}

///
fn main() {

    const PROGRAM_NAME: &str = "Session Manager";

    println!("😎 Init {}", PROGRAM_NAME);

    const PROJECT_CODE: &str = "session-manager";
    const VAR_NAME: &str = "DOKA_ENV";

    // Read the application config's file
    println!("😎 Config file using PROJECT_CODE={} VAR_NAME={}", PROJECT_CODE, VAR_NAME);

    let props = read_config(PROJECT_CODE, VAR_NAME);

    set_prop_values(props);

    let Ok(port) = get_prop_value(SERVER_PORT_PROPERTY).unwrap_or("".to_string()).parse::<u16>() else {
        eprintln!("💣 Cannot read the server port");
        exit(-56);
    };
    let Ok(log_config) = get_prop_value(LOG_CONFIG_FILE_PROPERTY) else {
        eprintln!("💣 Cannot read the log4rs config");
        exit(-57);
    };

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

    let Ok(cek) = get_prop_value(COMMON_EDIBLE_KEY_PROPERTY) else {
        panic!("💣 Cannot read the cek properties");
    };
    log_info!("😎 The CEK was correctly read : [{}]", format!("{}...", &cek[0..5]));

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

    let mut my_config = Config::new(Environment::Production);
    my_config.set_port(port);

    let base_url = format!("/{}", PROJECT_CODE);

    let _ = rocket::custom(my_config)
        .mount(&base_url, routes![open_session, read_session])
        .attach(Template::fairing())
        .launch();

    log_info!("🏁 End {}", PROGRAM_NAME);
}
