use std::str::FromStr;

use nom::{
    Err, Parser,
    branch::{alt, permutation},
    bytes::complete::{tag, take, take_till1},
    character::complete::{char, multispace0, not_line_ending, u64, usize},
    combinator::{
        all_consuming, cut, map, map_opt, map_parser, map_res, not, opt, peek, success, value,
        verify,
    },
    error::{ParseError, context},
    multi::{many_till, many0, many1},
    number::complete::double,
    sequence::{delimited, pair, preceded, separated_pair, terminated},
};
use nom_language::error::VerboseError;

use crate::{
    Button, Hold, Sensor, Slide, SlidePattern, SlideTrack, SpecialSlideTap, StarShapedTap, Tap,
    Touch, TouchHold,
};

type IResult<I, O, E = VerboseError<I>> = Result<(I, O), Err<E>>;

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
    context(
        "button",
        map_opt(map_parser(take(1usize), usize), Button::from_repr),
    )
    .parse(input)
}

fn ex(input: &str) -> IResult<&str, bool> {
    let (s, res) = opt(char('x')).parse(input)?;
    Ok((s, res.is_some()))
}

fn sensor(input: &str) -> IResult<&str, Sensor> {
    context(
        "sensor",
        map_res(alt((tag("C"), take(2usize))), Sensor::from_str),
    )
    .parse(input)
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
        context(
            "slide_pat",
            alt((
                value(SlidePattern::Line, char('-')),
                value(SlidePattern::ArcRight, char('>')),
                value(SlidePattern::ArcLeft, char('<')),
                value(SlidePattern::ArcShort, char('^')),
                value(SlidePattern::Shape_v, char('v')),
                value(SlidePattern::Thunder_s, char('s')),
                value(SlidePattern::Thunder_z, char('z')),
                value(SlidePattern::Fan, char('w')),
                value(SlidePattern::Grand_p, tag("pp")),
                value(SlidePattern::Grand_q, tag("qq")),
                value(SlidePattern::Shape_p, char('p')),
                value(SlidePattern::Shape_q, char('q')),
                pair(char('V'), cut(button)).map(|(_, v)| SlidePattern::Grand_v { middle: v }),
            )),
        )
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
    let slide_len = || {
        context(
            "slide_len",
            delimited(char('['), pair(slide_wait(), event_duration), char(']')),
        )
    };
    let slide_track_same_len = || {
        context(
            "slide_track_same_len",
            (many1((slide_pat(), button)), pair(break_, slide_len())),
        )
        .map(|(tracks, (break_slide, (wait, length)))| {
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
                        break_slide,
                    })
                    .collect::<Vec<_>>(),
                wait,
            )
        })
    };
    let slide_track_diff_len = || {
        context(
            "slide_track_diff_len",
            verify(
                many1((slide_pat(), button, break_, slide_len())),
                |v: &Vec<_>| v.iter().rev().skip(1).all(|v| !v.2),
            ),
        )
        .map(|v| {
            let wait = v[0].3.0.clone();
            let break_slide = v.last().unwrap().2;
            (
                v.into_iter()
                    .map(|(variant, end, _, (_, length))| SlideTrackData {
                        track: SlideTrack {
                            start: Button::A1, // Must fill later
                            end,
                            duration: 0., // Must fill later
                            variant,
                        },
                        length,
                        break_slide,
                    })
                    .collect::<Vec<_>>(),
                wait,
            )
        })
    };
    let slide_no_start_button = || {
        (alt((slide_track_diff_len(), slide_track_same_len()))).map(|(tracks, wait)| SlideData {
            slide: Slide {
                start: Button::A1, // Must fill later
                break_tap: false,  // Must fill later
                break_slide: tracks[0].break_slide,
                wait_duration: 0.,  // Must fill later
                tracks: Vec::new(), // Must fill later
                special_tap: None,  // Filled in down below
                ex_tap: false,      // Filled in down below
            },
            tracks,
            wait,
        })
    };
    let special_tap = opt(alt((
        value(SpecialSlideTap::NormalTap, char('@')),
        value(SpecialSlideTap::NoStarTapFadeIn, char('?')),
        value(SpecialSlideTap::NoStarTapInstant, char('!')),
    )));
    let slide_multi = preceded(char('*'), slide_no_start_button());

    let (s, (start, (ex, break_note, special_tap), mut slide, multi)) = (
        button,
        permutation((ex, break_, special_tap)),
        slide_no_start_button(),
        many0(slide_multi),
    )
        .parse(input)?;

    slide.slide.ex_tap = ex;
    slide.slide.special_tap = special_tap;

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
pub struct SlideTrackData {
    track: SlideTrack,
    length: EventDuration,
    break_slide: bool,
}

