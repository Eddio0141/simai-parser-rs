use std::collections::HashMap;

use nom::{
    Err, Parser,
    branch::{alt, permutation},
    bytes::complete::{tag, take_till1},
    character::complete::{char, multispace0, not_line_ending, u8, u64},
    combinator::{all_consuming, cut, map, map_opt, map_parser, opt, peek, success, value},
    error::{ParseError, context},
    multi::{many_till, many0, many1},
    number::complete::double,
    sequence::{delimited, pair, preceded, separated_pair, terminated},
};
use nom_language::error::VerboseError;

use crate::{
    Button, DuplicateFieldError, Hold, Map, SimaiFile, Slide, SlidePattern, SlideTrack, Tap,
};

pub type IResult<I, O, E = VerboseError<I>> = Result<(I, O), Err<E>>;

fn file_field_start<'a, O, E: ParseError<&'a str>>(
    field: impl Parser<&'a str, Output = O, Error = E>,
) -> impl Parser<&'a str, Output = O, Error = E> {
    delimited(char('&'), field, char('='))
}

fn file_field_line<'a, O, E: ParseError<&'a str>>(
    field: impl Parser<&'a str, Output = O, Error = E>,
) -> impl Parser<&'a str, Output = &'a str, Error = E> {
    preceded(file_field_start(field), not_line_ending)
}

fn field_with_num(input: &str) -> IResult<&str, u64> {
    delimited(char('_'), u64, peek(char('='))).parse(input)
}

fn button(input: &str) -> IResult<&str, Button> {
    map_opt(u8, Button::new).parse(input)
}

fn break_(input: &str) -> IResult<&str, bool> {
    let (s, value) = opt(char('b')).parse(input)?;
    Ok((s, value.is_some()))
}

type NoteDiv = (u64, u64);

fn length_note_div(input: &str) -> IResult<&str, NoteDiv> {
    separated_pair(u64, char(':'), u64).parse(input)
}

fn duration_secs(input: &str) -> IResult<&str, f64> {
    preceded(char('#'), double).parse(input)
}

fn bpm_with_note_div(input: &str) -> IResult<&str, (f64, NoteDiv)> {
    separated_pair(double, char('#'), length_note_div).parse(input)
}

fn event_duration(input: &str) -> IResult<&str, EventDuration> {
    alt((
        map(length_note_div, EventDuration::LengthNoteDiv),
        map(duration_secs, EventDuration::Seconds),
        map(bpm_with_note_div, EventDuration::BpmWithNoteDiv),
    ))
    .parse(input)
}

fn slide(input: &str) -> IResult<&str, SlideEvent> {
    let slide_pat = || {
        alt((
            value(SlidePattern::Line, char('-')),
            value(SlidePattern::ArcRight, char('>')),
            value(SlidePattern::ArcLeft, char('<')),
            value(SlidePattern::ArcShort, char('^')),
            value(SlidePattern::Thunder_s, char('s')),
            value(SlidePattern::Thunder_z, char('z')),
            value(SlidePattern::Fan, char('w')),
            value(SlidePattern::Grand_p, tag("pp")),
            value(SlidePattern::Grand_q, tag("qq")),
            value(SlidePattern::Shape_p, char('p')),
            value(SlidePattern::Shape_q, char('q')),
            pair(char('V'), cut(button)).map(|(_, v)| SlidePattern::Grand_v { middle: v }),
        ))
    };
    let slide_wait = || {
        alt((
            // Consumes ##
            terminated(
                double,
                pair(
                    tag("##"),
                    peek(alt((
                        value((), length_note_div),
                        value((), bpm_with_note_div),
                    ))),
                ),
            )
            .map(SlideWaitDuration::Seconds),
            // Doesn't consume last #
            terminated(double, pair(char('#'), peek(char('#')))).map(SlideWaitDuration::Seconds),
            // If the tracing length is in seconds, consume BPM
            terminated(double, peek((char('#'), double, char(']'))))
                .map(SlideWaitDuration::OneBeatAtBpm),
            peek(terminated(double, char('#'))).map(SlideWaitDuration::OneBeatAtBpm),
            success(SlideWaitDuration::OneBeatAtCurrentBpm),
        ))
    };
    let slide_len = || delimited(char('['), pair(slide_wait(), event_duration), char(']'));
    let slide_track_same_len = || {
        (many1((slide_pat(), button)), slide_len()).map(|(tracks, (wait, length))| {
            (
                tracks
                    .into_iter()
                    .map(|(variant, end)| SlideTrackData {
                        track: SlideTrack {
                            start: Button::A1, // Must fill later
                            end,
                            duration: 0., // Must fill later
                            variant,
                        },
                        length: length.clone(),
                    })
                    .collect::<Vec<_>>(),
                wait,
            )
        })
    };
    let slide_track_diff_len = || {
        many1((slide_pat(), button, slide_len())).map(|v| {
            let wait = v[0].2.0.clone();
            (
                v.into_iter()
                    .map(|(variant, end, (_, length))| SlideTrackData {
                        track: SlideTrack {
                            start: Button::A1, // Must fill later
                            end,
                            duration: 0., // Must fill later
                            variant,
                        },
                        length,
                    })
                    .collect::<Vec<_>>(),
                wait,
            )
        })
    };
    let slide_no_start_button = || {
        (
            alt((slide_track_diff_len(), slide_track_same_len())),
            break_,
        )
            .map(|((tracks, wait), break_slide)| SlideData {
                slide: Slide {
                    start: Button::A1, // Must fill later
                    break_note: false, // Must fill later
                    break_slide,
                    wait_duration: 0.,  // Must fill later
                    tracks: Vec::new(), // Must fill later
                },
                tracks,
                wait,
            })
    };
    let slide_multi = preceded(char('*'), slide_no_start_button());

    let (s, (start, break_note, slide, multi)) =
        (button, break_, slide_no_start_button(), many0(slide_multi)).parse(input)?;

    Ok((
        s,
        SlideEvent {
            start,
            slide,
            break_note,
            multi,
        },
    ))
}

