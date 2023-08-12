mod test_lib;

const TEST_TO_RUN : &[&str] = &["t01_upload_file", "t02_upload_download_file"];

#[cfg(test)]
mod api_fileserver_tests {
    use std::thread;
    use core::time::Duration;
    use dkdto::{ErrorMessage, LoginRequest};
    use doka_cli::request_client::{AdminServerClient, FileServerClient};
    use crate::test_lib::{Lookup, read_test_env};
    use crate::TEST_TO_RUN;

    const NB_PARTS : u32 = 9;

    #[test]
    fn t01_upload_file() -> Result<(), ErrorMessage> {
        let lookup = Lookup::new("t01_upload_file", TEST_TO_RUN); // auto dropping
        let test_env = read_test_env();

        eprintln!("test_env {:?}", &test_env);

        // Login
        let admin_server = AdminServerClient::new("localhost", 30060);
        let login_request = LoginRequest {
            login: test_env.login,
            password: test_env.password,
        };
        let login_reply = admin_server.login(&login_request)?;

        eprintln!("login_reply {:?}", &login_reply);

        // Upload the document
        let file_server = FileServerClient::new("localhost", 30080);

        let file_content = std::fs::read(r"F:\Dropbox\Upload\111-Bright_Snow.jpg").unwrap();
        let upload_reply = file_server.upload( "bright snow", &file_content,  &login_reply.session_id)?;
        eprintln!("Upload reply [{:?}]", &upload_reply);
        assert_eq!(NB_PARTS, upload_reply.block_count);

        wait_until_file_processing_complete(&file_server, &upload_reply.file_ref, &login_reply.session_id,upload_reply.block_count);

        // Get the information of the file
        let info_reply = file_server.info(&upload_reply.file_ref, &login_reply.session_id)?;

        eprintln!("Info reply [{:?}]", &info_reply);
        assert_eq!("image/jpeg", info_reply.media_type.unwrap());

        lookup.close();
        Ok(())
    }

    fn wait_until_file_processing_complete(file_server: &FileServerClient, file_ref: &str, session_id: &str, block_count: u32) {
        let mut count = 0;
        let duration = Duration::from_millis(500);
        loop {
            eprintln!("Stats count [{}]", count);
            match file_server.stats(&file_ref, &session_id) {
                Ok(stats_reply) => {
                    eprintln!("Stats reply [{:?}]", &stats_reply);
                    if stats_reply.encrypted_count == block_count as i64 {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Stats reply error [{:?}]", &e);
                }
            }
            thread::sleep(duration);
            if count > 10 {
                break;
            }
            count += 1;
        }
    }

    #[test]
    fn t02_upload_download_file() -> Result<(), ErrorMessage> {
        let lookup = Lookup::new("t02_upload_download_file", TEST_TO_RUN); // auto dropping
        let test_env = read_test_env();

        eprintln!("test_env {:?}", &test_env);

        // Login
        let admin_server = AdminServerClient::new("localhost", 30060);
        let login_request = LoginRequest {
            login: test_env.login,
            password: test_env.password,
        };
        let login_reply = admin_server.login(&login_request)?;

        eprintln!("login_reply {:?}", &login_reply);

        // Upload the document
        let file_server = FileServerClient::new("localhost", 30080);

        let file_content = std::fs::read(r"F:\Dropbox\Upload\111-Bright_Snow.jpg").unwrap();
        let upload_reply = file_server.upload( "bright snow", &file_content,  &login_reply.session_id)?;
        eprintln!("Upload reply [{:?}]", &upload_reply);
        assert_eq!(NB_PARTS, upload_reply.block_count);

        wait_until_file_processing_complete(&file_server, &upload_reply.file_ref, &login_reply.session_id,upload_reply.block_count);

        // Download the file
        let download_reply = file_server.download(&upload_reply.file_ref, &login_reply.session_id)?;

        eprintln!("Download reply size [{}]", &download_reply.data.len());
        assert_eq!(8890555, download_reply.data.len());

        lookup.close();
        Ok(())
    }
}