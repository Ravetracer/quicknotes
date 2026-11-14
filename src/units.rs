//! Units of measurement for calculation sheets (see `calc.rs`): the unit
//! catalogue, quantities with dimensions and their display.
//!
//! A [`Quantity`] keeps its value in SI base units (`si`) together with the
//! dimension and the units it is shown in. Currencies use the euro as base;
//! their factors come from [`crate::rates`].

use std::collections::HashMap;
use std::sync::OnceLock;

pub const N_DIMS: usize = 9;
/// Exponents of length, mass, time, current, temperature, amount of
/// substance, information, angle and money.
pub type Dim = [i8; N_DIMS];
pub const NONE: Dim = [0; N_DIMS];
const ANGLE_DIM: usize = 7;
const MONEY_DIM: usize = 8;
const BASE_SYMBOLS: [&str; N_DIMS] = ["m", "kg", "s", "A", "K", "mol", "bit", "rad", "EUR"];

const fn dim(l: i8, m: i8, t: i8, i: i8, k: i8) -> Dim {
    [l, m, t, i, k, 0, 0, 0, 0]
}
const fn one(index: usize) -> Dim {
    let mut d = NONE;
    d[index] = 1;
    d
}

const LENGTH: Dim = dim(1, 0, 0, 0, 0);
const AREA: Dim = dim(2, 0, 0, 0, 0);
const VOLUME: Dim = dim(3, 0, 0, 0, 0);
const MASS: Dim = dim(0, 1, 0, 0, 0);
const TIME: Dim = dim(0, 0, 1, 0, 0);
const SPEED: Dim = dim(1, 0, -1, 0, 0);
const ACCELERATION: Dim = dim(1, 0, -2, 0, 0);
const FORCE: Dim = dim(1, 1, -2, 0, 0);
const PRESSURE: Dim = dim(-1, 1, -2, 0, 0);
const ENERGY: Dim = dim(2, 1, -2, 0, 0);
const POWER: Dim = dim(2, 1, -3, 0, 0);
const TEMPERATURE: Dim = dim(0, 0, 0, 0, 1);
const FREQUENCY: Dim = dim(0, 0, -1, 0, 0);
const CURRENT: Dim = dim(0, 0, 0, 1, 0);
const VOLTAGE: Dim = dim(2, 1, -3, -1, 0);
const RESISTANCE: Dim = dim(2, 1, -3, -2, 0);
const CHARGE: Dim = dim(0, 0, 1, 1, 0);
const FUEL: Dim = dim(-2, 0, 0, 0, 0);
const AMOUNT: Dim = one(5);
const DATA: Dim = one(6);
const DATA_RATE: Dim = [0, 0, -1, 0, 0, 0, 1, 0, 0];
const ANGLE: Dim = one(ANGLE_DIM);
const MONEY: Dim = one(MONEY_DIM);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Category {
    Length,
    Area,
    Volume,
    Mass,
    Time,
    Speed,
    Acceleration,
    Force,
    Pressure,
    Energy,
    Power,
    Temperature,
    Angle,
    Data,
    DataRate,
    Frequency,
    Electric,
    Amount,
    FuelEconomy,
    Currency,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::Length => t!("Length"),
            Self::Area => t!("Area"),
            Self::Volume => t!("Volume"),
            Self::Mass => t!("Mass"),
            Self::Time => t!("Time"),
            Self::Speed => t!("Speed"),
            Self::Acceleration => t!("Acceleration"),
            Self::Force => t!("Force"),
            Self::Pressure => t!("Pressure"),
            Self::Energy => t!("Energy"),
            Self::Power => t!("Power"),
            Self::Temperature => t!("Temperature"),
            Self::Angle => t!("Angle"),
            Self::Data => t!("Data"),
            Self::DataRate => t!("Data Rate"),
            Self::Frequency => t!("Frequency"),
            Self::Electric => t!("Electricity"),
            Self::Amount => t!("Amount of Substance"),
            Self::FuelEconomy => t!("Fuel Economy"),
            Self::Currency => t!("Currency"),
        }
    }

    /// The category of a dimension, for messages ("length", "mass", …).
    fn of(dim: &Dim) -> Option<Self> {
        let all = [
            (LENGTH, Self::Length),
            (AREA, Self::Area),
            (VOLUME, Self::Volume),
            (MASS, Self::Mass),
            (TIME, Self::Time),
            (SPEED, Self::Speed),
            (ACCELERATION, Self::Acceleration),
            (FORCE, Self::Force),
            (PRESSURE, Self::Pressure),
            (ENERGY, Self::Energy),
            (POWER, Self::Power),
            (TEMPERATURE, Self::Temperature),
            (ANGLE, Self::Angle),
            (DATA, Self::Data),
            (DATA_RATE, Self::DataRate),
            (FREQUENCY, Self::Frequency),
            (AMOUNT, Self::Amount),
            (FUEL, Self::FuelEconomy),
            (MONEY, Self::Currency),
        ];
        all.iter().find(|(d, _)| d == dim).map(|(_, c)| *c)
    }
}

