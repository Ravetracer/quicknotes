//! Learning progress of flash cards: spaced repetition in the style of
//! Anki (SM-2), the review log and statistics.
//!
//! Stored in the app's data folder as `learning.json`, or sealed with the app
//! key as `learning.enc` while the app password is on. Cards are known by a
//! key derived from their question (see [`crate::flashcards`]); a card whose
//! question was edited is recognised by the similarity of its text.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

/// Answers, from forgotten to effortless.
pub const AGAIN: u8 = 1;
pub const HARD: u8 = 2;
/// The usual answer (the default case of [`schedule`]).
#[allow(dead_code)]
pub const GOOD: u8 = 3;
pub const EASY: u8 = 4;

/// Cards with an interval of at least this many days count as mature.
pub const MATURE_DAYS: f64 = 21.0;
const START_EASE: f64 = 2.5;
const MIN_EASE: f64 = 1.3;
/// Longest stored question excerpt (to recognise edited cards).
const EXCERPT: usize = 200;

/// Schedule of a card.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CardState {
    /// Day (see [`today`]) the card is due.
    pub due: i64,
    /// Days until the next review (0: being learned, due again today).
    pub interval: f64,
    pub ease: f64,
    /// Successful reviews.
    pub reps: u32,
    /// Times the card was forgotten after it had been learned.
    pub lapses: u32,
    /// Time of the last review (Unix seconds).
    pub last: i64,
    /// Document the card is in (file path or note id).
    #[serde(default)]
    pub doc: String,
    /// Beginning of the question, to find the card again after an edit.
    #[serde(default)]
    pub front: String,
}

/// One answer given.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Review {
    /// Unix seconds.
    pub at: i64,
    pub day: i64,
    pub key: String,
    pub rating: u8,
    /// Time taken to answer.
    pub ms: u32,
    /// Free practice: counted in the statistics, schedule unchanged.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub practice: bool,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct Store {
    pub cards: BTreeMap<String, CardState>,
    pub log: Vec<Review>,
}

/// The next schedule of a card answered with `rating`.
pub fn schedule(prev: Option<&CardState>, rating: u8, today: i64, now: i64) -> CardState {
    let (mut interval, mut ease, mut reps, mut lapses) = match prev {
        Some(p) => (p.interval, p.ease, p.reps, p.lapses),
        None => (0.0, START_EASE, 0, 0),
    };
    let learned = reps > 0 && interval >= 1.0;
    match rating {
        AGAIN => {
            if learned {
                lapses += 1;
            }
            ease = (ease - 0.2).max(MIN_EASE);
            interval = 0.0;
        }
        HARD => {
            interval = if interval < 1.0 { 1.0 } else { interval * 1.2 };
            ease = (ease - 0.15).max(MIN_EASE);
            reps += 1;
        }
        EASY => {
            interval = if interval < 1.0 { 4.0 } else { interval * ease * 1.3 };
            ease += 0.15;
            reps += 1;
        }
        _ => {
            interval = if interval < 1.0 { 1.0 } else { interval * ease };
            reps += 1;
        }
    }
    let interval = (interval.min(36_500.0) * 10.0).round() / 10.0;
    CardState {
        due: today + interval.round() as i64,
        interval,
        ease: (ease * 100.0).round() / 100.0,
        reps,
        lapses,
        last: now,
        doc: prev.map(|p| p.doc.clone()).unwrap_or_default(),
        front: prev.map(|p| p.front.clone()).unwrap_or_default(),
    }
}

/// An answer to record (see [`Store::review`]).
pub struct Answered<'a> {
    pub key: &'a str,
    /// Document of the card and its question (empty for protected tabs).
    pub doc: &'a str,
    pub front: &'a str,
    pub rating: u8,
    pub ms: u32,
    /// Free practice: logged, schedule unchanged.
    pub practice: bool,
}

/// A card of a document, for matching it with its stored state.
pub struct CardRef<'a> {
    pub key: &'a str,
    /// The question (plain text).
    pub front: &'a str,
}

/// Cards by learning state.
#[derive(Serialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub total: usize,
    pub new: usize,
    /// Due today (including cards being learned).
    pub due: usize,
    /// Seen, but not yet remembered for a day.
    pub learning: usize,
    /// Remembered for less than [`MATURE_DAYS`].
    pub young: usize,
    pub mature: usize,
}

impl Counts {
    pub fn add(&mut self, state: Option<&CardState>, today: i64) {
        self.total += 1;
        match state {
            None => self.new += 1,
            Some(s) => {
                if s.due <= today {
                    self.due += 1;
                }
                if s.reps == 0 || s.interval < 1.0 {
                    self.learning += 1;
                } else if s.interval < MATURE_DAYS {
                    self.young += 1;
                } else {
                    self.mature += 1;
                }
            }
        }
    }
}

