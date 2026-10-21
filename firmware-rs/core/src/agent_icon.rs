//! The small icon a task card shows for its agent, in place of the text prefix the daemon puts in its title
//! (#155). The agents' icons are pixel versions of their makers' marks in their colours; CI isn't an agent, so its
//! icon is our own. An agent without an icon keeps its prefix.

use crate::canvas::Canvas;

/// Icons are this many pixels square.
pub const SIZE: i32 = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentIcon {
    ClaudeCode,
    Codex,
    OpenCode,
    Copilot,
    Pi,
    /// GitHub Actions runs.
    Ci,
}

const WHITE: u16 = 0xFFFF;
const GRAY: u16 = 0x8410;
/// Claude's orange, #D97757.
const CLAUDE: u16 = 0xDBAA;
/// Copilot's purple, #8957E5.
const COPILOT: u16 = 0x8ABC;
/// A teal of our own for CI, #2DD4BF.
const CI: u16 = 0x2EB7;

impl AgentIcon {
    /// The icon for an agent as the daemon names it (`agent` in `tasks`).
    pub fn for_agent(agent: &str) -> Option<Self> {
        Some(match agent {
            "claude-code" => Self::ClaudeCode,
            "codex" => Self::Codex,
            "opencode" => Self::OpenCode,
            "copilot" => Self::Copilot,
            "pi" => Self::Pi,
            "github-actions" => Self::Ci,
            _ => return None,
        })
    }

    /// The prefix the daemon gives this agent's titles, which the icon replaces.
    pub fn prefix(self) -> &'static [u8] {
        match self {
            Self::ClaudeCode => b"CC:",
            Self::Codex => b"CX:",
            Self::OpenCode => b"OC:",
            Self::Copilot => b"CP:",
            Self::Pi => b"PI:",
            Self::Ci => b"CI:",
        }
    }

    /// Rows of the icon, a character a pixel: `#` the main colour, `+` the second, anything else left alone.
    fn pixels(self) -> (&'static [&'static str; 12], u16, u16) {
        match self {
            // Claude's spark: rays of uneven length from a centre.
            Self::ClaudeCode => (
                &[
                    ".....#......",
                    "..#..#...#..",
                    "...#.#..#...",
                    "....###.#...",
                    "#...####....",
                    ".#########..",
                    "...#####.###",
                    "..#.###.....",
                    ".#..#.##....",
                    "#..#..#.#...",
                    "...#..#..#..",
                    "......#.....",
                ],
                CLAUDE,
                CLAUDE,
            ),
            // Codex: the prompt in a rounded square.
            Self::Codex => (
                &[
                    "..########..",
                    ".#........#.",
                    "#..........#",
                    "#.#........#",
                    "#..#.......#",
                    "#...#......#",
                    "#..#.......#",
                    "#.#..++++..#",
                    "#..........#",
                    "#..........#",
                    ".#........#.",
                    "..########..",
                ],
                WHITE,
                WHITE,
            ),
            // OpenCode: its square mark, light above and dark below.
            Self::OpenCode => (
                &[
                    "############",
                    "############",
                    "##........##",
                    "##........##",
                    "##........##",
                    "##++++++++##",
                    "##++++++++##",
                    "##++++++++##",
                    "##++++++++##",
                    "##........##",
                    "############",
                    "############",
                ],
                WHITE,
                GRAY,
            ),
            // Copilot: the pilot's head with its goggles.
            Self::Copilot => (
                &[
                    "...######...",
                    "..#......#..",
                    ".##########.",
                    "##++##++####",
                    "##++##++####",
                    ".##########.",
                    "#..........#",
                    "#...#..#...#",
                    "#...#..#...#",
                    "#..........#",
                    ".#........#.",
                    "..########..",
                ],
                COPILOT,
                WHITE,
            ),
            // Pi: the letter.
            Self::Pi => (
                &[
                    "............",
                    "............",
                    ".##########.",
                    ".##########.",
                    "...##..##...",
                    "...##..##...",
                    "...##..##...",
                    "...##..##...",
                    "..##...##...",
                    "..##...###..",
                    ".##.....##..",
                    "............",
                ],
                WHITE,
                WHITE,
            ),
            // CI: two arrows chasing each other round a loop, our own.
            Self::Ci => (
                &[
                    "....####....",
                    "..##....#...",
                    ".#.......#..",
                    ".#......###.",
                    "#........#..",
                    "#...........",
                    "...........#",
                    "..#........#",
                    ".###......#.",
                    "..#.......#.",
                    "...#....##..",
                    "....####....",
                ],
                CI,
                CI,
            ),
        }
    }

    pub fn draw(self, canvas: &mut Canvas, x: i32, y: i32) {
        let (rows, main, second) = self.pixels();
        for (row, line) in rows.iter().enumerate() {
            for (column, pixel) in line.bytes().enumerate() {
                let color = match pixel {
                    b'#' => main,
                    b'+' => second,
                    _ => continue,
                };
                canvas.fill_rect(x + column as i32, y + row as i32, 1, 1, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_square() {
        for icon in [AgentIcon::ClaudeCode, AgentIcon::Codex, AgentIcon::OpenCode, AgentIcon::Copilot, AgentIcon::Pi, AgentIcon::Ci] {
            let (rows, _, _) = icon.pixels();
            assert!(rows.iter().all(|row| row.len() == SIZE as usize), "{icon:?}");
        }
    }

    #[test]
    fn agents_map_to_their_icons_and_prefixes() {
        assert_eq!(AgentIcon::for_agent("claude-code").map(AgentIcon::prefix), Some(&b"CC:"[..]));
        assert_eq!(AgentIcon::for_agent("github-actions"), Some(AgentIcon::Ci));
        assert_eq!(AgentIcon::for_agent("unknown"), None);
    }
}