/// Describes a dimension for error messages.
pub fn describe_dim(dim: &Dim) -> String {
    if *dim == NONE {
        return t!("a plain number").to_owned();
    }
    match Category::of(dim) {
        // German nouns keep their capital letter.
        Some(c) if crate::i18n::lang() == crate::i18n::Lang::De => c.label().to_owned(),
        Some(c) => c.label().to_lowercase(),
        None => format_units(&base_units(dim)),
    }
}

/// A unit as used in a quantity: symbol, dimension and conversion to SI
/// (`si = number * factor + offset`).
#[derive(Clone, Debug, PartialEq)]
pub struct Unit {
    pub symbol: String,
    pub dim: Dim,
    pub factor: f64,
    pub offset: f64,
}

// ===== catalogue ============================================================

struct Def {
    /// Space-separated names; the first one is shown.
    names: &'static str,
    name: &'static str,
    cat: Category,
    dim: Dim,
    factor: f64,
    offset: f64,
    /// Space-separated allowed prefixes.
    prefixes: &'static str,
}

const fn def(names: &'static str, name: &'static str, cat: Category, dim: Dim, factor: f64, prefixes: &'static str) -> Def {
    Def { names, name, cat, dim, factor, offset: 0.0, prefixes }
}

const PI: f64 = std::f64::consts::PI;
const US_GALLON: f64 = 3.785411784e-3;
const MILE: f64 = 1609.344;

use Category as C;

