use std::collections::HashMap;
use std::ops::Range;
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum AnimeDbs {
    Mal,
    Kitsu,
    AniList,
}

pub struct AnimeRelations {
    rules: Vec<AnimeRelationRule>,
}

impl AnimeRelations {
    pub fn new() -> Self {
        Self {
            rules: Self::build_rules(),
        }
    }

    pub fn get_rule<'a>(
        &'a self,
        db: &'a AnimeDbs,
        src_id: i64,
    ) -> impl Iterator<Item = &'a AnimeRelationRule> + 'a {
        self.rules
            .iter()
            .filter(move |x| x.has_rule_for(db, src_id))
    }

    fn build_rules() -> Vec<AnimeRelationRule> {
        let rules_str = include_str!("../vendor/anime-relations/anime-relations.txt");
        let rules_pos = rules_str.find("::rules").unwrap();
        let rules_str = &rules_str[rules_pos..];
        let lines = rules_str.lines().filter(|x| x.starts_with("- "));

        let mut result = Vec::new();
        for line in lines {
            result.push(Self::build_rule(line));
            if line.ends_with('!') {
                result.push(Self::build_dest_rule(line));
            }
        }

        result
    }

    fn build_rule(s: &str) -> AnimeRelationRule {
        lazy_static! {
            static ref LINE_RE: Regex = Regex::new(
                r"- ([0-9?]+)\|([0-9?]+)\|([0-9?]+):([0-9\-?]+) -> ([0-9?~]+)\|([0-9?~]+)\|([0-9?~]+):([0-9\-?]+)"
            ).unwrap();
        }

        let captures = LINE_RE.captures(s).unwrap();

        let mal_src = Self::convert_src_id_str(&captures[1]);
        let kitsu_src = Self::convert_src_id_str(&captures[2]);
        let anilist_src = Self::convert_src_id_str(&captures[3]);
        let range_src = Self::convert_range(&captures[4]);
        let mal_dst = Self::convert_dst_id_str(&captures[5], &mal_src);
        let kitsu_dst = Self::convert_dst_id_str(&captures[6], &kitsu_src);
        let anilist_dst = Self::convert_dst_id_str(&captures[7], &anilist_src);
        let range_dst = Self::convert_range(&captures[8]);

        let mut mapping = HashMap::new();
        if let (Some(mal_src_val), Some(mal_dst_val)) = (mal_src, mal_dst) {
            mapping.insert(AnimeDbs::Mal, (mal_src_val, mal_dst_val));
        }
        if let (Some(kitsu_src_val), Some(kitsu_dst_val)) = (kitsu_src, kitsu_dst) {
            mapping.insert(AnimeDbs::Kitsu, (kitsu_src_val, kitsu_dst_val));
        }
        if let (Some(anilist_src_val), Some(anilist_dst_val)) = (anilist_src, anilist_dst) {
            mapping.insert(AnimeDbs::AniList, (anilist_src_val, anilist_dst_val));
        }

        AnimeRelationRule::new(mapping, range_src, range_dst)
    }

    fn convert_src_id_str(s: &str) -> Option<i64> {
        if s == "?" {
            None
        } else {
            Some(i64::from_str(s).unwrap())
        }
    }

    fn convert_dst_id_str(s: &str, src: &Option<i64>) -> Option<i64> {
        if s == "?" {
            None
        } else if s == "~" {
            *src
        } else {
            Some(i64::from_str(s).unwrap())
        }
    }

    #[allow(clippy::range_plus_one)]
    fn convert_range(s: &str) -> Range<i32> {
        let s = s.replace('?', "99999");
        if s.contains('-') {
            let numbers: Vec<&str> = s.split('-').collect();
            let start = i32::from_str(numbers[0]).unwrap();
            let end = i32::from_str(numbers[1]).unwrap();
            start..end + 1
        } else {
            let num = i32::from_str(&s).unwrap();
            num..num + 1
        }
    }

    fn build_dest_rule(s: &str) -> AnimeRelationRule {
        let mut rule = Self::build_rule(s);

        for val in rule.db_mappings.values_mut() {
            val.0 = val.1;
        }

        rule
    }
}

#[derive(Debug)]
pub struct AnimeRelationRule {
    db_mappings: HashMap<AnimeDbs, (i64, i64)>,
    range_src: Range<i32>,
    range_dst: Range<i32>,
}

impl AnimeRelationRule {
    fn new(
        mut db_mappings: HashMap<AnimeDbs, (i64, i64)>,
        range_src: Range<i32>,
        range_dst: Range<i32>,
    ) -> Self {
        db_mappings.shrink_to_fit();
        Self {
            db_mappings,
            range_src,
            range_dst,
        }
    }

    pub fn has_rule_for(&self, anime_db: &AnimeDbs, id: i64) -> bool {
        self.db_mappings.contains_key(anime_db) && self.db_mappings[anime_db].0 == id
    }

    pub fn convert_episode_number(&self, anime_db: &AnimeDbs, id: i64, number: i32) -> (i64, i32) {
        assert!(self.has_rule_for(anime_db, id));
        let rule = self.db_mappings[anime_db];

        if self.range_src.contains(&number) {
            let diff = number - self.range_src.start;
            (rule.1, self.range_dst.start + diff)
        } else {
            (id, number)
        }
    }
}
