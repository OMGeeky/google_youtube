use anyhow::{anyhow, Context};
use std::default::Default;
use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};

use exponential_backoff::youtube::generic_check_backoff_youtube;
use google_youtube3::{
    self as youtube,
    api::Playlist,
    api::PlaylistItem,
    api::PlaylistItemSnippet,
    api::PlaylistListResponse,
    api::PlaylistSnippet,
    api::PlaylistStatus,
    api::ResourceId,
    api::Video,
    api::VideoSnippet,
    api::VideoStatus,
    hyper::{client::HttpConnector, Body, Response},
    hyper_rustls::HttpsConnector,
};
#[cfg(feature = "tracing")]
use tracing::instrument;
use youtube::YouTube;
use youtube::{hyper, hyper_rustls::HttpsConnectorBuilder};

use crate::prelude::*;

mod auth;
pub mod prelude;
pub mod scopes;
// mod config;

pub struct YoutubeClient {
    pub client: YouTube<HttpsConnector<HttpConnector>>,
}
impl Debug for YoutubeClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("YoutubeClient").finish()
    }
}
#[derive(Debug, Copy, Clone)]
pub enum PrivacyStatus {
    Public,
    Unlisted,
    Private,
}
impl PrivacyStatus {
    fn to_string(&self) -> String {
        match self {
            PrivacyStatus::Public => "public".to_string(),
            PrivacyStatus::Unlisted => "unlisted".to_string(),
            PrivacyStatus::Private => "private".to_string(),
        }
    }
}
impl YoutubeClient {
    #[cfg_attr(feature = "tracing", instrument)]
    pub async fn new(
        path_to_application_secret: Option<impl Into<String> + Debug>,
        scopes: Vec<impl Into<String> + Debug>,
        user: Option<impl Into<String> + Debug>,
    ) -> anyhow::Result<Self> {
        let scopes = scopes
            .into_iter()
            .map(|s| s.into())
            .collect::<Vec<String>>();
        let hyper_client = hyper::Client::builder().build(
            HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .build(),
        );

        let path_to_application_secret = path_to_application_secret
            .map(|x| x.into())
            .unwrap_or_else(|| {
                warn!("the path to the application secret was not provided. Using default!");
                "auth/service_account2.json".to_string()
            });
        trace!(
            "getting authenticator from path: {}",
            path_to_application_secret
        );
        let auth = auth::get_authenticator(path_to_application_secret, &scopes, user)
            .await
            .map_err(|e| anyhow!("error while getting authenticator: {}", e))?;
        trace!("creating youtube client");
        let client: YouTube<HttpsConnector<HttpConnector>> = YouTube::new(hyper_client, auth);

        let res = Self { client };
        Ok(res)
    }