const DEFS: &[Def] = &[
    // Length
    def("m meter metre Meter", "metre", C::Length, LENGTH, 1.0, "k c m d µ μ u n"),
    def("in inch inches Zoll", "inch", C::Length, LENGTH, 0.0254, ""),
    def("ft foot feet Fuß", "foot", C::Length, LENGTH, 0.3048, ""),
    def("yd yard yards", "yard", C::Length, LENGTH, 0.9144, ""),
    def("mi mile miles Meile Meilen", "mile", C::Length, LENGTH, MILE, ""),
    def("nmi NM nauticalmile Seemeile Seemeilen", "nautical mile", C::Length, LENGTH, 1852.0, ""),
    def("Å angstrom", "ångström", C::Length, LENGTH, 1e-10, ""),
    def("au AU", "astronomical unit", C::Length, LENGTH, 1.495978707e11, ""),
    def("ly lightyear lightyears Lichtjahr Lichtjahre", "light-year", C::Length, LENGTH, 9.4607304725808e15, ""),
    def("pc parsec parsecs", "parsec", C::Length, LENGTH, 3.085677581491367e16, ""),
    // Area
    def("ha hectare hectares Hektar", "hectare", C::Area, AREA, 1e4, ""),
    def("ac acre acres", "acre", C::Area, AREA, 4046.8564224, ""),
    def("sqft", "square foot", C::Area, AREA, 0.09290304, ""),
    def("sqmi", "square mile", C::Area, AREA, MILE * MILE, ""),
    def("qm", "square metre", C::Area, AREA, 1.0, ""),
    // Volume
    def("L l liter litre liters litres Liter", "litre", C::Volume, VOLUME, 1e-3, "m c d h µ μ u"),
    def("ccm cc", "cubic centimetre", C::Volume, VOLUME, 1e-6, ""),
    def("gal gallon gallons", "US gallon", C::Volume, VOLUME, US_GALLON, ""),
    def("impgal", "imperial gallon", C::Volume, VOLUME, 4.54609e-3, ""),
    def("qt quart quarts", "US quart", C::Volume, VOLUME, US_GALLON / 4.0, ""),
    def("pt pint pints", "US pint", C::Volume, VOLUME, US_GALLON / 8.0, ""),
    def("cup cups", "US cup", C::Volume, VOLUME, US_GALLON / 16.0, ""),
    def("floz", "US fluid ounce", C::Volume, VOLUME, US_GALLON / 128.0, ""),
    def("tbsp", "tablespoon", C::Volume, VOLUME, US_GALLON / 256.0, ""),
    def("tsp", "teaspoon", C::Volume, VOLUME, US_GALLON / 768.0, ""),
    def("EL", "Esslöffel (15 ml)", C::Volume, VOLUME, 1.5e-5, ""),
    def("TL", "Teelöffel (5 ml)", C::Volume, VOLUME, 5e-6, ""),
    def("bbl barrel barrels", "oil barrel", C::Volume, VOLUME, 0.158987294928, ""),
    // Mass
    def("g gram grams gramme Gramm", "gram", C::Mass, MASS, 1e-3, "k m µ μ u"),
    def("t tonne tonnes Tonne Tonnen", "tonne", C::Mass, MASS, 1000.0, ""),
    def("lb lbs pound pounds", "pound", C::Mass, MASS, 0.45359237, ""),
    def("oz ounce ounces Unze", "ounce", C::Mass, MASS, 0.028349523125, ""),
    def("ozt", "troy ounce", C::Mass, MASS, 0.0311034768, ""),
    def("st stone", "stone", C::Mass, MASS, 6.35029318, ""),
    def("ct carat carats Karat", "carat", C::Mass, MASS, 2e-4, ""),
    def("gr grain grains", "grain", C::Mass, MASS, 6.479891e-5, ""),
    def("Pfund", "Pfund (500 g)", C::Mass, MASS, 0.5, ""),
    def("Zentner", "Zentner (50 kg)", C::Mass, MASS, 50.0, ""),
    // Time
    def("s sec second seconds Sekunde Sekunden", "second", C::Time, TIME, 1.0, "m µ μ u n"),
    def("min minute minutes Minute Minuten", "minute", C::Time, TIME, 60.0, ""),
    def("h hr hour hours Stunde Stunden", "hour", C::Time, TIME, 3600.0, ""),
    def("d day days Tag Tage", "day", C::Time, TIME, 86400.0, ""),
    def("wk week weeks Woche Wochen", "week", C::Time, TIME, 604800.0, ""),
    def("month months Monat Monate", "month (average)", C::Time, TIME, 2629746.0, ""),
    def("yr year years Jahr Jahre", "year (average)", C::Time, TIME, 31556952.0, ""),
    // Speed and acceleration
    def("kmh kph", "kilometre per hour", C::Speed, SPEED, 1.0 / 3.6, ""),
    def("mph", "mile per hour", C::Speed, SPEED, MILE / 3600.0, ""),
    def("kn kt knot knots Knoten", "knot", C::Speed, SPEED, 1852.0 / 3600.0, ""),
    def("gn", "standard gravity", C::Acceleration, ACCELERATION, 9.80665, ""),
    // Force, pressure
    def("N newton newtons", "newton", C::Force, FORCE, 1.0, "k M"),
    def("kp kgf", "kilopond", C::Force, FORCE, 9.80665, ""),
    def("lbf", "pound-force", C::Force, FORCE, 4.4482216152605, ""),
    def("Pa pascal", "pascal", C::Pressure, PRESSURE, 1.0, "h k M G"),
    def("bar", "bar", C::Pressure, PRESSURE, 1e5, "m"),
    def("atm", "standard atmosphere", C::Pressure, PRESSURE, 101325.0, ""),
    def("psi", "pound per square inch", C::Pressure, PRESSURE, 6894.757293168, ""),
    def("mmHg", "millimetre of mercury", C::Pressure, PRESSURE, 133.322387415, ""),
    def("Torr torr", "torr", C::Pressure, PRESSURE, 101325.0 / 760.0, ""),
    // Energy, power
    def("J joule joules", "joule", C::Energy, ENERGY, 1.0, "k M G"),
    def("Wh", "watt-hour", C::Energy, ENERGY, 3600.0, "k M G T"),
    def("cal calorie calories Kalorie Kalorien", "calorie", C::Energy, ENERGY, 4.184, "k"),
    def("eV", "electronvolt", C::Energy, ENERGY, 1.602176634e-19, "k M G"),
    def("BTU btu", "British thermal unit", C::Energy, ENERGY, 1055.05585262, ""),
    def("W watt watts", "watt", C::Power, POWER, 1.0, "m k M G T"),
    def("hp", "horsepower", C::Power, POWER, 745.6998715822702, ""),
    def("PS", "Pferdestärke", C::Power, POWER, 735.49875, ""),
    // Temperature (see `TEMPERATURES` for the offsets)
    def("K kelvin", "kelvin", C::Temperature, TEMPERATURE, 1.0, ""),
    // Angle
    def("rad radian radians", "radian", C::Angle, ANGLE, 1.0, "m"),
    def("° deg degree degrees Grad", "degree", C::Angle, ANGLE, PI / 180.0, ""),
    def("gon", "gradian", C::Angle, ANGLE, PI / 200.0, ""),
    def("arcmin ′", "arcminute", C::Angle, ANGLE, PI / 10800.0, ""),
    def("arcsec ″", "arcsecond", C::Angle, ANGLE, PI / 648000.0, ""),
    def("turn turns rev Umdrehung Umdrehungen", "turn", C::Angle, ANGLE, 2.0 * PI, ""),
    // Data
    def("bit bits", "bit", C::Data, DATA, 1.0, "k M G T"),
    def("B byte bytes Byte", "byte", C::Data, DATA, 8.0, "k M G T P Ki Mi Gi Ti Pi"),
    def("bps", "bit per second", C::DataRate, DATA_RATE, 1.0, "k M G"),
    // Frequency
    def("Hz hertz", "hertz", C::Frequency, FREQUENCY, 1.0, "k M G T"),
    def("rpm", "revolutions per minute", C::Frequency, FREQUENCY, 1.0 / 60.0, ""),
    def("bpm", "beats per minute", C::Frequency, FREQUENCY, 1.0 / 60.0, ""),
    // Electricity
    def("A amp amps ampere amperes", "ampere", C::Electric, CURRENT, 1.0, "m µ μ u k"),
    def("V volt volts", "volt", C::Electric, VOLTAGE, 1.0, "m k M"),
    def("Ω Ω ohm ohms", "ohm", C::Electric, RESISTANCE, 1.0, "m k M"),
    def("Ah", "ampere-hour", C::Electric, CHARGE, 3600.0, "m"),
    def("mol", "mole", C::Amount, AMOUNT, 1.0, "m µ μ u k"),
    // Fuel economy (distance per volume; converts to and from L/100km)
    def("mpg", "miles per US gallon", C::FuelEconomy, FUEL, MILE / US_GALLON, ""),
    def("mpgimp", "miles per imperial gallon", C::FuelEconomy, FUEL, MILE / 4.54609e-3, ""),
];

