use std::error::Error;
use std::path::Path;
use google_youtube3::api::Playlist;

use tokio::fs::File;

use google_youtube::{PrivacyStatus, scopes, YoutubeClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");
    sample().await?;
    Ok(())
}

pub async fn sample() -> Result<(), Box<dyn Error>> {
    // get client
    let scopes = vec![
        // google_youtube::scopes::YOUTUBE,
        google_youtube::scopes::YOUTUBE_UPLOAD,
        google_youtube::scopes::YOUTUBE_READONLY,
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
        println!(
            "channel name: {:?}",
            element.snippet.unwrap().title.unwrap()
        );
    }

    println!("Channels done!\n\n");
    */

    // get a playlist by name or create it if it does not exist('LunaOni Clips' for example)
    let playlist = client.find_playlist_or_create_by_name("LunaOni Clips").await;
    println!("playlist: {:?}", playlist);

    println!("Playlist done!\n\n");
    println!("Uploading video... (30 times");
    for i in 0..30 {
        println!("+==={:2}==;uploading video...", i);
        let path = Path::new("test/test.mp4");
        // let file = File::open(path).await?;
        let description = "test video description";
        let title = "test video2";
        let tags = vec!["test".to_string(), "test2".to_string()];
        let privacy_status = PrivacyStatus::Private;

        println!("uploading video...");
        let insert = client
            .upload_video(&path, description, title, tags, privacy_status)
            .await;
        println!("uploading video... (done)");

        println!("adding to playlist...");
        if let Ok(video) = &insert{
            if let Ok(playlist) = &playlist {
                println!("adding video to playlist: {:?}", playlist);
                let _ = client.add_video_to_playlist(&video, &playlist).await;
            }
        }
        println!("adding to playlist... (done)");

        println!("\n\n{:?}\n\n/==={:2}========;", insert, i);
    }
    println!("Done!");
    Ok(())
}
