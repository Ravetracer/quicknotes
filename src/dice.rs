//! Dice notation for the GM tools.
//!
//! - `2d6+3`, `d20`, `d%` (d100 as tens and units), `4dF` (Fate), `3d{Name}`
//!   (a custom die), sums and differences of terms and numbers.
//! - Modifiers after a dice term: `kh2`/`k2` keep highest, `kl1` keep lowest,
//!   `dh1`/`dl1` drop highest/lowest, `!` exploding dice, `r2` reroll values
//!   up to 2 once, `>=8` / `>7` / `<=3` / `<4` count successes.
//! - `check 12/14/13 5` is a three-attribute check (DSA): each d20 above its
//!   attribute uses up skill points; an optional third number eases (+) or
//!   hardens (−) the check: `check 12/14/13 5 -2`.

use serde::Serialize;

/// A custom die: its name and faces (symbols or numbers).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CustomDie {
    pub name: String,
    pub faces: Vec<String>,
}

/// The result of a roll.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Roll {
    pub notation: String,
    /// The number, when the result is one.
    pub total: Option<i64>,
    /// The short result ("17", "3 successes", "Success, QS 2", "Hit ×2").
    pub summary: String,
    /// The single dice ("[12, ~~4~~] + 5").
    pub detail: String,
    /// A natural maximum (critical) or minimum on a single d20 and the like.
    pub critical: Option<bool>,
}

/// Random numbers 1..=sides.
pub trait Rng {
    fn die(&mut self, sides: u32) -> u32;
}

/// The system's random source.
pub struct SystemRng;