/// Temperature scales with an offset: names, display name, factor, offset.
const TEMPERATURES: &[(&str, &str, f64, f64)] = &[
    ("°C degC celsius Celsius", "degree Celsius", 1.0, 273.15),
    ("°F degF fahrenheit Fahrenheit", "degree Fahrenheit", 5.0 / 9.0, 459.67 * 5.0 / 9.0),
];

/// Currencies with ECB reference rates: code, name, extra names.
pub const CURRENCIES: &[(&str, &str, &str)] = &[
    ("EUR", "euro", "€ eur euro euros Euro"),
    ("USD", "US dollar", "$ usd dollar dollars Dollar"),
    ("GBP", "pound sterling", "£ gbp"),
    ("CHF", "Swiss franc", "chf"),
    ("JPY", "Japanese yen", "¥ jpy yen"),
    ("CNY", "Chinese yuan", "cny yuan"),
    ("CAD", "Canadian dollar", "cad"),
    ("AUD", "Australian dollar", "aud"),
    ("NZD", "New Zealand dollar", "nzd"),
    ("SEK", "Swedish krona", "sek"),
    ("NOK", "Norwegian krone", "nok"),
    ("DKK", "Danish krone", "dkk"),
    ("ISK", "Icelandic króna", "isk"),
    ("PLN", "Polish złoty", "pln"),
    ("CZK", "Czech koruna", "czk"),
    ("HUF", "Hungarian forint", "huf"),
    ("RON", "Romanian leu", "ron"),
    ("BGN", "Bulgarian lev", "bgn"),
    ("TRY", "Turkish lira", "₺ try"),
    ("ILS", "Israeli shekel", "₪ ils"),
    ("INR", "Indian rupee", "₹ inr"),
    ("KRW", "South Korean won", "₩ krw"),
    ("HKD", "Hong Kong dollar", "hkd"),
    ("SGD", "Singapore dollar", "sgd"),
    ("THB", "Thai baht", "thb"),
    ("MYR", "Malaysian ringgit", "myr"),
    ("IDR", "Indonesian rupiah", "idr"),
    ("PHP", "Philippine peso", "php"),
    ("MXN", "Mexican peso", "mxn"),
    ("BRL", "Brazilian real", "brl"),
    ("ZAR", "South African rand", "zar"),
];

const PREFIXES: &[(&str, &str, f64)] = &[
    ("n", "nano", 1e-9),
    ("µ", "micro", 1e-6),
    ("μ", "micro", 1e-6),
    ("u", "micro", 1e-6),
    ("m", "milli", 1e-3),
    ("c", "centi", 1e-2),
    ("d", "deci", 1e-1),
    ("h", "hecto", 1e2),
    ("k", "kilo", 1e3),
    ("M", "mega", 1e6),
    ("G", "giga", 1e9),
    ("T", "tera", 1e12),
    ("P", "peta", 1e15),
    ("Ki", "kibi", 1024.0),
    ("Mi", "mebi", 1048576.0),
    ("Gi", "gibi", 1073741824.0),
    ("Ti", "tebi", 1099511627776.0),
    ("Pi", "pebi", 1125899906842624.0),
];

/// A unit of the catalogue.
#[derive(Clone, Debug)]
pub struct Entry {
    pub symbol: String,
    pub name: String,
    pub category: Category,
    dim: Dim,
    factor: f64,
    offset: f64,
    /// ISO code of a currency (factor from the exchange rates).
    currency: Option<&'static str>,
    /// Listed in completions (unprefixed or a common prefix).
    pub common: bool,
}

struct Catalogue {
    entries: Vec<Entry>,
    index: HashMap<String, usize>,
    /// Long names in lower case ("Kilometer" finds "kilometer").
    lower: HashMap<String, usize>,
}

const COMMON_PREFIXED: &[&str] = &[
    "km", "cm", "mm", "µm", "nm", "kg", "mg", "mL", "ml", "cl", "dl", "ms", "µs", "kN", "hPa", "kPa", "MPa", "mbar", "kJ",
    "MJ", "kWh", "MWh", "kcal", "kW", "MW", "kbit", "Mbit", "Gbit", "kB", "MB", "GB", "TB", "KiB", "MiB", "GiB", "kbps",
    "Mbps", "Gbps", "kHz", "MHz", "GHz", "mA", "kV", "mV", "kΩ", "mAh",
];

