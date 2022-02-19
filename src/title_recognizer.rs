use std::cell::RefCell;
use std::str::FromStr;

use anitomy::{Anitomy, ElementCategory, Elements};
use log::debug;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Title {
    pub title: String,
    pub season_number: i32,
    pub episode_number: i32,
}

impl Title {
    pub fn new(title: String, season_number: i32, episode_number: i32) -> Self {
        Self {
            title,
            season_number,
            episode_number,
        }
    }
}

trait Recognizer: Send + Sync {
    fn recognize(&mut self, title: Option<&str>, filename: Option<&str>) -> Option<Title>;
}

#[derive(Default)]
pub struct TitleRecognizer {
    recognizers: Vec<Box<dyn Recognizer>>,
}

impl TitleRecognizer {
    pub fn new() -> Self {
        let recognizers: Vec<Box<dyn Recognizer>> = vec![
            Box::new(AniCliRecognizer::new()),
            Box::new(AnitomyRecognizer::new()),
        ];

        Self { recognizers }
    }

    pub fn recognize(&mut self, title: Option<&str>, filename: Option<&str>) -> Option<Title> {
        self.recognizers
            .iter_mut()
            .map(|x| x.recognize(title, filename))
            .filter_map(|x| x)
            .next()
    }
}

#[derive(Default)]
struct AnitomyRecognizer;

thread_local! {
    static ANITOMY: RefCell<Anitomy> = RefCell::new(Anitomy::new());
}

impl AnitomyRecognizer {
    pub fn new() -> Self {
        Default::default()
    }

    fn elements_to_title(elements: &Elements) -> Option<Title> {
        debug!("Found path elements: {:?}", elements);
        let title = elements.get(ElementCategory::AnimeTitle)?.to_owned();

        let episode_number: i32 = elements
            .get(ElementCategory::EpisodeNumber)
            .unwrap_or("1")
            .parse()
            .ok()?;
        if episode_number < 1 {
            return None;
        }

        let season_number: i32 = elements
            .get(ElementCategory::AnimeSeason)
            .unwrap_or("1")
            .parse()
            .ok()?;

        Some(Title::new(title, season_number, episode_number))
    }
}

impl Recognizer for AnitomyRecognizer {
    fn recognize(&mut self, _title: Option<&str>, filename: Option<&str>) -> Option<Title> {
        if let Some(filename) = filename {
            ANITOMY.with(|anitomy| match anitomy.borrow_mut().parse(filename) {
                Ok(ref elements) => Self::elements_to_title(elements),
                Err(_elements) => None,
            })
        } else {
            None
        }
    }
}

#[derive(Default)]
struct AniCliRecognizer;

impl AniCliRecognizer {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Recognizer for AniCliRecognizer {
    fn recognize(&mut self, title: Option<&str>, _filename: Option<&str>) -> Option<Title> {
        if let Some(title) = title {
            if title.starts_with("ani-cli: ") {
                let (title, episode_number_str) = title.split_once(" ep ")?;
                let episode_number = i32::from_str(episode_number_str).ok()?;

                return Some(Title::new(title.to_owned(), 1, episode_number));
            }
        }

        None
    }
}