    #[cfg_attr(feature = "tracing", instrument)]
    pub async fn find_playlist_by_name(&self, name: &str) -> Result<Option<Playlist>> {
        let part = vec!["snippet".to_string()];

        struct PlaylistParams {
            part: Vec<String>,
            mine: bool,
        }
        async fn list_playlist(
            client: &YouTube<HttpsConnector<HttpConnector>>,
            params: &PlaylistParams,
        ) -> google_youtube3::Result<(Response<Body>, PlaylistListResponse)> {
            client
                .playlists()
                .list(&params.part)
                .mine(params.mine)
                .doit()
                .await
        }
        let para = PlaylistParams { part, mine: true };
        let (_res, playlists): (Response<Body>, PlaylistListResponse) =
            generic_check_backoff_youtube(&self.client, &para, list_playlist)
                .await
                .map_err(|e| anyhow!("backoff error: {}", e))?
                .context("list_playlist returned an error")?;

        if let Some(items) = playlists.items {
            for element in items {
                if let Some(snippet) = &element.snippet {
                    if let Some(title) = &snippet.title {
                        if title == name {
                            return Ok(Some(element));
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    #[cfg_attr(feature = "tracing", instrument)]
    pub async fn find_playlist_or_create_by_name(
        &self,
        name: &str,
        privacy: PrivacyStatus,
    ) -> Result<Playlist> {
        let playlist = self.find_playlist_by_name(name).await?;
        if let Some(playlist) = playlist {
            return Ok(playlist);
        }
        let playlist = self.create_playlist(name, privacy).await?;
        Ok(playlist)
    }

    #[cfg_attr(feature = "tracing", instrument)]
    pub async fn add_video_to_playlist(&self, video: &Video, playlist: &Playlist) -> Result<()> {
        let playlist_item = PlaylistItem {
            snippet: Some(PlaylistItemSnippet {
                playlist_id: Some(playlist.id.clone().unwrap()),
                resource_id: Some(ResourceId {
                    kind: Some("youtube#video".to_string()),
                    video_id: Some(video.id.clone().unwrap()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        async fn insert_playlist_item(
            client: &YouTube<HttpsConnector<HttpConnector>>,
            playlist_item: &PlaylistItem,
        ) -> google_youtube3::Result<(Response<Body>, PlaylistItem)> {
            client
                .playlist_items()
                .insert(playlist_item.clone())
                .doit()
                .await
        }

        // let res = self.client.playlist_items().insert(playlist_item).doit().await?;

        let (res, _) =
            generic_check_backoff_youtube(&self.client, &playlist_item, insert_playlist_item)
                .await
                .map_err(|e| anyhow!("backoff error: {}", e))?
                .context("insert playlist item returned an error")?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("got status: {}", res.status().as_u16()))
        }
    }

    #[cfg_attr(feature = "tracing", instrument)]
    pub async fn upload_video(
        &self,
        path: impl AsRef<Path> + Debug,
        title: impl Into<String> + Debug,
        description: impl Into<String> + Debug,
        tags: impl Into<Vec<String>> + Debug,
        privacy_status: PrivacyStatus,
    ) -> Result<Video> {
        info!("test 123");
        let video = Video {
            snippet: Some(VideoSnippet {
                title: Some(title.into()),
                description: Some(description.into()),
                category_id: Some("20".to_string()),
                tags: Some(tags.into()),
                ..Default::default()
            }),

            status: Some(VideoStatus {
                privacy_status: Some(privacy_status.to_string()),
                public_stats_viewable: Some(true),
                embeddable: Some(true),
                self_declared_made_for_kids: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        };
        // let file = file.into_std().await;

        struct UploadParameters {
            video: Video,
            path: PathBuf,
        }

        let params = UploadParameters {
            video: video.clone(),
            path: path.as_ref().into(),
        };

        async fn upload_fn(
            client: &YouTube<HttpsConnector<HttpConnector>>,
            para: &UploadParameters,
        ) -> Result<(Response<Body>, Video), google_youtube3::Error> {
            info!("Opening file: {:?}", para.path);
            let stream = std::fs::File::open(&para.path).map_err(|e| {
                error!("could not open file: {} error: {}", &para.path.display(), e);
                e
            })?;
            info!("Uploading file: {:?}", para.path);

            let insert_call = client.videos().insert(para.video.clone());
            info!("Insert call created");
            let upload_call = insert_call.upload_resumable(stream, "video/mp4".parse().unwrap());
            // .upload(stream, "video/mp4".parse().unwrap());
            info!("Upload request");
            let res = upload_call.await.map_err(|e| {
                error!("upload call did not work: {}", e);
                e
            });
            info!("Upload request done");
            res
        }

        info!("Starting upload...");
        let (response, video) = generic_check_backoff_youtube(&self.client, &params, upload_fn)
            .await
            .map_err(|e| anyhow!("backoff error: {}", e))?
            .context("upload_fn returned an error")?;

        // let (response, video) = exponential_backoff::youtube::check_backoff_youtube_upload(
        //     &self.client,
        //     video,
        //     &path,
        //     "video/mp4".parse().unwrap(),
        // )
        // .await??;

        if response.status().is_success() {
            info!("Upload successful!");
            Ok(video)
        } else {
            info!("Upload failed!\n=====================================\n");
            info!("Status: {}", response.status());
            info!("Body: {:?}", response);
            info!("Video: {:?}", video);
            Err(anyhow!("got status: {}", response.status().as_u16()))
        }

        // return Ok(video);
        // let insert: google_youtube3::Result<(Response<Body>, Video)> = self
        //     .client
        //     .videos()
        //     .insert(video)
        //     .upload(file, "video/mp4".parse().unwrap())
        //     .await;
        //
        // match insert {
        //     Ok(insert) => Ok(insert),
        //     Err(e) => {
        //         info!("Error: {:?}", e);
        //         Err(Box::new(e))
        //     }
        // }
    }
    #[cfg_attr(feature = "tracing", instrument)]
    async fn create_playlist(&self, name: &str, privacy: PrivacyStatus) -> Result<Playlist> {
        let playlist = Playlist {
            snippet: Some(PlaylistSnippet {
                title: Some(name.to_string()),
                ..Default::default()
            }),
            status: Some(PlaylistStatus {
                privacy_status: Some(privacy.to_string()),
            }),
            ..Default::default()
        };

        async fn create_playlist(
            client: &YouTube<HttpsConnector<HttpConnector>>,
            params: &Playlist,
        ) -> google_youtube3::Result<(Response<Body>, Playlist)> {
            client.playlists().insert(params.clone()).doit().await
        }

        let (res, playlist) =
            generic_check_backoff_youtube(&self.client, &playlist, create_playlist)
                .await
                .map_err(|e| anyhow!("backoff error: {}", e))?
                .map_err(|e| anyhow!("create playlist returned an error: {}", e))?;

        if res.status().is_success() {
            Ok(playlist)
        } else {
            Err(anyhow!("got status: {}", res.status().as_u16()))
        }
    }
}

#[cfg_attr(feature = "tracing", instrument)]
pub async fn sample() -> Result<(), Box<dyn Error>> {
    info!("Hello from the youtube lib!");
    Ok(())
}