fn catalogue() -> &'static Catalogue {
    static CAT: OnceLock<Catalogue> = OnceLock::new();
    CAT.get_or_init(|| {
        let mut c = Catalogue { entries: Vec::new(), index: HashMap::new(), lower: HashMap::new() };
        let add = |c: &mut Catalogue, entry: Entry, names: &[String]| {
            let i = c.entries.len();
            c.entries.push(entry);
            for n in names {
                c.index.entry(n.clone()).or_insert(i);
                if n.chars().count() > 3 {
                    c.lower.entry(n.to_lowercase()).or_insert(i);
                }
            }
        };
        let long = |n: &str| n.chars().count() > 3 && n.chars().all(|c| c.is_ascii_lowercase());
        for d in DEFS {
            let names: Vec<&str> = d.names.split(' ').collect();
            let mut all: Vec<String> = Vec::new();
            for n in &names {
                all.push((*n).to_owned());
                if long(n) && !n.ends_with('s') {
                    all.push(format!("{n}s"));
                }
            }
            let entry = |symbol: String, name: String, factor: f64, common: bool| Entry {
                symbol,
                name,
                category: d.cat,
                dim: d.dim,
                factor,
                offset: d.offset,
                currency: None,
                common,
            };
            add(&mut c, entry(names[0].to_owned(), d.name.to_owned(), d.factor, true), &all);
            for p in d.prefixes.split(' ').filter(|p| !p.is_empty()) {
                let Some((_, long_prefix, f)) = PREFIXES.iter().find(|(s, _, _)| s == &p) else { continue };
                let mut prefixed = Vec::new();
                for n in &names {
                    if n.chars().count() <= 3 {
                        prefixed.push(format!("{p}{n}"));
                    } else if long(n) {
                        prefixed.push(format!("{long_prefix}{n}"));
                        prefixed.push(format!("{long_prefix}{n}s"));
                    }
                }
                let symbol = format!("{p}{}", names[0]);
                let common = COMMON_PREFIXED.contains(&symbol.as_str());
                add(&mut c, entry(symbol, format!("{long_prefix}{}", d.name), d.factor * f, common), &prefixed);
            }
        }
        for (names, name, factor, offset) in TEMPERATURES {
            let names: Vec<String> = names.split(' ').map(str::to_owned).collect();
            let entry = Entry {
                symbol: names[0].clone(),
                name: (*name).to_owned(),
                category: C::Temperature,
                dim: TEMPERATURE,
                factor: *factor,
                offset: *offset,
                currency: None,
                common: true,
            };
            add(&mut c, entry, &names);
        }
        for (code, name, extra) in CURRENCIES {
            let mut names = vec![(*code).to_owned()];
            names.extend(extra.split(' ').map(str::to_owned));
            let entry = Entry {
                symbol: (*code).to_owned(),
                name: (*name).to_owned(),
                category: C::Currency,
                dim: MONEY,
                factor: 1.0,
                offset: 0.0,
                currency: Some(code),
                common: true,
            };
            add(&mut c, entry, &names);
        }
        c
    })
}

/// All units of the catalogue (for completion and help).
pub fn entries() -> &'static [Entry] {
    &catalogue().entries
}

fn find(name: &str) -> Option<&'static Entry> {
    let c = catalogue();
    let i = c.index.get(name).or_else(|| (name.chars().count() > 3).then(|| c.lower.get(&name.to_lowercase())).flatten())?;
    c.entries.get(*i)
}

/// Looks up a unit by name: `None` for unknown names, `Some(Err)` when a
/// currency has no exchange rate yet. `m2`/`m3` mean m²/m³.
pub fn lookup(name: &str) -> Option<Result<Quantity, String>> {
    if let Some(entry) = find(name) {
        return Some(entry_unit(entry).map(|u| Quantity::unit(u, 1)));
    }
    let power = match name.chars().last()? {
        '2' => 2,
        '3' => 3,
        _ => return None,
    };
    let entry = find(&name[..name.len() - 1]).filter(|e| e.offset == 0.0 && e.currency.is_none())?;
    Some(entry_unit(entry).map(|u| Quantity::unit(u, power)))
}

fn entry_unit(entry: &Entry) -> Result<Unit, String> {
    let factor = match entry.currency {
        Some(code) => 1.0 / crate::rates::rate(code)?,
        None => entry.factor,
    };
    Ok(Unit { symbol: entry.symbol.clone(), dim: entry.dim, factor, offset: entry.offset })
}

impl Entry {
    pub fn dim(&self) -> Dim {
        self.dim
    }
}

