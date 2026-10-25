//! Application requests selected by the original game callback `0x4d76b0`.
//! Media playback and menu rendering belong to the application and are not implemented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    PlayMovie { path: &'static str, mode: u32 },
    Menu { mode: u32 },
}

/// Event 12 at `0x4d806c`: preserve the original movie name and playback mode.
pub fn victory(is_human: bool, flags: u32) -> Action {
    if !is_human {
        Action::Menu { mode: 1 }
    } else if flags & 0x10000 != 0 {
        Action::PlayMovie { path: "\\trbn.bne", mode: 4 }
    } else {
        Action::PlayMovie { path: "\\trw1.bne", mode: 3 }
    }
}

/// `0x528e4e..0x528eaa`: completion destinations used by victory movies and intro.
/// Other movie modes perform different engine operations and are not mapped here.
pub fn movie_menu(mode: u32) -> Option<u32> {
    match mode { 2 => Some(0), 3 => Some(2), 4 => Some(5), _ => None }
}

/// `0x529020`: WM_CHAR is consumed; Enter in mode 1 alone does not finish the movie.
pub fn movie_character(mode: u32, message: u16, character: u16) -> (bool, bool) {
    if message != 0x102 { (false, false) }
    else { (true, character != 13 || mode != 1) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn movie_controls_match_original() {
        for line in include_str!("../tests/data/movie_character_golden.txt").lines() {
            let w: Vec<u32> = line.split_whitespace().map(|s| s.parse().unwrap()).collect();
            assert_eq!(movie_character(w[0], w[1] as u16, w[2] as u16), (w[3] != 0, w[4] != 0));
        }
        for line in include_str!("../tests/data/movie_completion_golden.txt").lines() {
            let w: Vec<u32> = line.split_whitespace().map(|s| s.parse().unwrap()).collect();
            assert_eq!(movie_menu(w[0]), Some(w[1]));
        }
    }
    #[test]
    fn victory_requests_match_game_callback() {
        for line in include_str!("../tests/data/victory_action_golden.txt").lines() {
            let w: Vec<_> = line.split_whitespace().collect();
            let action = victory(w[0] == "1", w[1].parse().unwrap());
            let mode = w[3].parse().unwrap();
            let expected = match w[2] {
                "menu" => Action::Menu { mode },
                "\\trbn.bne" => Action::PlayMovie { path: "\\trbn.bne", mode },
                "\\trw1.bne" => Action::PlayMovie { path: "\\trw1.bne", mode },
                other => panic!("unknown original action {other}"),
            };
            assert_eq!(action, expected);
            assert_eq!(w[4], "4294967295");
        }
    }
}