impl Rng for SystemRng {
    fn die(&mut self, sides: u32) -> u32 {
        let sides = sides.max(1);
        // Rejection sampling avoids the modulo bias.
        let limit = u32::MAX - u32::MAX % sides;
        loop {
            let mut b = [0u8; 4];
            if getrandom::getrandom(&mut b).is_err() {
                return 1;
            }
            let v = u32::from_le_bytes(b);
            if v < limit {
                return v % sides + 1;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Sides(u32),
    Percent,
    Fate,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Cmp {
    Ge,
    Gt,
    Le,
    Lt,
}

#[derive(Clone, Debug, PartialEq)]
struct Dice {
    count: u32,
    kind: Option<Kind>,
    custom: Option<String>,
    keep_high: Option<u32>,
    keep_low: Option<u32>,
    drop_high: Option<u32>,
    drop_low: Option<u32>,
    explode: bool,
    reroll: Option<i64>,
    count_if: Option<(Cmp, i64)>,
}

#[derive(Clone, Debug, PartialEq)]
enum Term {
    Dice(Dice),
    Number(i64),
}

const MAX_DICE: u32 = 500;
const MAX_EXPLODE: usize = 100;

fn num(s: &[u8], i: &mut usize) -> Option<i64> {
    let start = *i;
    while *i < s.len() && s[*i].is_ascii_digit() {
        *i += 1;
    }
    std::str::from_utf8(&s[start..*i]).ok()?.parse().ok()
}

fn parse(notation: &str) -> Result<Vec<(i64, Term)>, String> {
    let compact: String = notation.chars().filter(|c| !c.is_whitespace()).collect();
    let s = compact.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    let mut sign = 1;
    let bad = || t!("Unknown dice “{}”", notation);
    if s.is_empty() {
        return Err(bad());
    }
    loop {
        if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
            sign = if s[i] == b'-' { -sign } else { sign };
            i += 1;
            continue;
        }
        let count = num(s, &mut i);
        if i < s.len() && (s[i] == b'd' || s[i] == b'D' || s[i] == b'w' || s[i] == b'W') {
            i += 1;
            let mut dice = Dice {
                count: count.unwrap_or(1).clamp(0, MAX_DICE as i64) as u32,
                kind: None,
                custom: None,
                keep_high: None,
                keep_low: None,
                drop_high: None,
                drop_low: None,
                explode: false,
                reroll: None,
                count_if: None,
            };
            match s.get(i) {
                Some(b'%') => {
                    i += 1;
                    dice.kind = Some(Kind::Percent);
                }
                Some(b'F' | b'f') if !s.get(i + 1).is_some_and(u8::is_ascii_digit) => {
                    i += 1;
                    dice.kind = Some(Kind::Fate);
                }
                Some(b'{') => {
                    let end = compact[i..].find('}').ok_or_else(bad)? + i;
                    dice.custom = Some(notation_name(notation, &compact[i + 1..end]));
                    i = end + 1;
                }
                _ => {
                    let sides = num(s, &mut i).ok_or_else(bad)?;
                    if !(1..=1000).contains(&sides) {
                        return Err(bad());
                    }
                    dice.kind = Some(Kind::Sides(sides as u32));
                }
            }
            // Modifiers.
            loop {
                let rest = &compact[i..];
                let take = |n: usize, i: &mut usize| -> u32 {
                    *i += n;
                    num(s, i).unwrap_or(1).max(0) as u32
                };
                if rest.starts_with("kh") {
                    dice.keep_high = Some(take(2, &mut i));
                } else if rest.starts_with("kl") {
                    dice.keep_low = Some(take(2, &mut i));
                } else if rest.starts_with("dh") {
                    dice.drop_high = Some(take(2, &mut i));
                } else if rest.starts_with("dl") {
                    dice.drop_low = Some(take(2, &mut i));
                } else if rest.starts_with('k') {
                    dice.keep_high = Some(take(1, &mut i));
                } else if rest.starts_with('!') {
                    dice.explode = true;
                    i += 1;
                } else if rest.starts_with('r') && rest[1..].starts_with(|c: char| c.is_ascii_digit()) {
                    i += 1;
                    dice.reroll = num(s, &mut i);
                } else if let Some((cmp, len)) = [(">=", Cmp::Ge), ("<=", Cmp::Le), (">", Cmp::Gt), ("<", Cmp::Lt)]
                    .iter()
                    .find(|(p, _)| rest.starts_with(p))
                    .map(|(p, c)| (*c, p.len()))
                {
                    i += len;
                    dice.count_if = Some((cmp, num(s, &mut i).ok_or_else(bad)?));
                } else {
                    break;
                }
            }
            out.push((sign, Term::Dice(dice)));
        } else if let Some(n) = count {
            out.push((sign, Term::Number(n)));
        } else {
            return Err(bad());
        }
        sign = 1;
        if i >= s.len() {
            break;
        }
        if s[i] != b'+' && s[i] != b'-' {
            return Err(bad());
        }
    }
    Ok(out)
}

/// The custom die name as written (spaces kept).
fn notation_name(notation: &str, compact_name: &str) -> String {
    let Some(start) = notation.find('{') else { return compact_name.to_owned() };
    let end = notation[start..].find('}').map_or(notation.len(), |e| start + e);
    notation[start + 1..end].trim().to_owned()
}

fn cmp_ok(cmp: Cmp, v: i64, n: i64) -> bool {
    match cmp {
        Cmp::Ge => v >= n,
        Cmp::Gt => v > n,
        Cmp::Le => v <= n,
        Cmp::Lt => v < n,
    }
}

/// Whether a text is dice notation (for clickable rolls).
pub fn is_notation(s: &str) -> bool {
    let s = s.trim();
    if s.starts_with("check ") {
        return true;
    }
    s.chars().any(|c| matches!(c, 'd' | 'D' | 'w' | 'W')) && s.len() < 60 && parse(s).is_ok_and(|t| t.iter().any(|(_, t)| matches!(t, Term::Dice(_))))
}

/// Rolls `notation` with the custom dice `custom`.
pub fn roll(notation: &str, custom: &[CustomDie], rng: &mut dyn Rng) -> Result<Roll, String> {
    let notation = notation.trim();
    if let Some(rest) = notation.strip_prefix("check ").or_else(|| notation.strip_prefix("probe ")) {
        return check(notation, rest, rng);
    }
    let terms = parse(notation)?;
    let mut total: i64 = 0;
    let mut numeric = true;
    let mut parts: Vec<String> = Vec::new();
    let mut successes: Option<i64> = None;
    let mut symbols: Vec<(String, usize)> = Vec::new();
    let mut critical = None;
    let single_d20 = terms.iter().filter(|(_, t)| matches!(t, Term::Dice(_))).count() == 1;
    for (sign, term) in &terms {
        let prefix = if parts.is_empty() {
            if *sign < 0 { "−" } else { "" }
        } else if *sign < 0 {
            " − "
        } else {
            " + "
        };
        match term {
            Term::Number(n) => {
                total += sign * n;
                parts.push(format!("{prefix}{n}"));
            }
            Term::Dice(d) => {
                if let Some(name) = &d.custom {
                    let die = custom
                        .iter()
                        .find(|c| c.name.eq_ignore_ascii_case(name))
                        .ok_or_else(|| t!("Unknown die “{}”", name))?;
                    if die.faces.is_empty() {
                        return Err(t!("Unknown die “{}”", name));
                    }
                    let faces: Vec<&String> = (0..d.count).map(|_| &die.faces[rng.die(die.faces.len() as u32) as usize - 1]).collect();
                    let all_numbers = faces.iter().all(|f| f.trim().parse::<i64>().is_ok());
                    if all_numbers {
                        let sum: i64 = faces.iter().map(|f| f.trim().parse::<i64>().unwrap_or(0)).sum();
                        total += sign * sum;
                    } else {
                        numeric = false;
                        for f in &faces {
                            for sym in f.split_whitespace() {
                                match symbols.iter_mut().find(|(s, _)| s == sym) {
                                    Some((_, n)) => *n += 1,
                                    None => symbols.push((sym.to_owned(), 1)),
                                }
                            }
                        }
                    }
                    let shown: Vec<String> = faces.iter().map(|f| if f.trim().is_empty() { "–".to_owned() } else { (*f).clone() }).collect();
                    parts.push(format!("{prefix}[{}]", shown.join(", ")));
                    continue;
                }
                let kind = d.kind.unwrap_or(Kind::Sides(6));
                let mut values: Vec<(i64, String)> = Vec::new();
                for _ in 0..d.count {
                    let once = |rng: &mut dyn Rng| -> (i64, String) {
                        match kind {
                            Kind::Fate => {
                                let v = rng.die(3) as i64 - 2;
                                (v, ["−", "0", "+"][(v + 1) as usize].to_owned())
                            }
                            Kind::Percent => {
                                let tens = rng.die(10) as i64 - 1;
                                let units = rng.die(10) as i64 - 1;
                                let v = if tens == 0 && units == 0 { 100 } else { tens * 10 + units };
                                (v, format!("{}{}", tens * 10, if units > 0 { format!("+{units}") } else { String::new() }))
                            }
                            Kind::Sides(s) => {
                                let v = rng.die(s) as i64;
                                (v, v.to_string())
                            }
                        }
                    };
                    let mut v = once(rng);
                    if let Some(limit) = d.reroll {
                        if v.0 <= limit {
                            let old = v.1.clone();
                            v = once(rng);
                            v.1 = format!("~~{old}~~ {}", v.1);
                        }
                    }
                    values.push(v.clone());
                    if d.explode {
                        if let Kind::Sides(s) = kind {
                            let mut last = v.0;
                            let mut n = 0;
                            while last == s as i64 && s > 1 && n < MAX_EXPLODE {
                                let extra = rng.die(s) as i64;
                                values.push((extra, format!("{extra}!")));
                                last = extra;
                                n += 1;
                            }
                        }
                    }
                }
                // Keep and drop.
                let mut order: Vec<usize> = (0..values.len()).collect();
                order.sort_by_key(|&i| values[i].0);
                let mut dropped = vec![false; values.len()];
                let n = values.len();
                let drop_lowest = |k: usize, dropped: &mut Vec<bool>| order.iter().take(k.min(n)).for_each(|&i| dropped[i] = true);
                if let Some(k) = d.keep_high {
                    drop_lowest(n.saturating_sub(k as usize), &mut dropped);
                }
                if let Some(k) = d.drop_low {
                    drop_lowest(k as usize, &mut dropped);
                }
                let drop_highest = |k: usize, dropped: &mut Vec<bool>| order.iter().rev().take(k.min(n)).for_each(|&i| dropped[i] = true);
                if let Some(k) = d.keep_low {
                    drop_highest(n.saturating_sub(k as usize), &mut dropped);
                }
                if let Some(k) = d.drop_high {
                    drop_highest(k as usize, &mut dropped);
                }
                let shown: Vec<String> = values.iter().zip(&dropped).map(|((_, s), x)| if *x { format!("~~{s}~~") } else { s.clone() }).collect();
                parts.push(format!("{prefix}[{}]", shown.join(", ")));
                let kept: Vec<i64> = values.iter().zip(&dropped).filter(|(_, x)| !**x).map(|((v, _), _)| *v).collect();
                if let Some((cmp, target)) = d.count_if {
                    let hits = kept.iter().filter(|v| cmp_ok(cmp, **v, target)).count() as i64;
                    successes = Some(successes.unwrap_or(0) + hits);
                } else {
                    total += sign * kept.iter().sum::<i64>();
                }
                if single_d20 && kept.len() == 1 {
                    if let Kind::Sides(s) = kind {
                        if s == 20 {
                            critical = match kept[0] {
                                20 => Some(true),
                                1 => Some(false),
                                _ => None,
                            };
                        }
                    }
                }
            }
        }
    }
    let detail = parts.concat();
    if let Some(hits) = successes {
        let single = terms.len() == 1 && matches!(&terms[0].1, Term::Dice(d) if d.count == 1);
        let summary = if single {
            if hits > 0 { t!("Success") } else { t!("Failure") }.to_owned()
        } else if hits == 1 {
            t!("1 success").to_owned()
        } else {
            t!("{} successes", hits)
        };
        return Ok(Roll { notation: notation.to_owned(), total: Some(hits), summary, detail, critical });
    }
    if !numeric {
        let mut summary: Vec<String> = symbols.iter().map(|(s, n)| if *n > 1 { format!("{s} ×{n}") } else { s.clone() }).collect();
        if total != 0 {
            summary.insert(0, total.to_string());
        }
        if summary.is_empty() {
            summary.push("–".into());
        }
        return Ok(Roll { notation: notation.to_owned(), total: None, summary: summary.join(", "), detail, critical });
    }
    let summary = if terms.iter().any(|(_, t)| matches!(t, Term::Dice(d) if d.kind == Some(Kind::Fate))) && total > 0 {
        format!("+{total}")
    } else {
        total.to_string()
    };
    Ok(Roll { notation: notation.to_owned(), total: Some(total), summary, detail, critical })
}

/// `check 12/14/13 5 [-2]`: three d20 against three attributes; rolls above
/// an attribute use up skill points. Two 1s are a critical success, two 20s
/// a botch. Quality level: 1 per 3 remaining points (at least 1).
fn check(notation: &str, rest: &str, rng: &mut dyn Rng) -> Result<Roll, String> {
    let bad = || t!("Unknown dice “{}”", notation);
    let mut words = rest.split_whitespace();
    let attrs: Vec<i64> = words.next().ok_or_else(bad)?.split('/').map(|a| a.trim().parse::<i64>()).collect::<Result<_, _>>().map_err(|_| bad())?;
    if attrs.is_empty() || attrs.len() > 3 {
        return Err(bad());
    }
    let skill: i64 = words.next().map(|w| w.trim_start_matches("FW").trim_start_matches("FP").parse()).transpose().map_err(|_| bad())?.unwrap_or(0);
    let modifier: i64 = words.next().map(|w| w.replace('−', "-").parse()).transpose().map_err(|_| bad())?.unwrap_or(0);
    let rolls: Vec<i64> = attrs.iter().map(|_| rng.die(20) as i64).collect();
    let ones = rolls.iter().filter(|r| **r == 1).count();
    let twenties = rolls.iter().filter(|r| **r == 20).count();
    let mut points = skill;
    for (r, a) in rolls.iter().zip(&attrs) {
        let over = r - (a + modifier);
        if over > 0 {
            points -= over;
        }
    }
    let detail = rolls.iter().zip(&attrs).map(|(r, a)| format!("{r}/{}", a + modifier)).collect::<Vec<_>>().join(" · ");
    let detail = if attrs.len() == 3 { format!("[{detail}], {}", t!("{} points left", points)) } else { format!("[{detail}]") };
    let (summary, critical, total) = if ones >= 2 {
        (t!("Critical success").to_owned(), Some(true), Some(points.max(0)))
    } else if twenties >= 2 {
        (t!("Botch").to_owned(), Some(false), None)
    } else if points >= 0 {
        let ql = if attrs.len() == 3 { ((points + 2) / 3).clamp(1, 6) } else { 1 };
        (if attrs.len() == 3 { t!("Success, QL {}", ql) } else { t!("Success").to_owned() }, None, Some(ql))
    } else {
        (t!("Failure").to_owned(), None, None)
    };
    Ok(Roll { notation: notation.to_owned(), total, summary, detail, critical })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Returns the given values in turn.
    pub struct Fixed(pub Vec<u32>, pub usize);

    impl Rng for Fixed {
        fn die(&mut self, sides: u32) -> u32 {
            let v = self.0[self.1 % self.0.len()];
            self.1 += 1;
            v.min(sides).max(1)
        }
    }

    fn r(notation: &str, values: &[u32]) -> Roll {
        roll(notation, &[CustomDie { name: "Hit die".into(), faces: vec!["Hit".into(), "Miss".into(), "Hit Crit".into(), "".into()] }], &mut Fixed(values.to_vec(), 0)).unwrap()
    }

    #[test]
    fn sums_and_modifiers() {
        let x = r("2d6+3", &[4, 5]);
        assert_eq!((x.total, x.detail.as_str()), (Some(12), "[4, 5] + 3"));
        assert_eq!(r("d20-1", &[20]).critical, Some(true));
        assert_eq!(r("1d20 - 2", &[1]).total, Some(-1));
        let x = r("2d20kh1+5", &[7, 15]);
        assert_eq!((x.total, x.detail.as_str()), (Some(20), "[~~7~~, 15] + 5"));
        assert_eq!(r("2d20kl1", &[7, 15]).total, Some(7));
        assert_eq!(r("4d6dl1", &[3, 1, 6, 4]).total, Some(13));
        assert_eq!(r("3d6!", &[6, 2, 3, 6, 1]).detail, "[6, 2!, 3, 6, 1!]");
        assert_eq!(r("2d6r1", &[1, 5, 3]).detail, "[~~1~~ 5, 3]");
        let x = r("6d10>=8", &[8, 2, 10, 7, 9, 1]);
        assert_eq!((x.total, x.summary.as_str()), (Some(3), "3 successes"));
        assert_eq!(r("1d20<=12", &[13]).summary, "Failure");
        assert_eq!(r("4dF", &[3, 3, 1, 2]).summary, "+1");
        assert_eq!(r("d%", &[8, 4]).total, Some(73));
        assert_eq!(r("d%", &[1, 1]).total, Some(100));
        assert_eq!(r("3W6", &[1, 2, 3]).total, Some(6));
    }

    #[test]
    fn custom_dice_and_errors() {
        let x = r("3d{Hit die}", &[1, 3, 4]);
        assert_eq!((x.summary.as_str(), x.detail.as_str()), ("Hit ×2, Crit", "[Hit, Hit Crit, –]"));
        assert!(roll("2d{Nope}", &[], &mut Fixed(vec![1], 0)).is_err());
        assert!(roll("hello", &[], &mut Fixed(vec![1], 0)).is_err());
        assert!(roll("2d", &[], &mut Fixed(vec![1], 0)).is_err());
        assert!(is_notation("2d6+3") && is_notation("d20") && is_notation("check 12/13/14 4"));
        assert!(!is_notation("12") && !is_notation("dead") && !is_notation("Leather"));
    }

    #[test]
    fn three_attribute_checks() {
        let x = r("check 12/14/13 5", &[10, 16, 13]);
        assert_eq!((x.summary.as_str(), x.total), ("Success, QL 1", Some(1)));
        let x = r("check 12/14/13 5", &[15, 17, 13]);
        assert_eq!(x.summary, "Failure");
        assert_eq!(r("check 12/14/13 5", &[1, 1, 19]).summary, "Critical success");
        assert_eq!(r("check 12/14/13 5", &[20, 20, 2]).summary, "Botch");
        assert_eq!(r("check 12/14/13 8 +2", &[2, 3, 4]).summary, "Success, QL 3");
    }
}
