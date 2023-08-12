use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use anyhow::anyhow;

use dkconfig::properties::get_prop_value;

use doka_cli::request_client::FileServerClient;

use crate::session_commands::read_session_id;

///
pub(crate) fn file_upload(item_info: &str, path :&str) -> anyhow::Result<()> {
    println!("👶 Uploading the file...");

    let server_host = get_prop_value("server.host")?;
    let file_server_port: u16 = get_prop_value("fs.port")?.parse()?;
    println!("File server port : {}", file_server_port);
    let client = FileServerClient::new(&server_host, file_server_port);

    let sid = read_session_id()?;

    let file = File::open(Path::new(&path))?;
    let mut buf_reader = BufReader::new(file);
    let mut binary : Vec<u8> = vec![];
    let _n = buf_reader.read_to_end(&mut binary)?;
    let wr_reply = client.upload(&item_info, &binary, &sid);

    match wr_reply {
        Ok(reply) => {
            println!("😎 File successfully uploaded, reference : {}, number of blocks : {} ", reply.file_ref, reply.block_count);
            Ok(())
        }
        Err(e) => {
            Err(anyhow!("{}", e.message))
        }
    }
}

///
/// Download the content behind the reference into the file at the path
///
pub(crate) fn file_download(path : &str, file_ref: &str) -> anyhow::Result<()> {
    println!("👶 Downloading the file...");

    let server_host = get_prop_value("server.host")?;
    let file_server_port: u16 = get_prop_value("fs.port")?.parse()?;
    println!("File server port: {}", file_server_port);
    let client = FileServerClient::new(&server_host, file_server_port);
    //let path = o_path.ok_or(anyhow!("💣 Missing path"))?;
    //let file_reference = o_file_ref.ok_or(anyhow!("💣 Missing file reference"))?;
    let sid = read_session_id()?;

    let wr_reply = client.download(&file_ref, &sid);

    match wr_reply {
        Ok(reply) => {
            // Store the result in a file
            let size = reply.data.len();
            if size > 0 {
                let mut file = std::fs::File::create(&path)?;
                let mut content = Cursor::new(reply.data);
                std::io::copy(&mut content, &mut file)?;
                println!("Document stored at: {}", &path);
                println!("Document type: {}", reply.media_type);
                println!("Document size: {}", size);
            } else {
                println!("Document not stored because it's empty");
            }
        }
        Err(e) => {
            println!("Status Code: {}", e.message);
        }
    }
    Ok(())
}