#[derive(Clone)]
struct SlideData {
    slide: Slide,
    wait: SlideWaitDuration,
    tracks: Vec<SlideTrackData>,
}

#[derive(Clone)]
struct SlideTrackData {
    track: SlideTrack,
    length: EventDuration,
}

#[derive(Clone)]
struct SlideEvent {
    start: Button,
    slide: SlideData,
    break_note: bool,
    multi: Vec<SlideData>,
}

#[derive(Clone)]
enum Events {
    Tap(Tap),
    TapEach((Tap, Vec<Tap>)),
    Hold((Hold, EventDuration)),
    Slide(SlideEvent),
    Sep,
    Bpm(f64),
    ExplicitDuration(f64),
    LengthDivider(u64),
}

#[derive(Clone)]
enum EventDuration {
    LengthNoteDiv(NoteDiv),
    Seconds(f64),
    BpmWithNoteDiv((f64, NoteDiv)),
}

#[derive(Clone)]
enum SlideWaitDuration {
    Seconds(f64),
    OneBeatAtBpm(f64),
    OneBeatAtCurrentBpm,
}

fn simai_map(input: &str) -> IResult<&str, (u64, Map)> {
    let inote = file_field_start(preceded(tag("inote_"), u64));

    let tap = || (button, break_).map(|(button, break_)| Tap { button, break_ });
    // tap_each has no breaks
    let tap_each = || {
        (button, many1(button)).map(|(button, taps)| {
            (
                Tap {
                    button,
                    break_: false,
                },
                taps.into_iter()
                    .map(|button| Tap {
                        button,
                        break_: false,
                    })
                    .collect::<Vec<_>>(),
            )
        })
    };
    let hold = || {
        (
            button,
            char('h'),
            break_,
            delimited(char('['), event_duration, char(']')),
        )
            .map(|(button, _, break_, duration)| {
                (
                    Hold {
                        button,
                        break_,
                        length: 0.,
                    },
                    duration,
                )
            })
    };

    let sep = || char(',');
    let end = char('E');
    let bpm = || delimited(char('('), double, char(')'));
    let length_divider = || delimited(char('{'), u64, char('}'));

    // Start of map requirements
    let start_of_map = context(
        "start of the map",
        permutation((
            preceded(
                multispace0,
                alt((
                    map(bpm(), Events::Bpm),
                    map(duration_secs, Events::ExplicitDuration),
                )),
            ),
            map(
                preceded(multispace0, length_divider()),
                Events::LengthDivider,
            ),
        )),
    );

    let event = || {
        terminated(
            alt((
                map(hold(), Events::Hold),
                map(slide, Events::Slide),
                map(tap_each(), Events::TapEach),
                map(tap(), Events::Tap),
                value(Events::Sep, sep()),
                map(bpm(), Events::Bpm),
                map(duration_secs, Events::ExplicitDuration),
                map(length_divider(), Events::LengthDivider),
            )),
            multispace0,
        )
    };

    let (input, _) = multispace0(input)?;

    let (input, (inote, initial_events, _, (events, _))) = ((
        inote,
        cut(start_of_map),
        multispace0,
        many_till(pair(cut(event()), many0(preceded(char('/'), event()))), end),
    ))
        .parse(input)?;

    Ok((input, todo!()))
}

