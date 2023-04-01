use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::time::Duration;

use google_youtube3::hyper::client::HttpConnector;
use google_youtube3::hyper_rustls::HttpsConnector;
use google_youtube3::oauth2;
use google_youtube3::oauth2::authenticator::Authenticator;
use google_youtube3::oauth2::authenticator_delegate::InstalledFlowDelegate;
use strfmt::strfmt;
use tokio::time::sleep;

use crate::auth;
use downloader_config::{load_config, Config};

struct CustomFlowDelegate {}

impl InstalledFlowDelegate for CustomFlowDelegate {
    fn redirect_uri(&self) -> Option<&str> {
        if load_config().use_local_auth_redirect {
            Some("http://localhost:8080/googleapi/auth")
        } else {
            Some("https://game-omgeeky.de:7443/googleapi/auth")
        }
    }

    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(present_user_url(url, need_code))
    }
}

async fn present_user_url(url: &str, need_code: bool) -> Result<String, String> {
    println!("Please open this URL in your browser:\n{}\n", url);
    if need_code {
        let conf = load_config();

        let mut code = String::new();
        if conf.use_file_auth_response {
            code = get_auth_code(&conf).await.unwrap_or("".to_string());
        } else {
            println!("Enter the code you get after authorization here: ");
            std::io::stdin().read_line(&mut code).unwrap();
        }
        Ok(code.trim().to_string())
    } else {
        println!("No code needed");
        Ok("".to_string())
    }
}

async fn get_auth_code(config: &Config) -> Result<String, Box<dyn Error>> {
    let code: String;

    let path = &config.path_auth_code;
    let path = Path::new(path);
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            println!("Error removing file: {:?}", e);
            panic!("Error removing file: {:?}", e);
        }
    }

    println!("Waiting for auth code in file: {}", path.display());
    loop {
        let res = std::fs::read_to_string(path); //try reading the file
        if let Ok(content) = res {
            let l = content.lines().next(); //code should be on first line of the file
            let re = match l {
                Some(s) => s,
                None => {
                    sleep(Duration::from_secs(config.auth_file_read_timeout)).await;
                    continue;
                }
            };

            code = re.to_string();
            // std::fs::remove_file(path)?;
            break;
        }
        // wait a few seconds
        sleep(Duration::from_secs(config.auth_file_read_timeout)).await;
    }

    Ok(code)
}

pub(crate) async fn get_authenticator<S: Into<String>>(
    path_to_application_secret: String,
    scopes: &Vec<String>,
    user: Option<S>,
) -> Result<Authenticator<HttpsConnector<HttpConnector>>, Box<dyn Error>> {
    let app_secret = oauth2::read_application_secret(path_to_application_secret).await?;
    let method = oauth2::InstalledFlowReturnMethod::Interactive;
    let config = load_config();
    let mut vars: HashMap<String, String> = HashMap::new();
    let user = match user {
        Some(u) => u.into(),
        None => "unknown".to_string(),
    };
    vars.insert("user".to_string(), user.clone());
    let persistent_path: String =
        strfmt(&config.path_authentications, &vars).expect("Error formatting path");
    println!("Persistent auth path for user:{} => {}", user, persistent_path);
    let auth = oauth2::InstalledFlowAuthenticator::builder(app_secret, method)
        .flow_delegate(Box::new(auth::CustomFlowDelegate {}))
        .persist_tokens_to_disk(persistent_path)
        .build()
        .await?; //TODO: somehow get rid of this unwrap that is happening in the library

    auth.token(&scopes).await?;
    Ok(auth)
}