/// Statistics of the answers given to a set of cards.
#[derive(Serialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct LogStats {
    pub reviews: usize,
    pub correct: usize,
    /// Time spent answering, in milliseconds.
    pub ms: u64,
    pub today_reviews: usize,
    pub today_ms: u64,
    /// Days in a row with answers, up to today (or yesterday).
    pub streak: u32,
    /// The last 30 days: day, correct and wrong answers.
    pub days: Vec<(i64, usize, usize)>,
}

impl Store {
    pub fn state(&self, key: &str) -> Option<&CardState> {
        self.cards.get(key)
    }

    /// The stored states of `cards` of document `doc`: by key, or else the
    /// state of a card of the document that no longer exists and whose
    /// question is similar (an edited card).
    pub fn resolve<'a>(&'a self, doc: &str, cards: &[CardRef<'_>]) -> Vec<Option<(&'a str, &'a CardState)>> {
        let current: HashSet<&str> = cards.iter().map(|c| c.key).collect();
        let mut orphans: Vec<(&'a str, &'a CardState)> = self
            .cards
            .iter()
            .filter(|(k, s)| !doc.is_empty() && s.doc == doc && !current.contains(k.as_str()) && !s.front.is_empty())
            .map(|(k, s)| (k.as_str(), s))
            .collect();
        cards
            .iter()
            .map(|c| {
                if let Some((k, s)) = self.cards.get_key_value(c.key) {
                    return Some((k.as_str(), s));
                }
                let front = excerpt(c.front);
                let (i, score) = orphans
                    .iter()
                    .enumerate()
                    .map(|(i, (_, s))| (i, similarity(&front, &s.front)))
                    .max_by(|a, b| a.1.total_cmp(&b.1))?;
                (score >= 0.7).then(|| orphans.remove(i))
            })
            .collect()
    }

