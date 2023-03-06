

mod customer_commands;
mod session_commands;
mod item_commands;
mod file_commands;
mod token_commands;
mod command_options;

use std::env;
use std::env::current_exe;

use std::path::{Path, PathBuf};
use std::process::exit;
use anyhow::{anyhow};
use dkconfig::conf_reader::{read_config_from_path};
use dkconfig::properties::{get_prop_value, set_prop_values};
use crate::customer_commands::customer_command;
use crate::file_commands::file_command;
use crate::item_commands::item_command;
use crate::session_commands::session_command;
use crate::token_commands::token_command;

// This is a dummy token
// TODO Token generation from a system user (should be limited in time)
// const SECURITY_TOKEN : &str = "j6nk2GaKdfLl3nTPbfWW0C_Tj-MFLrJVS2zdxiIKMZpxNOQGnMwFgiE4C9_cSScqshQvWrZDiPyAVYYwB8zCLRBzd3UUXpwLpK-LMnpqVIs";
// const SECURITY_TOKEN : &str = "6t3qlTv-mJyW3c52WqtH76RL6N1tgWuoqL1bs5CoWvNSeuUYpYjPvjytlPwCOhxv";

#[derive(Debug)]
struct Params {
    object: String,
    action: String,
    options : Vec<(String, String)>,
}

fn parse(args : &Vec<String>) -> anyhow::Result<Params> {
    // println!("number of args, [{}]", args.len());
    let object = args.get(1).ok_or(anyhow!("Don't find 1st param"))?.clone();
    let action = args.get(2).ok_or(anyhow!("Don't find 2nd param"))?.clone();
    let mut options : Vec<(String, String)> = vec![];
    let mut i = 3;

    loop {
        if i > args.len()-1 {
            break;
        }
        let option_name = args.get(i).ok_or(anyhow!("Don't find param, i=[{}]", i))?.clone();
        let option_value = args.get(i+1).ok_or(anyhow!("Don't find param, i+1=[{}]", i+1))?.clone();
        options.push((option_name, option_value));
        i += 2;
    }

    Ok(Params {
        object,
        action,
        options,
    })
}


fn read_configuration_file() -> anyhow::Result<()> {
    let config_path = get_target_file("config/application.properties")?;
    let config_path_str = config_path.to_str().ok_or(anyhow!("Cannot convert path to str"))?;
    println!("Define the properties from file : {}", config_path_str);
    let props = read_config_from_path( &config_path )?;

    set_prop_values(props);

    Ok(())
}

/// Get the location of a file into the working folder
fn get_target_file(termnination_path: &str) -> anyhow::Result<PathBuf> {

    let doka_cli_env = env::var("DOKA_CLI_ENV").unwrap_or("".to_string());

    if ! doka_cli_env.is_empty() {
        Ok(Path::new(&doka_cli_env).join("doka-cli").join(termnination_path).to_path_buf())
    } else {
        let path = current_exe()?; //
        let parent_path = path.parent().ok_or(anyhow!("Problem to identify parent's binary folder"))?;
        Ok(parent_path.join(termnination_path))
    }
}


///
/// dk [object] [action] [options]
///
/// We need a service discovery and/or a proxy to know where the services are located
/// They are potentially on different servers and ports
///
fn main() -> () {
    println!("doka-cli version 0.1.0");

    let mut exit_code = 0;
    let args: Vec<String> = env::args().collect();

    let params =  match parse(&args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("💣 Error while parsing the arguments, err=[{}]", e);
            exit_program(80);
        }
    };

    // println!("Params [{:?}]", &params);

    match read_configuration_file() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("💣 Error while reading the configuration file, err=[{}]", e);
            exit_program(110);
        }
    }

    let server_host = get_prop_value("server.host").unwrap();
    println!("Server host [{}]", &server_host);

    //

    match params.object.as_str() {
        "token" => {
            match token_command(&params) {
                Ok(_) => {
                    exit_code = 0;
                }
                Err(e) => {
                    exit_code = 70;
                    eprintln!("💣 Error {exit_code} : {}", e);
                }
            }
        }
        "customer" => {
            match customer_command(&params) {
                Ok(_) => {
                    exit_code = 0;
                }
                Err(e) => {
                    exit_code = 80;
                    eprintln!("💣 Error {exit_code} : {}", e);
                }
            }
        }
        "session" => {
            match session_command(&params) {
                Ok(_) => {
                    exit_code = 0;
                }
                Err(e) => {

                    exit_code = 90;
                    eprintln!("💣 Error {exit_code} : {}", e);
                }
            }
        }
        "item" => {
            match item_command(&params) {
                Ok(_) => {
                    exit_code = 0;
                }
                Err(e) => {

                    exit_code = 120;
                    eprintln!("💣 Error {exit_code} : {}", e);
                }
            }
        }
        "file" => {
            match file_command(&params) {
                Ok(_) => {
                    exit_code = 0;
                }
                Err(e) => {

                    exit_code = 140;
                    eprintln!("💣 Error {exit_code} : {}", e);
                }
            }
        }
        _ => {

        }
    }

    exit_program(exit_code);
}

fn exit_program(code: i32) -> ! {
    println!("Terminated [{}]", code);
    exit(code)
}
