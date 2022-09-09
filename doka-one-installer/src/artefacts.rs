use std::io::Cursor;
use std::path::Path;
use std::time::Duration;
use commons_error::*;
use crate::config::Config;

const TIMEOUT : Duration = Duration::from_secs(60 * 60); // 60 min

fn get_binary_data( url : &str) -> anyhow::Result<bytes::Bytes> {
    let request_builder = reqwest::blocking::Client::new().get(url).timeout(TIMEOUT);
    let response = request_builder.send()?; // .bytes().unwrap();
    Ok(response.bytes()?)
}


/// Download the artefact into the <install_dir>/artefacts  folder
fn download_file(config: &Config, artefact_name: &str) -> anyhow::Result<()> {

    println!("Downloading artefacts {artefact_name}");

    let zip_file = format!("{}.zip", artefact_name);

    // TODO Please create the correct folders before...
    let p = Path::new(&config.installation_path ).join("artefacts").join(&zip_file);

    // TODO test the https when the certificate is correct...
    let url = format!("http://doka.one/artefacts/0.1.0/{}", zip_file);

    let bin_artefact = get_binary_data(&url)?;

    let mut file = std::fs::File::create(&p)?;
    let mut content = Cursor::new(bin_artefact);
    std::io::copy(&mut content, &mut file)?;

    Ok(())
}


fn unzip(config: &Config, artefact_name: &str) -> anyhow::Result<()> {

    println!("Decompress artefacts {artefact_name}");

    let zip_file = format!("{}.zip", artefact_name);

    // Already exists
    let path  = Path::new(&config.installation_path ).join("artefacts").join(&zip_file);
    let archive = std::fs::read(path)?;

    // Already exists
    let target_dir  = Path::new(&config.installation_path ).join("bin").join(artefact_name);

    dbg!(&target_dir);

    // The third parameter allows you to strip away toplevel directories.
    // If `archive` contained a single directory, its contents would be extracted instead.
    let _ = zip_extract::extract(Cursor::new(archive), &target_dir, true)
        .map_err(eprint_fwd!("Cannot unzip"))?;

    println!("Done");

    Ok(())
}


pub(crate) fn download_artefacts(config: &Config) -> anyhow::Result<()> {
    println!("Downloading artefacts ...");

    // Download the doka services and the cli
    download_file(&config,  "key-manager")?;
    // download_file(&config,  "session-manager")?;
    // download_file(&config,  "admin-server")?;
    // download_file(&config,  "document-server")?;
    // download_file(&config,  "file-server")?;
    // download_file(&config,  "doka-cli")?;

    // Download the extra artefacts
    // | serman : https://www.dropbox.com/s/i7gptd0l289250t/serman.zip?dl=0
    download_file(&config,  "serman")?;
    // // | tika : https://www.dropbox.com/s/ftsf0elcal7pyqj/tika-server.zip?dl=0
    // download_file(&config,  "tika-server")?;
    // // | jdk
    // download_file(&config,  "jdk-17")?;


    // Unzip artefacts
    unzip(&config,  "key-manager")?;
    // unzip(&config,  "session-manager")?;
    // unzip(&config,  "admin-server")?;
    // unzip(&config,  "document-server")?;
    // unzip(&config,  "file-server")?;
    // unzip(&config,  "doka-cli")?;
    unzip(&config,  "serman")?;
    // unzip(&config,  "tika-server")?;
    // unzip(&config,  "jdk-17")?;

    Ok(())
}