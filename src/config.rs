// use std::env;
//
// #[derive(Clone)]
// pub struct Config {
//     pub path_auth_code: String,
//     pub path_authentications: String,
//     pub use_file_auth_response: bool,
//     pub use_local_auth_redirect: bool,
//     pub auth_file_read_timeout: u64,
// }
//
// pub fn load_config() -> Config {
//     let path_auth_code =
//         env::var("PATH_AUTH_CODE").unwrap_or("/tmp/twba/auth/code.txt".to_string());
//     let path_authentications =
//         env::var("PATH_AUTHENTICATIONS").unwrap_or("/tmp/twba/auth/{user}.json".to_string());
//     let use_file_auth_response =
//         env::var("USE_FILE_AUTH_RESPONSE").unwrap_or("1".to_string()) == "1";
//     let use_local_auth_redirect =
//         env::var("USE_LOCAL_AUTH_REDIRECT").unwrap_or("0".to_string()) == "1";
//     let auth_file_read_timeout = env::var("AUTH_FILE_READ_TIMEOUT")
//         .unwrap_or("5".to_string())
//         .parse()
//         .unwrap();
//     Config {
//         path_auth_code,
//         use_file_auth_response,
//         path_authentications,
//         use_local_auth_redirect,
//         auth_file_read_timeout,
//     }
// }