/// Unit expressions offered by the converter dialog, per category.
pub const DIALOG: &[(Category, &[&str])] = &[
    (C::Length, &["mm", "cm", "m", "km", "in", "ft", "yd", "mi", "nmi", "au", "ly"]),
    (C::Area, &["mm²", "cm²", "m²", "km²", "ha", "in²", "ft²", "yd²", "ac", "mi²"]),
    (C::Volume, &["mL", "cL", "L", "m³", "cm³", "tsp", "tbsp", "floz", "cup", "pt", "qt", "gal", "impgal", "bbl"]),
    (C::Mass, &["mg", "g", "kg", "t", "oz", "lb", "st", "ct", "ozt"]),
    (C::Time, &["ms", "s", "min", "h", "d", "wk", "month", "yr"]),
    (C::Speed, &["m/s", "km/h", "mph", "kn", "ft/s"]),
    (C::Temperature, &["°C", "°F", "K"]),
    (C::Pressure, &["Pa", "hPa", "kPa", "bar", "mbar", "atm", "psi", "mmHg"]),
    (C::Energy, &["J", "kJ", "cal", "kcal", "Wh", "kWh", "eV", "BTU"]),
    (C::Power, &["W", "kW", "MW", "hp", "PS"]),
    (C::Force, &["N", "kN", "kp", "lbf"]),
    (C::Angle, &["°", "rad", "gon", "arcmin", "arcsec", "turn"]),
    (C::Data, &["bit", "B", "kB", "MB", "GB", "TB", "KiB", "MiB", "GiB", "TiB"]),
    (C::DataRate, &["bps", "kbps", "Mbps", "Gbps", "MB/s"]),
    (C::Frequency, &["Hz", "kHz", "MHz", "GHz", "rpm"]),
    (C::FuelEconomy, &["L/100km", "km/L", "mpg", "mpgimp"]),
];

/// Display name of a unit expression of [`DIALOG`].
pub fn dialog_label(expr: &str) -> String {
    match find(expr) {
        Some(e) => format!("{} ({})", e.symbol, e.name),
        None => expr.to_owned(),
    }
}

// ===== quantities ===========================================================

/// A number with dimension and display units. Without units it is a plain
/// number (or shown in SI base units when it has a dimension).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Quantity {
    pub si: f64,
    pub dim: Dim,
    pub units: Vec<(Unit, i8)>,
    /// A temperature difference: offsets do not apply.
    pub delta: bool,
    /// Shown split into these units ("1 h 30 min").
    pub mixed: Vec<Unit>,
}

fn add_dims(a: &Dim, b: &Dim, sign: i8) -> Dim {
    let mut d = *a;
    for (x, y) in d.iter_mut().zip(b) {
        *x += y * sign;
    }
    d
}

fn scale_dim(a: &Dim, n: i8) -> Dim {
    a.map(|x| x * n)
}

impl Quantity {
    pub fn plain(v: f64) -> Self {
        Self { si: v, ..Default::default() }
    }

    /// `1 unit^power` (offsets ignored). Units without dimension (`unit
    /// dozen = 12`) are plain numbers.
    pub fn unit(u: Unit, power: i8) -> Self {
        if u.dim == NONE {
            return Self::plain(u.factor.powi(power as i32));
        }
        Self { si: u.factor.powi(power as i32), dim: scale_dim(&u.dim, power), units: vec![(u, power)], ..Default::default() }
    }

    pub fn is_plain(&self) -> bool {
        self.dim == NONE && self.units.is_empty()
    }

    /// `number unit` of a literal like `20 °C`: offsets apply.
    pub fn literal(number: f64, unit: &Quantity) -> Self {
        let mut q = unit.clone();
        match q.offset_unit() {
            Some(u) => q.si = number * u.factor + u.offset,
            None => q.si = number * unit.si,
        }
        q
    }

    /// The single unit with an offset this quantity is shown in, if any.
    fn offset_unit(&self) -> Option<&Unit> {
        match self.units.as_slice() {
            [(u, 1)] if u.offset != 0.0 && !self.delta => Some(u),
            _ => None,
        }
    }

    /// Product of the display factors.
    fn display_factor(&self) -> f64 {
        self.units.iter().map(|(u, p)| u.factor.powi(*p as i32)).product()
    }

    /// The value in its display units.
    pub fn number(&self) -> f64 {
        match self.offset_unit() {
            Some(u) => (self.si - u.offset) / u.factor,
            None => self.si / self.display_factor(),
        }
    }

    /// The same units with another displayed number.
    pub fn with_number(&self, n: f64) -> Self {
        let mut q = self.clone();
        q.si = match self.offset_unit() {
            Some(u) => n * u.factor + u.offset,
            None => n * self.display_factor(),
        };
        q
    }

    pub fn is_currency(&self) -> bool {
        self.dim == MONEY
    }

    fn unit_offset(&self) -> f64 {
        self.offset_unit().map_or(0.0, |u| u.offset)
    }

    /// `self ± other`; the result keeps the units of `self`.
    pub fn add(&self, other: &Self, subtract: bool) -> Result<Self, String> {
        if self.dim != other.dim {
            return Err(incompatible(self, other, subtract));
        }
        let mut q = if self.units.is_empty() && !other.units.is_empty() { other.clone() } else { self.clone() };
        q.mixed.clear();
        // Temperatures: the right side counts as a difference.
        let right = other.si - other.unit_offset();
        q.si = if subtract { self.si - right } else { self.si + right };
        if subtract && self.offset_unit().is_some() && other.offset_unit().is_some() {
            q.si = self.si - other.si;
            q.delta = true;
        }
        Ok(q)
    }

