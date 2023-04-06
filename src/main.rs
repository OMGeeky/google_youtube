use log::{debug, error, info, trace, warn};
use std::error::Error;
use std::path::Path;

use google_youtube3::api::Playlist;
use simplelog::ColorChoice;
use tokio::fs::File;

use google_youtube::{scopes, PrivacyStatus, YoutubeClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Debug,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("TermLogger init failed");
    info!("Hello, world!");
    sample().await?;
    Ok(())
}

pub async fn sample() -> Result<(), Box<dyn Error>> {
    // get client
    let scopes = vec![
        scopes::YOUTUBE,
        scopes::YOUTUBE_UPLOAD,
        scopes::YOUTUBE_READONLY,
    ];
    // let client_secret_path = "auth/youtube_client_secret.json";
    let client_secret_path = "auth/test_rust_client_secret_2.json";
    let user = "nopixelvods";
    let client = YoutubeClient::new(Some(client_secret_path), scopes, Some(user)).await?;

    /*
    // get list of channels of the authenticated user
    let part = vec!["snippet".to_string()];
    let (_res, channels) = client
        .client
        .channels()
        .list(&part)
        .mine(true)
        .doit()
        .await?;
    for element in channels.items.unwrap() {
        info!(
            "channel name: {:?}",
            element.snippet.unwrap().title.unwrap()
        );
    }

    info!("Channels done!\n\n");
    */

    // get a playlist by name or create it if it does not exist('LunaOni Clips' for example)
    let playlist = client
        .find_playlist_or_create_by_name("LunaOni Clips")
        .await;
    info!("playlist: {:?}", playlist);

    info!("Playlist done!\n\n");
    info!("Uploading video... (30 times");
    for i in 0..30 {
        info!("+==={:2}==;uploading video...", i);
        // let path = Path::new("test/test.mp4");
        let path = Path::new("D:/1740252892.mp4_000.mp4");
        // let file = File::open(path).await?;
        let title = format!("test video {}", i);
        let description = "test video description";
        let tags = vec!["test".to_string(), "test2".to_string()];
        let privacy_status = PrivacyStatus::Private;

        info!("uploading video...");
        let insert = client
            .upload_video(&path, title.as_str(), description, tags, privacy_status)
            .await;
        info!("uploading video... (done)");

        info!("adding to playlist...");
        if let Ok(video) = &insert {
            if let Ok(playlist) = &playlist {
                info!("adding video to playlist: {:?}", playlist);
                let _ = client.add_video_to_playlist(&video, &playlist).await;
                info!("adding video to playlist: (done)");
            } else {
                info!("playlist not found");
            }
        } else {
            info!("video upload failed");
        }

        info!("\n\n{:?}\n\n/==={:2}========;", insert, i);
    }
    info!("Done!");
    Ok(())
}
