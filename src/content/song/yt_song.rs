
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use anyhow::{
    Result,
    Context,
};
use serde::{Deserialize, Serialize};

use crate::{
    content::{
        manager::action::{
            ContentManagerAction,
            RustParallelAction,
        },
        song::traits::{
            SongTrait,
            Func,
            SongDisplay,
        },
    },
    service::{
        python::{
            action::PyAction,
            code::PyCodeBuilder,
            item::{
                Json,
                Ytdl,
            },
        },
        yt::ytdl::YtdlSong,
    },
    image::UnprocessedImage,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct YtSong {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub id: String,
}

type YtdlSongCallback = Box<dyn FnOnce(&YtdlSong) -> Result<ContentManagerAction> + Sync + Send>;

impl YtSong {
    fn get_ytdl_song(&self, callback: YtdlSongCallback) -> Result<ContentManagerAction> {
        // TODO: cache the result into the struct itself
        let action = PyAction::ExecCode {
            code: PyCodeBuilder::new()
            .threaded()
            .set_dbg_status(false)
            .dbg_func(
                "
                    with open('config/temp/ytdl_song.log', 'r') as f:
                        data = f.read()
                    return data
                ",
                None,
            )
            .func(
                format!("
                    ytdl_data = ytdl.extract_info(url='https://youtu.be/{}', download=False)
                    data = json.dumps(ytdl_data, indent=4)
                    return data
                ", self.id),
                Some(vec![
                    Ytdl::new("ytdl").into(),
                    Json::new("json").into(),
                ]),
            )
            .build()?,
            callback: Box::new(move |res: String| {
                let callback = callback;
                // debug!("{res}");
                let song = serde_json::from_str::<YtdlSong>(&res)?;
                // dbg!(&song);
                callback(&song)
            }),
        }.into();
        Ok(action)
    }
}

#[typetag::serde]
impl SongTrait for YtSong {
    fn is_online(&self) -> bool {
        true
    }
    fn get_uri(&self, callback: Func) -> Result<ContentManagerAction> {
        self.get_ytdl_song(Box::new(|song: &YtdlSong| {
            // yanked and translated code from ytdlp github readme
            // https://github.com/yt-dlp/yt-dlp#use-a-custom-format-selector
            let best_video_ext = song
            .formats
            .as_ref()
            .context("")?
            .iter()
            .rev()
            .filter(|f| {
                f.vcodec.is_some() &&
                f.vcodec.as_ref().unwrap() != "none" &&
                f.acodec.is_some() &&
                f.acodec.as_ref().unwrap() == "none"
            })
            .next()
            .context("")?
            .ext
            .as_ref()
            .context("")?;
            let best_audio_url = song
            .formats
            .as_ref()
            .context("")?
            .iter()
            .rev()
            .filter(|f| {
                f.acodec.is_some() &&
                f.acodec.as_ref().unwrap() != "none" &&
                f.vcodec.is_some() &&
                f.vcodec.as_ref().unwrap() == "none" &&
                f.ext.is_some() &&
                f.ext.as_ref().unwrap() == best_video_ext
            })
            .next()
            .context("")?
            .url
            .as_ref()
            .context("")?
            .clone();
            callback(best_audio_url)
        }))
    }
    fn play(&self) -> Result<ContentManagerAction> {
        self.get_uri(Box::new(|uri: String| {
            Ok(ContentManagerAction::PlaySongURI { uri })
        }))   
    }
    fn show_art(&self) -> Result<ContentManagerAction> {
        self.get_ytdl_song(Box::new(|song: &YtdlSong| {
            let best_thumbnail_url = song
            .thumbnails
            .as_ref()
            .context("")?
            .iter()
            .filter(|e| e.preference.is_some() && e.url.is_some())
            .reduce(|a, b| {
                if a.preference.unwrap() > b.preference.unwrap() {
                    a
                } else {
                    b
                }
            })
            .context("")?
            .url
            .as_ref()
            .unwrap()
            .to_owned();
            Ok(vec![
                ContentManagerAction::ClearImage,
                RustParallelAction::Callback {
                    callback: Box::new(|| {
                        let mut img = UnprocessedImage::Url(best_thumbnail_url);
                        img.prepare_image()?;
                        let action = ContentManagerAction::UpdateImage { img }.into();
                        Ok(action)
                    }),
                }.into(),
            ].into())
        }))
    }
    fn get_showable_info(&self) -> Box<dyn Iterator<Item = std::borrow::Cow<'static, str>>> {
        Box::new([
            format!("title: {}", self.title),
            format!("video id: {}", self.id),
        ].into_iter().map(Into::into))
    }

    fn as_display(&self) -> &dyn SongDisplay {
        self
    }
}

impl SongDisplay for YtSong {
    fn title(&self) -> &str {
        self.title.as_ref()
    }
    fn artist(&self) -> Option<&str> {
        Some(self.artist.as_ref())
    }
    fn album(&self) -> Option<&str> {
        self.album.as_ref().map(String::as_str)
    }
}