    /// `self * other^sign` (sign 1 multiplies, -1 divides).
    pub fn mul(&self, other: &Self, sign: i8) -> Self {
        let si = if sign > 0 { self.si * other.si } else { self.si / other.si };
        let dim = add_dims(&self.dim, &other.dim, sign);
        let mut units = self.units.clone();
        for (u, p) in &other.units {
            let p = p * sign;
            if let Some(slot) = units.iter_mut().find(|(v, _)| v.symbol == u.symbol || v.dim == u.dim) {
                slot.1 += p;
            } else {
                units.push((u.clone(), p));
            }
        }
        units.retain(|(_, p)| *p != 0);
        if dim == NONE {
            units.clear();
        }
        Self { si, dim, units, delta: self.delta || other.delta, mixed: Vec::new() }
    }

    /// `self ^ n` for a whole or root exponent.
    pub fn pow(&self, n: f64) -> Result<Self, String> {
        if self.dim == NONE {
            let mut q = self.clone();
            q.si = self.si.powf(n);
            q.units.clear();
            return Ok(q);
        }
        let whole = n.round();
        if (n - whole).abs() < 1e-9 && whole.abs() <= 12.0 {
            let k = whole as i8;
            return Ok(Self {
                si: self.si.powi(k as i32),
                dim: scale_dim(&self.dim, k),
                units: self.units.iter().map(|(u, p)| (u.clone(), p * k)).filter(|(_, p)| *p != 0).collect(),
                ..Default::default()
            });
        }
        // Roots: every exponent must be divisible.
        let root = (1.0 / n).round();
        if (1.0 / n - root).abs() < 1e-9 && root >= 2.0 {
            let r = root as i8;
            if self.dim.iter().all(|x| x % r == 0) {
                let units = if self.units.iter().all(|(_, p)| p % r == 0) {
                    self.units.iter().map(|(u, p)| (u.clone(), p / r)).collect()
                } else {
                    Vec::new()
                };
                return Ok(Self { si: self.si.powf(n), dim: self.dim.map(|x| x / r), units, ..Default::default() });
            }
        }
        Err(t!("cannot raise {} to the power {}", describe_dim(&self.dim), crate::calc::format_number(n)))
    }

    /// The text shown as result ("3.2 m", "1 h 30 min").
    pub fn display(&self) -> String {
        if !self.mixed.is_empty() {
            return format_mixed(self.si, &self.mixed);
        }
        if self.is_plain() {
            return crate::calc::format_number(self.si);
        }
        if self.units.is_empty() {
            let base = base_units(&self.dim);
            return format!("{} {}", format_amount(self.si, false), format_units(&base));
        }
        let unit = format_units(&self.units);
        let n = format_amount(self.number(), self.is_currency());
        if unit.starts_with(['°', '′', '″']) && unit.chars().count() == 1 {
            format!("{n}{unit}")
        } else {
            format!("{n} {unit}")
        }
    }
}

fn incompatible(a: &Quantity, b: &Quantity, subtract: bool) -> String {
    let (x, y) = (describe_dim(&a.dim), describe_dim(&b.dim));
    if subtract {
        t!("cannot subtract {} from {}", y, x)
    } else {
        t!("cannot add {} and {}", x, y)
    }
}

/// SI base units of a dimension.
fn base_units(dim: &Dim) -> Vec<(Unit, i8)> {
    dim.iter()
        .zip(BASE_SYMBOLS)
        .filter(|(p, _)| **p != 0)
        .map(|(p, s)| (Unit { symbol: s.to_owned(), dim: NONE, factor: 1.0, offset: 0.0 }, *p))
        .collect()
}

fn superscript(n: i8) -> String {
    const DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    let mut s = String::new();
    if n < 0 {
        s.push('⁻');
    }
    for c in n.unsigned_abs().to_string().chars() {
        s.push(DIGITS[c.to_digit(10).unwrap_or(0) as usize]);
    }
    s
}

/// `km/h`, `m/s²`, `kg·m²`, `1/s`, `kg/(m·s²)`.
pub fn format_units(units: &[(Unit, i8)]) -> String {
    if units.is_empty() {
        return String::new();
    }
    let part = |u: &Unit, p: i8| if p == 1 { u.symbol.clone() } else { format!("{}{}", u.symbol, superscript(p)) };
    let num: Vec<String> = units.iter().filter(|(_, p)| *p > 0).map(|(u, p)| part(u, *p)).collect();
    let den: Vec<String> = units.iter().filter(|(_, p)| *p < 0).map(|(u, p)| part(u, -p)).collect();
    let mut s = if num.is_empty() { "1".to_owned() } else { num.join("·") };
    match den.len() {
        0 => {}
        1 => s = format!("{s}/{}", den[0]),
        _ => s = format!("{s}/({})", den.join("·")),
    }
    s
}

