use std::cell::RefCell;

use anitomy::{Anitomy, ElementCategory};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Title {
    pub title: String,
    pub episode_number: i32,
}

impl Title {
    pub fn new(title: String, episode_number: i32) -> Self {
        Self {
            title,
            episode_number,
        }
    }
}

#[derive(Default)]
pub struct TitleRecognizer {}

thread_local! {
    static ANITOMY: RefCell<Anitomy> = RefCell::new(Anitomy::new());
}

impl TitleRecognizer {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn recognize(&mut self, filename: &str) -> Option<Title> {
        ANITOMY.with(|anitomy| match anitomy.borrow_mut().parse(filename) {
            Ok(ref elements) => {
                println!("{:?}", elements);
                let mut title = elements.get(ElementCategory::AnimeTitle)?.to_owned();

                let episode_number: i32 = elements
                    .get(ElementCategory::EpisodeNumber)
                    .unwrap_or("1")
                    .parse()
                    .ok()?;
                if episode_number < 1 {
                    return None;
                }

                let season = elements.get(ElementCategory::AnimeSeason);
                if let Some(season) = season {
                    title += " Season ";
                    title += season;
                }

                Some(Title::new(title, episode_number))
            }
            Err(_elements) => None,
        })
    }
}
