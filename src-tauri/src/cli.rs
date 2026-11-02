use std::{
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use serde_json::Value;

use crate::app::AppState;

fn data_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return PathBuf::from(appdata).join("com.valotrack.desktop");
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".valotrack")
}

fn number(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_f64)
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "-".to_string())
}

fn percent(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_f64)
        .map(|value| format!("{value:.0}"))
        .unwrap_or_else(|| "-".to_string())
}

fn ansi(hex: &str) -> String {
    let clean = hex.trim_start_matches('#');
    if clean.len() < 6 {
        return "\x1b[0m".to_string();
    }
    let red = u8::from_str_radix(&clean[0..2], 16).unwrap_or(255);
    let green = u8::from_str_radix(&clean[2..4], 16).unwrap_or(255);
    let blue = u8::from_str_radix(&clean[4..6], 16).unwrap_or(255);
    format!("\x1b[38;2;{red};{green};{blue}m")
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn color(value: &Value, key: &str, fallback: &str) -> String {
    let raw = text(value, key);
    if raw.is_empty() {
        fallback.to_string()
    } else {
        raw.to_string()
    }
}

fn render(board: &Value) {
    let state = text(board, "stateLabel");
    let map = text(board, "map");
    let mode = text(board, "mode");
    println!("\x1b[1m\x1b[31mVALOTRACK\x1b[0m  {state}  {map}  {mode}");
    if let Some(score) = board.get("score") {
        if !score.is_null() {
            println!(
                "score {} : {}   round {}",
                score.get("ally").and_then(Value::as_i64).unwrap_or(0),
                score.get("enemy").and_then(Value::as_i64).unwrap_or(0),
                score.get("round").and_then(Value::as_i64).unwrap_or(0)
            );
        }
    }
    if let Some(win) = board.get("winProb").and_then(Value::as_i64) {
        println!("win probability {win}%");
    }

    let Some(teams) = board.get("teams").and_then(Value::as_object) else {
        println!("No lobby data. Open VALORANT and reach agent select or a match.");
        return;
    };

    for (team, players) in teams {
        let Some(players) = players.as_array() else {
            continue;
        };
        println!("\n\x1b[1m{team}\x1b[0m");
        println!(
            "{:<22} {:<12} {:<14} {:>5} {:>5} {:>5} {:>5} {:>4}",
            "PLAYER", "AGENT", "RANK", "RR", "K/D", "HS%", "WR%", "LVL"
        );
        for player in players {
            let rank = text(player, "rank");
            let rr = player.get("rr").and_then(Value::as_i64).unwrap_or(0);
            let level = player.get("level").and_then(Value::as_i64).unwrap_or(0);
            let rank_color = ansi(&color(player, "rankColor", ""));
            println!(
                "{:<22} {:<12} {}{:<14}\x1b[0m {:>5} {:>5} {:>5} {:>5} {:>4}",
                text(player, "name"),
                text(player, "agent"),
                rank_color,
                rank,
                rr,
                number(player, "kd"),
                percent(player, "hsPct"),
                percent(player, "winRate"),
                level
            );
        }
    }
}

pub fn run() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("valotrack-cli [--once] [--interval <seconds>]");
        println!("Prints the live VALORANT scoreboard from the local Riot client.");
        return;
    }
    let once = args.iter().any(|arg| arg == "--once");
    let interval = args
        .iter()
        .position(|arg| arg == "--interval")
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(5)
        .max(1);

    let state = match AppState::new_headless(data_dir()) {
        Ok(state) => Arc::new(state),
        Err(error) => {
            eprintln!("Valotrack could not start: {error}");
            return;
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("Valotrack could not start its runtime: {error}");
            return;
        }
    };

    loop {
        if !once {
            print!("\x1b[2J\x1b[H");
        }
        match runtime.block_on(crate::live::board(&state)) {
            Ok(board) => render(&board),
            Err(error) => eprintln!("\x1b[31m{error}\x1b[0m"),
        }
        if once {
            break;
        }
        std::thread::sleep(Duration::from_secs(interval));
    }
}