/// Numbers with units: about six significant digits (at most four
/// decimals); money with two decimals.
pub fn format_amount(v: f64, money: bool) -> String {
    if money {
        let s = format!("{v:.2}");
        return if s == "-0.00" { "0.00".into() } else { s };
    }
    let a = v.abs();
    if a == 0.0 {
        return "0".into();
    }
    if !(1e-6..1e15).contains(&a) {
        let s = format!("{v:.5e}");
        let (mantissa, exp) = s.split_once('e').unwrap_or((&s, "0"));
        let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
        return format!("{mantissa}e{exp}");
    }
    let magnitude = a.log10().floor() as i32;
    let decimals = if magnitude >= 0 { (5 - magnitude).clamp(0, 4) } else { 5 - magnitude } as usize;
    let s = format!("{v:.decimals$}");
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.') } else { &s };
    if s == "-0" { "0".into() } else { s.to_owned() }
}

/// "1 h 30 min", "5 ft 3.2 in": whole numbers of every unit but the last.
fn format_mixed(si: f64, units: &[Unit]) -> String {
    let negative = si < 0.0;
    let mut rest = si.abs();
    let mut parts = Vec::new();
    for (i, u) in units.iter().enumerate() {
        let n = rest / u.factor;
        if i + 1 == units.len() {
            let text = format_amount(n, false);
            if text != "0" || parts.is_empty() {
                parts.push(format!("{text} {}", u.symbol));
            }
        } else {
            // Rounding errors must not turn 2 h into 1 h 60 min.
            let whole = (n + 1e-9).floor();
            if whole > 0.0 {
                parts.push(format!("{} {}", crate::calc::format_number(whole), u.symbol));
            }
            rest = (rest - whole * u.factor).max(0.0);
        }
    }
    let text = parts.join(" ");
    if negative { format!("-{text}") } else { text }
}

/// Units for "convert to several units", largest first; `None` unless all
/// are known, of the same dimension and without offset.
pub fn mixed_target(names: &[&str]) -> Option<Result<Vec<Unit>, String>> {
    if names.len() < 2 {
        return None;
    }
    let mut units = Vec::new();
    for n in names {
        let entry = find(n)?;
        if entry.offset != 0.0 {
            return None;
        }
        match entry_unit(entry) {
            Ok(u) => units.push(u),
            Err(e) => return Some(Err(e)),
        }
    }
    if units.windows(2).any(|w| w[0].dim != w[1].dim) {
        return None;
    }
    units.sort_by(|a, b| b.factor.total_cmp(&a.factor));
    Some(Ok(units))
}

/// Whether a quantity counts as an angle (or plain radians) for trigonometry.
pub fn is_angle(q: &Quantity) -> bool {
    q.dim == NONE || q.dim == ANGLE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: f64, name: &str) -> Quantity {
        Quantity::literal(n, &lookup(name).unwrap().unwrap())
    }

    #[test]
    fn prefixes_and_names() {
        assert_eq!(q(1.0, "km").si, 1000.0);
        assert_eq!(q(1.0, "Kilometer").si, 1000.0);
        assert_eq!(q(1.0, "kilometres").si, 1000.0);
        assert_eq!(q(2.0, "MiB").si, 2.0 * 8.0 * 1048576.0);
        assert!((q(1.0, "mL").si - 1e-6).abs() < 1e-15);
        assert_eq!(q(1.0, "m2").display(), "1 m²");
        assert!(lookup("foo").is_none());
    }

    #[test]
    fn display() {
        let speed = q(120.0, "km").mul(&q(1.5, "h"), -1);
        assert_eq!(speed.display(), "80 km/h");
        assert_eq!(q(3.0, "m").add(&q(20.0, "cm"), false).unwrap().display(), "3.2 m");
        assert_eq!(q(20.0, "°C").display(), "20 °C");
        assert_eq!(q(90.0, "°").display(), "90°");
        assert_eq!(format_amount(10.498687664, false), "10.4987");
        assert_eq!(format_amount(0.000123456789, false), "0.000123457");
        assert_eq!(format_amount(1234567.8, false), "1234568");
        assert_eq!(format_amount(2.5, true), "2.50");
        assert_eq!(format_units(&q(1.0, "kg").mul(&q(1.0, "m"), -1).mul(&q(1.0, "s").pow(2.0).unwrap(), -1).units), "kg/(m·s²)");
    }

    #[test]
    fn temperatures() {
        let diff = q(20.0, "°C").add(&q(15.0, "°C"), true).unwrap();
        assert_eq!(diff.display(), "5 °C");
        assert!(diff.delta);
        assert_eq!(q(20.0, "°C").add(&q(5.0, "°C"), false).unwrap().display(), "25 °C");
    }

    #[test]
    fn mixed_units() {
        let units = mixed_target(&["min", "h"]).unwrap().unwrap();
        assert_eq!(format_mixed(5400.0, &units), "1 h 30 min");
        assert_eq!(format_mixed(7200.0, &units), "2 h");
        assert!(mixed_target(&["h", "m"]).is_none());
    }
}