    /// Moves the states (and log entries) of edited cards to their new keys
    /// and remembers document and question of every card. Returns whether
    /// anything changed. `remember_text` is off for password-protected tabs.
    pub fn adopt(&mut self, doc: &str, cards: &[CardRef<'_>], remember_text: bool) -> bool {
        let moves: Vec<(String, String)> = self
            .resolve(doc, cards)
            .into_iter()
            .zip(cards)
            .filter_map(|(found, card)| found.filter(|(k, _)| *k != card.key).map(|(k, _)| (k.to_owned(), card.key.to_owned())))
            .collect();
        let mut changed = !moves.is_empty();
        for (old, new) in &moves {
            if let Some(state) = self.cards.remove(old) {
                self.cards.insert(new.clone(), state);
            }
            for r in self.log.iter_mut().filter(|r| r.key == *old) {
                r.key = new.clone();
            }
        }
        for card in cards {
            if let Some(state) = self.cards.get_mut(card.key) {
                let front = if remember_text { excerpt(card.front) } else { String::new() };
                if state.doc != doc || state.front != front {
                    state.doc = doc.to_owned();
                    state.front = front;
                    changed = true;
                }
            }
        }
        changed
    }

    /// Records an answer.
    pub fn review(&mut self, a: &Answered<'_>, today: i64, now: i64) {
        let rating = a.rating.clamp(AGAIN, EASY);
        if !a.practice {
            let mut next = schedule(self.cards.get(a.key), rating, today, now);
            next.doc = a.doc.to_owned();
            next.front = excerpt(a.front);
            self.cards.insert(a.key.to_owned(), next);
        }
        self.log.push(Review { at: now, day: today, key: a.key.to_owned(), rating, ms: a.ms, practice: a.practice });
    }

    /// New cards (of `keys`) answered for the first time today.
    pub fn new_today(&self, keys: &HashSet<&str>, today: i64) -> usize {
        let mut first: HashMap<&str, i64> = HashMap::new();
        for r in self.log.iter().filter(|r| !r.practice && keys.contains(r.key.as_str())) {
            first.entry(r.key.as_str()).or_insert(r.day);
        }
        first.values().filter(|d| **d == today).count()
    }

    pub fn log_stats(&self, keys: &HashSet<&str>, today: i64) -> LogStats {
        let mut s = LogStats::default();
        let mut per_day: BTreeMap<i64, (usize, usize)> = BTreeMap::new();
        for r in self.log.iter().filter(|r| keys.contains(r.key.as_str())) {
            s.reviews += 1;
            s.ms += u64::from(r.ms);
            let correct = r.rating > AGAIN;
            if correct {
                s.correct += 1;
            }
            if r.day == today {
                s.today_reviews += 1;
                s.today_ms += u64::from(r.ms);
            }
            let e = per_day.entry(r.day).or_default();
            if correct {
                e.0 += 1;
            } else {
                e.1 += 1;
            }
        }
        let mut day = if per_day.contains_key(&today) { today } else { today - 1 };
        while per_day.contains_key(&day) {
            s.streak += 1;
            day -= 1;
        }
        s.days = (today - 29..=today).map(|d| {
            let (c, w) = per_day.get(&d).copied().unwrap_or_default();
            (d, c, w)
        }).collect();
        s
    }

    /// Adds the progress of another store (from a backup): the newer state
    /// of each card, and the answers not yet in the log.
    pub fn merge(&mut self, other: Store) {
        for (key, state) in other.cards {
            match self.cards.get(&key) {
                Some(mine) if mine.last >= state.last => {}
                _ => {
                    self.cards.insert(key, state);
                }
            }
        }
        let known: HashSet<(i64, String, u8)> = self.log.iter().map(|r| (r.at, r.key.clone(), r.rating)).collect();
        self.log.extend(other.log.into_iter().filter(|r| !known.contains(&(r.at, r.key.clone(), r.rating))));
        self.log.sort_by_key(|r| r.at);
    }

    // ----- storage ---------------------------------------------------------

    fn paths() -> (PathBuf, PathBuf) {
        let dir = crate::store::data_dir();
        (dir.join("learning.json"), dir.join("learning.enc"))
    }

    pub fn load() -> Store {
        let (plain, sealed) = Self::paths();
        let data = match (std::fs::read(&sealed), crate::store::app_key()) {
            (Ok(data), Some(key)) => match key.open(&data, AAD) {
                Ok(plain) => plain.to_vec(),
                Err(e) => {
                    eprintln!("quicknotes: cannot decrypt the learning progress: {e}");
                    return Store::default();
                }
            },
            _ => match std::fs::read(&plain) {
                Ok(data) => data,
                Err(_) => return Store::default(),
            },
        };
        Self::from_json(&data)
    }

    fn from_json(data: &[u8]) -> Store {
        serde_json::from_slice(data).unwrap_or_else(|e| {
            eprintln!("quicknotes: cannot read the learning progress: {e}");
            Store::default()
        })
    }

    /// Writes the progress, encrypted while the app password is on.
    pub fn save(&self) -> std::io::Result<()> {
        let (plain, sealed) = Self::paths();
        let json = serde_json::to_vec(self).map_err(std::io::Error::other)?;
        std::fs::create_dir_all(crate::store::data_dir())?;
        let write = |path: &PathBuf, data: &[u8]| {
            let tmp = path.with_extension("tmp");
            std::fs::write(&tmp, data)?;
            std::fs::rename(&tmp, path)
        };
        match crate::store::app_key() {
            Some(key) => {
                write(&sealed, &key.seal(&json, AAD))?;
                let _ = std::fs::remove_file(&plain);
            }
            None => {
                write(&plain, &json)?;
                let _ = std::fs::remove_file(&sealed);
            }
        }
        Ok(())
    }

    /// The progress as JSON (for backups).
    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn from_backup(data: &[u8]) -> Store {
        Self::from_json(data)
    }
}

const AAD: &[u8] = b"quicknotes-learning";

/// The local day number (days since 1970-01-01 in the local time zone).
pub fn today() -> i64 {
    let now = gtk::glib::DateTime::now_local();
    match now {
        Ok(t) => (t.to_unix() + t.utc_offset().as_seconds()).div_euclid(86_400),
        Err(_) => now_unix().div_euclid(86_400),
    }
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn excerpt(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    words.join(" ").chars().take(EXCERPT).collect()
}

/// Similarity of two texts from 0 to 1 (by edit distance).
fn similarity(a: &str, b: &str) -> f64 {
    let (a, b): (Vec<char>, Vec<char>) = (a.to_lowercase().chars().collect(), b.to_lowercase().chars().collect());
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 1.0;
    }
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = if ca == cb { prev } else { 1 + prev.min(row[j]).min(row[j + 1]) };
            prev = cur;
        }
    }
    1.0 - row[b.len()] as f64 / longest as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedules_like_anki() {
        let s = schedule(None, GOOD, 100, 0);
        assert_eq!((s.due, s.interval, s.reps), (101, 1.0, 1));
        let s = schedule(Some(&s), GOOD, 101, 0);
        assert_eq!((s.due, s.interval), (104, 2.5));
        let s = schedule(Some(&s), EASY, 104, 0);
        assert_eq!(s.interval, 8.1);
        assert_eq!(s.ease, 2.65);
        let forgotten = schedule(Some(&s), AGAIN, 111, 0);
        assert_eq!((forgotten.due, forgotten.interval, forgotten.lapses), (111, 0.0, 1));
        assert_eq!(forgotten.ease, 2.45);
        let s = schedule(None, AGAIN, 5, 0);
        assert_eq!((s.due, s.lapses, s.reps), (5, 0, 0));
        assert_eq!(schedule(None, EASY, 5, 0).due, 9);
        let mut hard = schedule(None, GOOD, 0, 0);
        for _ in 0..20 {
            hard = schedule(Some(&hard), HARD, 0, 0);
        }
        assert_eq!(hard.ease, MIN_EASE);
    }

    #[test]
    fn counts_states() {
        let mut c = Counts::default();
        c.add(None, 10);
        c.add(Some(&schedule(None, AGAIN, 10, 0)), 10);
        let young = schedule(None, GOOD, 10, 0);
        c.add(Some(&young), 10);
        let mut mature = young.clone();
        mature.interval = 30.0;
        mature.due = 40;
        c.add(Some(&mature), 10);
        assert_eq!(c, Counts { total: 4, new: 1, due: 1, learning: 1, young: 1, mature: 1 });
    }

    #[test]
    fn reviews_and_statistics() {
        let mut s = Store::default();
        s.review(&Answered { key: "a", doc: "doc", front: "Capital of France", rating: GOOD, ms: 4000, practice: false }, 10, 1000);
        s.review(&Answered { key: "b", doc: "doc", front: "Mitochondria", rating: AGAIN, ms: 2000, practice: false }, 10, 1010);
        s.review(&Answered { key: "b", doc: "doc", front: "Mitochondria", rating: GOOD, ms: 1000, practice: false }, 10, 1020);
        s.review(&Answered { key: "a", doc: "doc", front: "Capital of France", rating: GOOD, ms: 500, practice: true }, 9, 900);
        assert_eq!(s.state("a").unwrap().due, 11);
        assert_eq!(s.state("a").unwrap().front, "Capital of France");
        let keys: HashSet<&str> = ["a", "b"].into();
        assert_eq!(s.new_today(&keys, 10), 2);
        let st = s.log_stats(&keys, 10);
        assert_eq!((st.reviews, st.correct, st.ms, st.today_reviews, st.streak), (4, 3, 7500, 3, 2));
        assert_eq!(st.days.len(), 30);
        assert_eq!(st.days[29], (10, 2, 1));
    }

    #[test]
    fn edited_cards_keep_their_progress() {
        let mut s = Store::default();
        s.review(&Answered { key: "old", doc: "doc", front: "What is the capital of France?", rating: GOOD, ms: 1, practice: false }, 1, 1);
        s.review(&Answered { key: "other", doc: "doc2", front: "What is the capital of France", rating: GOOD, ms: 1, practice: false }, 1, 1);
        let cards = [CardRef { key: "new", front: "What is the capital of France ?" }];
        let found = s.resolve("doc", &cards);
        assert_eq!(found[0].map(|f| f.0), Some("old"));
        // A very different question is a new card.
        let other = [CardRef { key: "x", front: "Largest planet?" }];
        assert!(s.resolve("doc", &other)[0].is_none());
        assert!(s.adopt("doc", &cards, true));
        assert!(s.state("new").is_some() && s.state("old").is_none());
        assert!(s.log.iter().any(|r| r.key == "new"));
        assert!(!s.adopt("doc", &cards, true));
        // Locked documents do not keep the question text.
        assert!(s.adopt("doc", &cards, false));
        assert_eq!(s.state("new").unwrap().front, "");
    }

    #[test]
    fn merges_backups() {
        let mut a = Store::default();
        a.review(&Answered { key: "x", doc: "d", front: "X", rating: GOOD, ms: 1, practice: false }, 1, 100);
        let mut b = Store::default();
        b.review(&Answered { key: "x", doc: "d", front: "X", rating: AGAIN, ms: 1, practice: false }, 2, 200);
        b.review(&Answered { key: "y", doc: "d", front: "Y", rating: GOOD, ms: 1, practice: false }, 2, 150);
        let copy = serde_json::from_slice::<Store>(&a.to_json()).unwrap();
        a.merge(b);
        a.merge(copy);
        assert_eq!(a.state("x").unwrap().last, 200);
        assert!(a.state("y").is_some());
        assert_eq!(a.log.len(), 3);
        assert!(a.log.windows(2).all(|w| w[0].at <= w[1].at));
    }

    #[test]
    fn similarity_by_edit_distance() {
        assert_eq!(similarity("abc", "abc"), 1.0);
        assert_eq!(similarity("", ""), 1.0);
        assert!(similarity("kitten", "sitting") > 0.5);
        assert!(similarity("Paris", "Jupiter") < 0.3);
    }
}
