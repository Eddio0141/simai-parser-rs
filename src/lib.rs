use exn::{Result, ensure};
use nom::Finish;
use nom_language::error::convert_error;
use std::collections::HashMap;
use strum_macros::{EnumString, FromRepr};

pub mod errors;
mod parser;

pub use errors::*;

use crate::parser::Field;

#[derive(Debug, Clone, PartialEq)]
/// Represents a simai file
pub struct SimaiFile<'a> {
    /// Title of the map
    pub title: Option<&'a str>,
    /// Music artist
    pub artist: Option<&'a str>,
    /// Map designers
    pub designers: HashMap<u64, &'a str>,
    /// Offset to first note
    pub first: f64,
    /// Names of the levels, where the key is the level number
    pub level_names: HashMap<u64, &'a str>,
    /// Display BPM of the map. Doesn't actually do anything in the map
    pub wholebpm: Option<f64>,
    /// Other unknown fields
    pub other_fields: HashMap<&'a str, &'a str>,
    /// All maps, where the key is the level number
    pub maps: HashMap<u64, Map>,
}

impl<'a> SimaiFile<'a> {
    pub fn parse(input: &'a str) -> Result<Self, SimaiParseError> {
        let (_, fields) = parser::simai_file(input)
            .finish()
            .map_err(|e| SimaiParseError::Nom(convert_error(input, e)))?;

        let mut title = None;
        let mut artist = None;
        let mut first = None;
        let mut wholebpm = None;
        let mut designers = HashMap::new();
        let mut level_names = HashMap::new();
        let mut other_fields = HashMap::new();
        let mut maps = HashMap::new();

        for field in fields {
            match field {
                Field::Title(value) => {
                    ensure!(title.is_none(), SimaiParseError::DuplicateField("title"));
                    title = Some(value)
                }
                Field::Artist(value) => {
                    ensure!(artist.is_none(), SimaiParseError::DuplicateField("artist"));
                    artist = Some(value)
                }
                Field::First(value) => {
                    ensure!(first.is_none(), SimaiParseError::DuplicateField("first"));
                    first = Some(value)
                }
                Field::WholeBpm(value) => {
                    ensure!(
                        wholebpm.is_none(),
                        SimaiParseError::DuplicateField("wholebpm")
                    );
                    wholebpm = Some(value)
                }
                Field::Designer((num, value)) => {
                    ensure!(
                        !designers.contains_key(&num),
                        SimaiParseError::DuplicateFieldEntry(num, "des")
                    );
                    designers.insert(num, value);
                }
                Field::Level((num, value)) => {
                    ensure!(
                        !level_names.contains_key(&num),
                        SimaiParseError::DuplicateFieldEntry(num, "lv")
                    );
                    level_names.insert(num, value);
                }
                Field::INote((num, events)) => {
                    ensure!(
                        !maps.contains_key(&num),
                        SimaiParseError::DuplicateFieldEntry(num, "inote")
                    );
                    maps.insert(num, Map::parse_map_events(first, events));
                }
                Field::OtherFields((name, value)) => {
                    other_fields.insert(name, value);
                }
            }
        }

        Ok(SimaiFile {
            title,
            artist,
            designers,
            first: first.unwrap_or_default(),
            level_names,
            wholebpm,
            other_fields,
            maps,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Represents a single map
pub struct Map {
    /// Map events, ordered by time
    ///
    /// Some events happens at the same time
    events: Vec<Event>,
}

impl Map {
    fn parse_map_events(first: Option<f64>, events: impl Iterator<Item = parser::Event>) -> Self {
        let mut time = first.unwrap_or_default();
        let mut bpm = 1.;
        let mut divider = 1.;
        let mut sep_secs = 1.;

        let mut res = Vec::new();

        for event in events {
            match event {
                parser::Event::Tap(tap) => {
                    res.push(Event {
                        time,
                        event_type: EventType::Tap(tap),
                    });
                }
                parser::Event::TapEach((tap, taps)) => {
                    res.push(Event {
                        time,
                        event_type: EventType::Tap(tap),
                    });

                    // Handle rest of taps
                    let mut psuedo_time = time;
                    for tap in taps {
                        if tap.psuedo_each {
                            psuedo_time += 0.001;
                        }

                        res.push(Event {
                            time: psuedo_time,
                            event_type: EventType::Tap(Tap {
                                position: tap.position,
                                break_: false,
                                ex: false,
                                star_shaped: None,
                            }),
                        });
                    }
                }
                parser::Event::Hold((hold, duration)) => {}
                parser::Event::Slide(slide_event) => todo!(),
                parser::Event::Touch(touch) => todo!(),
                parser::Event::TouchHold(_) => todo!(),
                parser::Event::Sep => time += sep_secs,
                parser::Event::Bpm(value) => bpm = value,
                // TODO: does this change BPM?
                parser::Event::ExplicitDuration(value) => sep_secs = value,
                parser::Event::LengthDivider(value) => {
                    sep_secs = 240. / bpm / divider;
                    divider = value;
                }
            }
        }

        Map { events: res }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// Event time in seconds
    pub time: f64,
    pub event_type: EventType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    Tap(Tap),
    Hold(Hold),
    Slide(Slide),
    Touch(Touch),
    TouchHold(TouchHold),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
/// Button, separate from sensor
pub enum Button {
    A1 = 1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    A8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
/// Touch sensor on the screen
pub enum Sensor {
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    A8,
    B1,
    B2,
    B3,
    B4,
    B5,
    B6,
    B7,
    B8,
    C1,
    C2,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    E1,
    E2,
    E3,
    E4,
    E5,
    E6,
    E7,
    E8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Tap note
pub struct Tap {
    /// Note position
    pub position: Button,
    /// Break note
    pub break_: bool,
    /// Ex note
    pub ex: bool,
    /// Properties of a star shaped tap
    pub star_shaped: Option<StarShapedTap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StarShapedTap {
    /// Star shaped but not rotating
    NonRotating,
    /// Star shaped and rotating with a constant speed
    Rotating,
}

#[derive(Debug, Clone, PartialEq)]
/// Hold note
pub struct Hold {
    /// Note position
    pub position: Button,
    /// Break note
    pub break_: bool,
    /// Ex note
    pub ex: bool,
    /// Duration of the hold in seconds
    pub duration: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Slide {
    /// Slide tap and slide track start
    pub start: Button,
    /// If the slide tap is break
    pub break_tap: bool,
    /// Ex note for the tap
    pub ex_tap: bool,
    /// If the slide track is break
    pub break_slide: bool,
    /// Special slide tap properties
    pub special_tap: Option<SpecialSlideTap>,
    /// Number of seconds to wait before the slide star moves
    pub wait_duration: f64,
    /// All sequence of tracks of this slide
    pub tracks: Vec<SlideTrack>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialSlideTap {
    /// Changes the starting star-shaped tap to be normal
    NormalTap,
    /// Removes the star-shaped tap, and the tracing star fades in before moving
    NoStarTapFadeIn,
    /// Removes the star-shaped tap, and the tracing star appears when moving
    NoStarTapInstant,
}

#[derive(Debug, Clone, PartialEq)]
/// A single track of a slide
pub struct SlideTrack {
    /// Track start
    pub start: Button,
    /// Track end
    pub end: Button,
    /// Track length in number of seconds
    pub duration: f64,
    /// Slide variant
    pub variant: SlidePattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
/// Slide pattern
///
/// Currently this follows the specs noted in https://w.atwiki.jp/simai/pages/1003.html#id_a16709b9
pub enum SlidePattern {
    /// A straight line
    Line,
    /// Circle around the screen, moving right
    ArcRight,
    /// Circle around the screen, moving left
    ArcLeft,
    /// Circle around the screen, moving the shorter path. It is never half a circle in length
    ArcShort,
    /// Goes from start to center to end in two straight lines
    Shape_v,
    /// Goes from start to end while curving around the center
    Shape_p,
    /// Goes from start to end while curving around the center
    Shape_q,
    /// Makes a thunderbolt shape in an s shape
    Thunder_s,
    /// Makes a thunderbolt shape in an z shape
    Thunder_z,
    /// Grand p shape. Connects the start to the end while curving along an imaginary circle which is tangent to both the screen center and the circled judgment line
    Grand_p,
    /// Grand q shape. Connects the start to the end while curving along an imaginary circle which is tangent to both the screen center and the circled judgment line
    Grand_q,
    /// Grand v shape. Connects the start to the end through a middle turning point in two straight lines. The line connecting the start to the middle turning point is always a short straight line
    Grand_v { middle: Button },
    /// Fan shape, also called the Wi-Fi slider. Consists of three separate tracks
    Fan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Touch note
pub struct Touch {
    /// Note position
    pub position: Sensor,
    /// If the touch has a firework effect
    pub firework: bool,
}

#[derive(Debug, Clone, PartialEq)]
/// Touch hold note
pub struct TouchHold {
    /// Note position
    pub position: Sensor,
    /// Duration of hold in seconds
    pub duration: f64,
    /// If the touch has a firework effect
    pub firework: bool,
}