enum Field<'a> {
    Title(&'a str),
    Artist(&'a str),
    First(f64),
    WholeBpm(f64),
    Designer((u64, &'a str)),
    Level((u64, &'a str)),
    OtherFields((&'a str, &'a str)),
    INote((u64, Map)),
}

pub fn simai_file<'a>(input: &'a str) -> IResult<&'a str, SimaiFile<'a>, VerboseError<&'a str>> {
    let title = file_field_line(tag("title"));
    let artist = file_field_line(tag("artist"));
    let first = map_parser(file_field_line(tag("first")), double);
    let wholebpm = map_parser(file_field_line(tag("wholebpm")), double);
    let des = pair(
        file_field_start(preceded(
            tag("des"),
            alt((field_with_num, value(0, peek(char('='))))),
        )),
        not_line_ending,
    );
    let lv = pair(
        file_field_start(preceded(tag("lv"), field_with_num)),
        not_line_ending,
    );
    let other_fields = pair(file_field_start(take_till1(|c| c == '=')), not_line_ending);

    let (input, _) = multispace0(input)?;
    let (input, fields) = all_consuming(many0(terminated(
        alt((
            map(title, Field::Title),
            map(artist, Field::Artist),
            map(first, Field::First),
            map(wholebpm, Field::WholeBpm),
            map(des, Field::Designer),
            map(lv, Field::Level),
            map(simai_map, Field::INote),
            map(other_fields, Field::OtherFields),
        )),
        multispace0,
    )))
    .parse(input)?;

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
                if title.is_some() {
                    // return Err(Err::Failure(DuplicateFieldError("title").into()));
                    todo!()
                }
                title = Some(value)
            }
            Field::Artist(value) => artist = Some(value),
            Field::First(value) => first = Some(value),
            Field::WholeBpm(value) => wholebpm = Some(value),
            Field::Designer((num, value)) => {
                designers.insert(num, value);
            }
            Field::Level((num, value)) => {
                level_names.insert(num, value);
            }
            Field::INote((num, value)) => {
                maps.insert(num, value);
            }
            Field::OtherFields((name, value)) => {
                other_fields.insert(name, value);
            }
        }
    }

    Ok((
        input,
        SimaiFile {
            title,
            artist,
            designers,
            first: first.unwrap_or_default(),
            level_names,
            wholebpm,
            other_fields,
            maps,
        },
    ))
}

#[cfg(test)]
mod tests {
    use nom::{Parser, bytes::complete::tag};

    use crate::parser::*;

    #[test]
    fn file_field() {
        let (s, value) = file_field_start(tag::<_, _, ()>("title"))
            .parse("&title=some title 123")
            .unwrap();
        assert_eq!(value, "title");
        assert_eq!(s, "some title 123");

        let (s, value) = file_field_line(tag::<_, _, ()>("title"))
            .parse("&title=some title 123")
            .unwrap();
        assert_eq!(value, "some title 123");
        assert!(s.is_empty());

        let (s, (title, _, artist)) = (
            file_field_line(tag::<_, _, ()>("title")),
            multispace0,
            file_field_line(tag("artist")),
        )
            .parse("&title=some title 123\n&artist=some artist")
            .unwrap();
        assert_eq!(title, "some title 123");
        assert_eq!(artist, "some artist");
        assert!(s.is_empty());

        let (s, num) = field_with_num("_123=foo").unwrap();
        assert_eq!(s, "=foo");
        assert_eq!(num, 123);
    }

