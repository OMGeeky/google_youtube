use anyhow::anyhow;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
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

use crate::prelude::*;
use downloader_config::{load_config, Config};
struct CustomFlowDelegate {
    user: String,
}

impl CustomFlowDelegate {
    fn new(user: String) -> Self {
        Self { user }
    }
}

impl InstalledFlowDelegate for CustomFlowDelegate {
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    fn redirect_uri(&self) -> Option<&str> {
        if load_config().use_local_auth_redirect {
            trace!("local redirect uri");
            Some("http://localhost:8080/googleapi/auth")
        } else {
            trace!("remote redirect uri");
            Some("https://game-omgeeky.de:7443/googleapi/auth")
        }
    }

    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(self.present_user_url(url, need_code))
    }
}
impl CustomFlowDelegate {
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    async fn present_user_url(&self, url: &str, need_code: bool) -> Result<String, String> {
        println!(
            "Please open this URL in your browser to authenticate for {}:\n{}\n",
            self.user, url
        );
        info!("Please open this URL in your browser:\n{}\n", url);
        if need_code {
            let conf = load_config();

            let mut code = String::new();
            if conf.use_file_auth_response {
                code = get_auth_code(&conf).await.unwrap_or("".to_string());
            } else {
                println!("Enter the code you get after authorization here: ");
                info!("Enter the code you get after authorization here: ");
                std::io::stdin().read_line(&mut code).unwrap();
            }
            Ok(code.trim().to_string())
        } else {
            println!("No code needed");
            info!("No code needed");
            Ok("".to_string())
        }
    }
}
#[cfg_attr(feature = "tracing", tracing::instrument)]
async fn get_auth_code(config: &Config) -> Result<String, Box<dyn Error>> {
    let code: String;

    let path = &config.path_auth_code;
    let path = Path::new(path);
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            println!("Error removing file: {:?}", e);
            error!("Error removing file: {:?}", e);
            panic!("Error removing file: {:?}", e);
        }
    }

    println!("Waiting for auth code in file: {}", path.display());
    info!("Waiting for auth code in file: {}", path.display());
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

#[cfg_attr(feature = "tracing", tracing::instrument)]
pub(crate) async fn get_authenticator(
    path_to_application_secret: String,
    scopes: &Vec<String>,
    user: Option<impl Into<String> + Debug>,
) -> Result<Authenticator<HttpsConnector<HttpConnector>>> {
    let user = user.map(|x| x.into());
    trace!(
        "getting authenticator for user: {:?} with scopes: {:?} and secret_path: {}",
        user,
        scopes,
        path_to_application_secret
    );
    trace!("reading application secret");
    let app_secret = oauth2::read_application_secret(path_to_application_secret).await?;
    trace!("read application secret");

    let config = load_config();
    let mut vars: HashMap<String, String> = HashMap::new();
    let user = match user {
        Some(u) => u.into(),
        None => "unknown".to_string(),
    };
    vars.insert("user".to_string(), user.clone());
    let persistent_path = strfmt(&config.path_authentications, &vars)
        .map_err(|e| anyhow!("Error formatting path: {}", e))?;
    let persistent_path: &Path = Path::new(&persistent_path);
    debug!(
        "Persistent auth path for user:{} => {}",
        user,
        persistent_path.display()
    );
    let persistent_path_parent = persistent_path
        .parent()
        .ok_or(anyhow!("could not get parent of path"))?;
    if !persistent_path.exists() || persistent_path.is_dir() {
        warn!(
            "persistent path does not exist or is a dir: {}",
            persistent_path.display()
        );
        let create_dir = std::fs::create_dir_all(persistent_path_parent);
        warn!("result of create dir: {:?}", create_dir);
    }
    trace!("building authenticator");
    let method = oauth2::InstalledFlowReturnMethod::Interactive;
    let auth = oauth2::InstalledFlowAuthenticator::builder(app_secret, method)
        .flow_delegate(Box::new(CustomFlowDelegate::new(user)))
        .persist_tokens_to_disk(persistent_path.to_path_buf())
        .force_account_selection(true)
        .build()
        .await
        //TODO: somehow get rid of this unwrap that is happening in the library
        .map_err(|e| anyhow!("got an error from the authenticator: {}", e))?;
    trace!("got authenticator, requesting scopes");
    let access_token = auth
        .token(&scopes)
        .await
        .map_err(|e| anyhow!("could not get access to the requested scopes: {}", e))?;
    trace!("got scope access: {:?}", access_token);
    Ok(auth)
}