#[derive(Clone)]
pub struct SlideEvent {
    start: Button,
    slide: SlideData,
    break_note: bool,
    multi: Vec<SlideData>,
}

#[derive(Clone)]
pub struct TapEachData {
    pub psuedo_each: bool,
    pub position: Button,
}

#[derive(Clone)]
pub enum Event {
    Tap(Tap),
    TapEach((Tap, Vec<TapEachData>)),
    Hold((Hold, Option<EventDuration>)),
    Slide(SlideEvent),
    Touch(Touch),
    TouchHold((TouchHold, Option<EventDuration>)),
    Sep,
    Bpm(f64),
    ExplicitDuration(f64),
    LengthDivider(f64),
}

#[derive(Clone)]
pub enum EventDuration {
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

fn firework(input: &str) -> IResult<&str, bool> {
    let (s, res) = opt(char('f')).parse(input)?;

    Ok((s, res.is_some()))
}

fn touch(input: &str) -> IResult<&str, Touch> {
    let (s, (position, firework)) = pair(sensor, firework).parse(input)?;

    Ok((s, Touch { position, firework }))
}

fn touch_hold(input: &str) -> IResult<&str, (TouchHold, Option<EventDuration>)> {
    let (s, (position, (_, firework), duration)) = (
        sensor,
        permutation((char('h'), firework)),
        opt(delimited(char('['), event_duration, char(']'))),
    )
        .parse(input)?;

    Ok((
        s,
        (
            TouchHold {
                position,
                duration: 0.,
                firework,
            },
            duration,
        ),
    ))
}

fn tap(input: &str) -> IResult<&str, Tap> {
    let star_shaped_tap = {
        opt(alt((
            value(StarShapedTap::Rotating, tag("$$")),
            value(StarShapedTap::NonRotating, char('$')),
        )))
    };

    let (s, (button, (break_, ex, star_shaped))) =
        (button, permutation((break_, ex, star_shaped_tap))).parse(input)?;

    Ok((
        s,
        Tap {
            position: button,
            break_,
            ex,
            star_shaped,
        },
    ))
}

fn tap_each(input: &str) -> IResult<&str, (Tap, Vec<TapEachData>)> {
    let pseudo_each = opt(char('`')).map(|v| v.is_some());

    // tap_each has no breaks
    let (s, (button, taps)) = (button, many1((pseudo_each, button))).parse(input)?;

    Ok((
        s,
        (
            Tap {
                position: button,
                break_: false,
                ex: false,
                star_shaped: None,
            },
            taps.into_iter()
                .map(|(psuedo_each, position)| TapEachData {
                    position,
                    psuedo_each,
                })
                .collect::<Vec<_>>(),
        ),
    ))
}

fn hold(input: &str) -> IResult<&str, (Hold, Option<EventDuration>)> {
    let (s, (button, (_, break_, ex), duration)) = (
        button,
        permutation((char('h'), break_, ex)),
        opt(delimited(char('['), event_duration, char(']'))),
    )
        .parse(input)?;

    Ok((
        s,
        (
            Hold {
                position: button,
                break_,
                duration: 0., // Must fill later
                ex,
            },
            duration,
        ),
    ))
}

fn simai_map(input: &str) -> IResult<&str, (u64, impl Iterator<Item = Event>)> {
    let inote = file_field_start(preceded(tag("inote_"), u64));

    let sep = || char(',');
    let end = char('E');
    let bpm = || delimited(char('('), double, char(')'));
    let length_divider = || delimited(char('{'), double, char('}'));

    // Start of map requirements
    let start_of_map = context(
        "start of the map",
        permutation((
            preceded(
                multispace0,
                alt((
                    map(bpm(), Event::Bpm),
                    map(duration_secs, Event::ExplicitDuration),
                )),
            ),
            map(
                preceded(multispace0, length_divider()),
                Event::LengthDivider,
            ),
        )),
    );

    let event = || {
        context(
            "event",
            cut(terminated(
                alt((
                    map(hold, Event::Hold),
                    map(slide, Event::Slide),
                    map(tap_each, Event::TapEach),
                    map(tap, Event::Tap),
                    map(touch_hold, Event::TouchHold),
                    map(touch, Event::Touch),
                    value(Event::Sep, sep()),
                    map(bpm(), Event::Bpm),
                    map(duration_secs, Event::ExplicitDuration),
                    map(length_divider(), Event::LengthDivider),
                )),
                multispace0,
            )),
        )
    };

    let (input, _) = multispace0(input)?;

    let (input, (inote, initial_events, _, (events, _))) = (
        inote,
        cut(start_of_map),
        multispace0,
        many_till(
            pair(event(), many0(preceded(char('/'), event()))),
            pair(not(sensor), end),
        ),
    )
        .parse(input)?;

    let (e1, e2) = initial_events;

    let initial_events = [e1, e2];

    let events = initial_events.into_iter().chain(
        events
            .into_iter()
            .flat_map(|(e1, e2)| [e1].into_iter().chain(e2)),
    );

    Ok((input, (inote, events)))
}

pub enum Field<'a, I: Iterator<Item = Event>> {
    Title(&'a str),
    Artist(&'a str),
    First(f64),
    WholeBpm(f64),
    Designer((u64, &'a str)),
    Level((u64, &'a str)),
    OtherFields((&'a str, &'a str)),
    INote((u64, I)),
}

pub fn simai_file<'a>(
    input: &'a str,
) -> IResult<&'a str, Vec<Field<'a, impl Iterator<Item = Event>>>, VerboseError<&'a str>> {
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
    context(
        "event parser",
        all_consuming(many0(terminated(
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
        ))),
    )
    .parse(input)
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
        let (s, fields) = simai_file(file).unwrap();

        assert!(s.is_empty());
        assert_eq!(fields.len(), 1);
        assert!(matches!(fields[0], Field::Title("some title 123")));

        let file = "&lv_1=lv 1\n&lv_2=lv 2";
        let (s, fields) = simai_file(file).unwrap();
        assert!(s.is_empty());
        assert_eq!(fields.len(), 2);
        assert!(matches!(fields[0], Field::Level((1, "lv 1"))));
        assert!(matches!(fields[1], Field::Level((2, "lv 2"))));

        let file = "&des=des 0\n&des_1=des 1";
        let (s, fields) = simai_file(file).unwrap();
        assert!(s.is_empty());
        assert_eq!(fields.len(), 2);
        assert!(matches!(fields[0], Field::Designer((0, "des 0"))));
        assert!(matches!(fields[1], Field::Designer((1, "des 1"))));

        let file = r#"&title=some title 123
&artist=some artist
&first=-1.337
&des=des 0
&des_1=des 1
&wholebpm=120.5
&lv_2=lv 2
&lv_1=lv 1"#;

        let (s, fields) = simai_file(file).unwrap();
        assert!(s.is_empty());
        assert_eq!(fields.len(), 8);
        assert!(matches!(fields[0], Field::Title("some title 123")));
        assert!(matches!(fields[1], Field::Artist("some artist")));
        assert!(matches!(fields[2], Field::First(-1.337)));
        assert!(matches!(fields[3], Field::Designer((0, "des 0"))));
        assert!(matches!(fields[4], Field::Designer((1, "des 1"))));
        assert!(matches!(fields[5], Field::WholeBpm(120.5)));
        assert!(matches!(fields[6], Field::Level((2, "lv 2"))));
        assert!(matches!(fields[7], Field::Level((1, "lv 1"))));

        let (s, fields) = simai_file("").unwrap();
        assert!(s.is_empty());
        assert!(fields.is_empty());
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
                break_tap: false,
                break_slide: false,
                wait_duration: 0.,
                tracks: Vec::new(),
                special_tap: None,
                ex_tap: false
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

    #[test]
    fn slide_chain() {
        let (_, res) = slide("2-4v7^4[8:3]").unwrap();
        assert_eq!(res.slide.tracks.len(), 3);
        assert!(
            matches!(res.slide.tracks[0].length, EventDuration::LengthNoteDiv(div) if div.0 == 8 && div.1 ==3)
        );
        assert_eq!(
            res.slide.tracks[0].track,
            SlideTrack {
                start: Button::A1,
                end: Button::A4,
                duration: 0.,
                variant: SlidePattern::Line
            }
        );
        assert!(
            matches!(res.slide.tracks[1].length, EventDuration::LengthNoteDiv(div) if div.0 == 8 && div.1 ==3)
        );
        assert_eq!(
            res.slide.tracks[1].track,
            SlideTrack {
                start: Button::A1,
                end: Button::A7,
                duration: 0.,
                variant: SlidePattern::Shape_v
            }
        );
        assert!(
            matches!(res.slide.tracks[2].length, EventDuration::LengthNoteDiv(div) if div.0 == 8 && div.1 ==3)
        );
        assert_eq!(
            res.slide.tracks[2].track,
            SlideTrack {
                start: Button::A1,
                end: Button::A4,
                duration: 0.,
                variant: SlidePattern::ArcShort
            }
        );
    }

    #[test]
    fn touch_test() {
        let (s, touch) = touch("E1,A2").unwrap();
        assert_eq!(s, ",A2");
        assert_eq!(
            touch,
            Touch {
                position: Sensor::E1,
                firework: false
            }
        )
    }
}