    #[test]
    fn fields() {
        let file = "&title=some title 123";
        let (s, file) = simai_file(file).unwrap();

        assert!(s.is_empty());
        assert_eq!(file.title, Some("some title 123"));

        let file = "&lv_1=lv 1\n&lv_2=lv 2";
        let (s, file) = simai_file(file).unwrap();
        assert!(s.is_empty());
        assert_eq!(file.level_names.len(), 2);
        assert_eq!(file.level_names.get(&1), Some(&"lv 1"));
        assert_eq!(file.level_names.get(&2), Some(&"lv 2"));

        let file = "&des=des 0\n&des_1=des 1";
        let (s, file) = simai_file(file).unwrap();
        assert!(s.is_empty());
        assert_eq!(file.designers.len(), 2);
        assert_eq!(file.designers.get(&0), Some(&"des 0"));
        assert_eq!(file.designers.get(&1), Some(&"des 1"));

        let file = r#"&title=some title 123
&artist=some artist
&first=-1.337
&des=des 0
&des_1=des 1
&wholebpm=120.5
&lv_2=lv 2
&lv_1=lv 1"#;

        let (s, file) = simai_file(file).unwrap();
        assert!(s.is_empty());
        assert_eq!(file.title, Some("some title 123"));
        assert_eq!(file.artist, Some("some artist"));
        assert_eq!(file.first, -1.337);
        assert_eq!(file.wholebpm, Some(120.5));
        assert_eq!(file.designers.len(), 2);
        assert_eq!(file.designers.get(&0), Some(&"des 0"));
        assert_eq!(file.designers.get(&1), Some(&"des 1"));
        assert_eq!(file.level_names.len(), 2);
        assert_eq!(file.level_names.get(&1), Some(&"lv 1"));
        assert_eq!(file.level_names.get(&2), Some(&"lv 2"));

        let (s, file) = simai_file("").unwrap();
        assert!(s.is_empty());
        assert_eq!(file.first, 0.);
    }

    #[test]
    fn slide_min() {
        let (_, res) = slide("1-4[1:2]").unwrap();
        assert_eq!(res.start, Button::A1);
        assert!(!res.break_note);
        assert!(res.multi.is_empty());
        assert_eq!(
            res.slide.slide,
            Slide {
                start: Button::A1,
                break_note: false,
                break_slide: false,
                wait_duration: 0.,
                tracks: Vec::new()
            }
        );
        assert!(matches!(
            res.slide.wait,
            SlideWaitDuration::OneBeatAtCurrentBpm
        ));
        assert_eq!(res.slide.tracks.len(), 1);
        let track = &res.slide.tracks[0];
        assert_eq!(
            track.track,
            SlideTrack {
                start: Button::A1,
                end: Button::A4,
                duration: 0.,
                variant: SlidePattern::Line
            }
        );
        assert!(
            matches!(track.length, EventDuration::LengthNoteDiv(div) if div.0 == 1 && div.1 == 2)
        );
    }

    #[test]
    fn slide_durations() {
        let (_, res) = slide("1-4[160#8:3]").unwrap();
        assert!(matches!(
            res.slide.wait,
            SlideWaitDuration::OneBeatAtBpm(bpm) if bpm == 160.
        ));
        assert!(
            matches!(res.slide.tracks[0].length, EventDuration::BpmWithNoteDiv((bpm, div)) if bpm == 160. && div.0 == 8 && div.1 == 3)
        );

        let (_, res) = slide("1-4[160#2]").unwrap();
        assert!(matches!(
            res.slide.wait,
            SlideWaitDuration::OneBeatAtBpm(bpm) if bpm == 160.
        ));
        assert!(matches!(res.slide.tracks[0].length, EventDuration::Seconds(secs) if secs == 2.));

        let (_, res) = slide("1-4[3##1.5]").unwrap();
        assert!(matches!(
            res.slide.wait,
            SlideWaitDuration::Seconds(secs) if secs == 3.
        ));
        assert!(matches!(res.slide.tracks[0].length, EventDuration::Seconds(secs) if secs == 1.5));

        let (_, res) = slide("1-4[3##8:3]").unwrap();
        assert!(matches!(
            res.slide.wait,
            SlideWaitDuration::Seconds(secs) if secs == 3.
        ));
        assert!(
            matches!(res.slide.tracks[0].length, EventDuration::LengthNoteDiv(div) if div.0 == 8 && div.1 == 3)
        );

        let (_, res) = slide("1-4[3##160#8:3]").unwrap();
        assert!(matches!(
            res.slide.wait,
            SlideWaitDuration::Seconds(secs) if secs == 3.
        ));
        assert!(
            matches!(res.slide.tracks[0].length, EventDuration::BpmWithNoteDiv((bpm, div)) if bpm == 160. && div.0 == 8 && div.1 == 3)
        );
    }
}
