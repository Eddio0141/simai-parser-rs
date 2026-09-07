use std::{collections::HashMap, num::NonZeroU8, str::FromStr};

pub mod errors;
mod parser;

pub use errors::*;
use nom::Finish;

#[derive(Debug, Clone, PartialEq)]
pub struct SimaiFile<'a> {
    pub title: Option<&'a str>,
    pub artist: Option<&'a str>,
    pub designers: HashMap<u64, &'a str>,
    pub first: f64,
    pub level_names: HashMap<u64, &'a str>,
    pub wholebpm: Option<f64>,
    pub other_fields: HashMap<&'a str, &'a str>,
    pub maps: HashMap<u64, Map>,
}

impl<'a> FromStr for SimaiFile<'a> {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let file = parser::simai_file(s).finish().unwrap();
        dbg!(file);

        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Map {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl Button {
    pub fn new(value: u8) -> Option<Button> {
        match value {
            1 => Some(Self::A1),
            2 => Some(Self::A2),
            3 => Some(Self::A3),
            4 => Some(Self::A4),
            5 => Some(Self::A5),
            6 => Some(Self::A6),
            7 => Some(Self::A7),
            8 => Some(Self::A8),
            0 | 9.. => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tap {
    pub button: Button,
    pub break_: bool,
}

#[derive(Debug, Clone)]
pub struct Hold {
    pub button: Button,
    pub break_: bool,
    pub length: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Slide {
    /// Slide tap and slide track start
    pub start: Button,
    /// If the slide tap is break
    pub break_note: bool,
    /// If the slide track is break
    pub break_slide: bool,
    /// Duration of time to wait before the slide star moves
    pub wait_duration: f64,
    /// All sequence of tracks of this slide
    pub tracks: Vec<SlideTrack>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlideTrack {
    /// Track start
    pub start: Button,
    /// Track end
    pub end: Button,
    /// Duration of this track
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
