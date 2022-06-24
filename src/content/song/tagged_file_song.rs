
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};


use std::{
    fmt::Debug,
    borrow::Cow,
};
use anyhow::{
    Result,
    Context,
};
use lofty::{
    TaggedFile,
    ItemKey,
    AudioFile,
};

use crate::{
    image::UnprocessedImage,
    content::{
        manager::{
            action::{
                ContentManagerAction,
                RustParallelAction,
            },
        },
        song::traits::{
            SongTrait,
            Func,
            SongDisplay,
        },
    },
};

#[derive(Debug)]
pub struct TaggedFileSong {
    title: String,
    album: String,
    artist: String,
    path: Cow<'static, str>,
}
impl TaggedFileSong {
    pub fn from_file<'a>(path: Cow<'a, str>) -> Result<Option<Self>> {
        let tf = lofty::read_from_path(path.as_ref(), true)?;
        let _ = log_song(&path);
        let st: TaggedSong = tf.into();
        let title = st.title();
        let album = st.album();
        let artist = st.artist();
        if title.is_some() && album.is_some() && artist.is_some() {
            let song = Self {
                path: path.into_owned().into(),
                title: title.unwrap().to_owned(),
                album: album.unwrap().to_owned(),
                artist: artist.unwrap().to_owned(),
            };
            Ok(Some(song))
        } else {
            Ok(None)
        }
    }
}

struct TaggedSong(TaggedFile);
impl From<TaggedFile> for TaggedSong {
    fn from(f: TaggedFile) -> Self {
        Self(f)
    }
}
impl TaggedSong {
    fn artist(&self) -> Option<&str> {
        self.get_val(&ItemKey::TrackArtist)
    }
    fn title(&self) -> Option<&str> {
        self.get_val(&ItemKey::TrackTitle)
    }
    fn album(&self) -> Option<&str> {
        self.get_val(&ItemKey::AlbumTitle)
    }
    fn get_val(&self, key: &ItemKey) -> Option<&str> {
        self.0
        .tags()
        .iter()
        .map(lofty::Tag::items)
        .map(|t| t.iter())
        .flatten()
        .filter(|t| t.key() == key)
        .find_map(|t| t.value().text())
    }
}

/// a function i used for checking what is returned by lofty
fn log_song(path: &str) -> Result<()> {
    debug!("logging song {path}");
    let probe = lofty::Probe::open(&path)?;
    let file_type = probe.file_type();
    // https://docs.rs/lofty/latest/lofty/struct.TaggedFile.html
    let tagged_file = probe.read(true)?;
    let properties = tagged_file.properties();
    // apparently a file can have multiple tags in it
    let tags = tagged_file
    .tags().iter()
    .map(lofty::Tag::items)
    .map(|e| e.iter()).flatten()
    .map(|e| (format!("{:#?}", e.key()), e.value().text().unwrap()))
    .collect::<Vec<_>>()
    ;
    let pics = tagged_file
    .tags().iter()
    .map(lofty::Tag::pictures)
    .collect::<Vec<_>>()
    ;
    let tag_type = tagged_file.primary_tag_type();
    dbg!(file_type, properties, tags, pics, tag_type);
    Ok(())
}


impl SongTrait for TaggedFileSong {
    fn is_online(&self) -> bool {
        false
    }
    fn get_showable_info(&self) -> Box<dyn Iterator<Item = Cow<'static, str>>> {
        Box::new([
            format!("title: {}", self.title),
            format!("artist: {}", self.artist),
            format!("album: {}", self.album),
        ].into_iter().map(Into::into))
    }
    fn get_uri(&self, callback: Func) -> Result<ContentManagerAction> {
        callback(format!("file://{}", self.path))
    }
    fn play(&self) -> Result<ContentManagerAction> {
        self.get_uri(Box::new(|uri: String| {
            Ok(ContentManagerAction::PlaySongURI { uri })
        }))
    }
    fn show_art(&self) -> Result<ContentManagerAction> {
        let path = self.path.clone();
        let callback = move || {
            let tf = lofty::read_from_path(path.as_ref(), true)?;
            let tags = tf.primary_tag().context("no primary tag on the image")?;
            let pics = tags.pictures();
            let img = if pics.len() >= 1 {
                Ok(
                    image::io::Reader::new(
                        std::io::Cursor::new(
                            pics[0].data().to_owned()
                        )
                    )
                    .with_guessed_format()?
                    .decode()?
                )
            } else {
                Err(anyhow::anyhow!("no image"))
            };
            let mut img = UnprocessedImage::Image {img: img?};
            img.prepare_image()?;
            let action = RustParallelAction::ContentManagerAction {
                action: ContentManagerAction::UpdateImage {
                    img,
                }.into(),
            }.into();
            Ok(action)
        };
        let action = vec![
            ContentManagerAction::ClearImage,
            RustParallelAction::Callback {
                callback: Box::new(callback),
            }.into(),
        ].into();
        Ok(action)
    }

    fn as_display(&self) -> &dyn super::traits::SongDisplay {
        self
    }
}

impl SongDisplay for TaggedFileSong {
    fn title(&self) -> &str {
        self.title.as_ref()
    }
    fn album(&self) -> Option<&str> {
        Some(self.album.as_ref())
    }
    fn artist(&self) -> Option<&str> {
        Some(self.artist.as_ref())
    }
}
