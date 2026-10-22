use std::{env, fs};

use simai_parser::*;

fn main() {
    let mut args = env::args_os();

    args.next();

    for arg in args {
        println!("Processing file `{}`", arg.to_string_lossy());

        let file = match fs::read_to_string(arg) {
            Ok(ok) => ok,
            Err(err) => {
                println!("Couldn't read file: {err}\n");
                continue;
            }
        };

        let file = match SimaiFile::parse(&file) {
            Ok(ok) => ok,
            Err(err) => {
                println!("Couldn't parse as a Simai file, {err}\n");
                continue;
            }
        };

        let SimaiFile {
            title,
            artist,
            designers,
            first,
            level_names,
            wholebpm,
            other_fields,
            maps,
        } = file;

        println!("title: {title:?}");
        println!("artist: {artist:?}");
        println!("designers: {designers:?}");
        println!("first: {first:?}");
        println!("wholebpm: {wholebpm:?}");
        println!("other_fields: {other_fields:?}");
        println!();

        for (id, map) in maps {
            println!(
                "map id {id}, name: {}",
                level_names.get(&id).unwrap_or(&"UNKNOWN")
            );
            println!("-----");

            let Map { events } = map;

            let mut prev_time = f64::NAN;

            for event in events {
                let Event { time, event_type } = event;

                if time == prev_time {
                    print!("         ");
                } else {
                    print!("[{time:.4}] ");
                }

                prev_time = time;

                match event_type {
                    EventType::Tap(Tap { data, star_shaped }) => {
                        let TapData {
                            position,
                            break_,
                            ex,
                        } = data;
                        println!(
                            "Tap, position: {position}, break: {break_}, ex: {ex}, star shaped: {star_shaped:?}"
                        );
                    }
                    EventType::Hold(Hold {
                        position,
                        break_,
                        ex,
                        duration,
                    }) => println!(
                        "Hold, position: {position}, break: {break_}, ex: {ex}, duration: {duration:.4}s"
                    ),
                    EventType::Slide(Slide {
                        tap,
                        wait_duration,
                        break_slide,
                        tracks,
                        total_track_duration,
                    }) => {
                        print!("Slide, ");
                        match &tap {
                            SlideTap::StarTap(tap_data) | SlideTap::NormalTap(tap_data) => {
                                match tap {
                                    SlideTap::StarTap(_) => print!("star tap, "),
                                    SlideTap::NormalTap(_) => print!("normal tap, "),
                                    SlideTap::NoStarTapFadeIn | SlideTap::NoStarTapInstant => {
                                        unreachable!()
                                    }
                                }

                                let TapData {
                                    position,
                                    break_,
                                    ex,
                                } = tap_data;

                                print!("tap position: {position}, tap break: {break_}, ex: {ex}, ");
                            }
                            SlideTap::NoStarTapFadeIn => {
                                print!("no star tap, fade in trace star, ")
                            }
                            SlideTap::NoStarTapInstant => {
                                print!("no star tap, instant trace path, ")
                            }
                        }

                        print!("wait: {wait_duration:.4}s, break slide: {break_slide}, ");

                        print!("tracks ({total_track_duration:.4}s): [");
                        let mut first = true;
                        for track in tracks {
                            let SlideTrack {
                                start,
                                end,
                                duration,
                                variant,
                            } = track;

                            if !first {
                                print!(", ")
                            }

                            match variant {
                                SlidePattern::Line => print!("line"),
                                SlidePattern::ArcRight => print!("arc right"),
                                SlidePattern::ArcLeft => print!("arc left"),
                                SlidePattern::ArcShort => print!("arc short"),
                                SlidePattern::Shape_v => print!("shape v"),
                                SlidePattern::Shape_p => print!("shape p"),
                                SlidePattern::Shape_q => print!("shape q"),
                                SlidePattern::Thunder_s => print!("thunder s"),
                                SlidePattern::Thunder_z => print!("thunder z"),
                                SlidePattern::Grand_p => print!("grand p"),
                                SlidePattern::Grand_q => print!("grand q"),
                                SlidePattern::Grand_v { middle } => print!("grand v ({middle})"),
                                SlidePattern::Fan => print!("fan"),
                            }

                            print!(" {start} -> {end} ({duration:.4}s)");

                            first = false;
                        }
                        println!("]");
                    }
                    EventType::Touch(Touch { position, firework }) => {
                        println!("Touch, position: {position}, firework: {firework}")
                    }
                    EventType::TouchHold(TouchHold {
                        position,
                        duration,
                        firework,
                    }) => println!(
                        "Touch hold, position: {position}, firework: {firework}, duration: {duration:.4}s"
                    ),
                }
            }
        }

        println!();
    }
}
