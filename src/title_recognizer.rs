use anitomy::{Anitomy, ElementCategory};

pub struct Title {
    pub title: String,
    pub episode_number: i32,
}

pub struct TitleRecognizer {
    anitomy: Anitomy,
}

impl TitleRecognizer {
    pub fn new() -> Self {
        Self {
            anitomy: Anitomy::new(),
        }
    }

    pub fn recognize(&mut self, filename: &str) -> Option<Title> {
        match self.anitomy.parse(filename) {
            Ok(ref elements) => {
                let title = elements.get(ElementCategory::AnimeTitle)?.to_owned();
                let episode_number: i32 =
                    elements.get(ElementCategory::EpisodeNumber)?.parse().ok()?;

                Some(Title {
                    title,
                    episode_number,
                })
            }
            Err(ref elements) => None,
        }
    }
}
