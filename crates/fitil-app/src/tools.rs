//! The drawing tools of the left bar, grouped as in TradingView: a group's button opens a
//! menu of its tools, in sections. Tools in no group have a button of their own.

use fitil_core::DrawingKind;

/// Tools that share a button of the left bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolGroup {
    /// Lines, channels and pitchforks.
    Lines,
}

/// Part of a group's menu, under a heading.
pub struct Section {
    /// The Fluent key of the heading.
    pub key: &'static str,
    pub tools: &'static [DrawingKind],
}

static LINES: [Section; 3] = [
    Section {
        key: "drawing-section-lines",
        tools: &[
            DrawingKind::TrendLine,
            DrawingKind::Ray,
            DrawingKind::InfoLine,
            DrawingKind::ExtendedLine,
            DrawingKind::TrendAngle,
            DrawingKind::HorizontalLine,
            DrawingKind::HorizontalRay,
            DrawingKind::VerticalLine,
            DrawingKind::CrossLine,
        ],
    },
    Section {
        key: "drawing-section-channels",
        tools: &[
            DrawingKind::ParallelChannel,
            DrawingKind::RegressionTrend,
            DrawingKind::FlatTopBottom,
            DrawingKind::DisjointChannel,
        ],
    },
    Section {
        key: "drawing-section-pitchforks",
        tools: &[
            DrawingKind::Pitchfork,
            DrawingKind::SchiffPitchfork,
            DrawingKind::ModifiedSchiffPitchfork,
            DrawingKind::InsidePitchfork,
        ],
    },
];

impl ToolGroup {
    pub const ALL: [ToolGroup; 1] = [ToolGroup::Lines];

    /// The Fluent key of the group's name, shown when hovering its button.
    pub fn key(self) -> &'static str {
        match self {
            ToolGroup::Lines => "drawing-group-lines",
        }
    }

    pub fn sections(self) -> &'static [Section] {
        match self {
            ToolGroup::Lines => &LINES,
        }
    }

    /// The tool the group's button shows before any of its tools is picked.
    pub fn first(self) -> DrawingKind {
        self.sections()[0].tools[0]
    }

    pub fn contains(self, kind: DrawingKind) -> bool {
        self.sections()
            .iter()
            .any(|section| section.tools.contains(&kind))
    }

    /// The group a tool is in, if any.
    pub fn of(kind: DrawingKind) -> Option<ToolGroup> {
        Self::ALL.into_iter().find(|group| group.contains(kind))
    }
}

/// The Fluent key of a tool's name.
pub fn name_key(kind: DrawingKind) -> String {
    format!("drawing-{}", kind.id())
}

/// The key that picks a tool together with Alt, as in TradingView.
pub fn shortcut(kind: DrawingKind) -> Option<char> {
    match kind {
        DrawingKind::TrendLine => Some('T'),
        DrawingKind::HorizontalLine => Some('H'),
        DrawingKind::HorizontalRay => Some('J'),
        DrawingKind::VerticalLine => Some('V'),
        DrawingKind::CrossLine => Some('C'),
        _ => None,
    }
}

/// The tool Alt and `key` pick, if any.
pub fn by_shortcut(key: &str) -> Option<DrawingKind> {
    DrawingKind::ALL.into_iter().find(|kind| {
        shortcut(*kind).is_some_and(|letter| key.eq_ignore_ascii_case(&letter.to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_tool_is_in_one_place() {
        let grouped: Vec<DrawingKind> = ToolGroup::ALL
            .iter()
            .flat_map(|group| group.sections())
            .flat_map(|section| section.tools.iter().copied())
            .collect();
        for kind in DrawingKind::ALL {
            let times = grouped.iter().filter(|&&tool| tool == kind).count();
            assert!(times <= 1, "{kind:?} is in a menu {times} times");
        }
        assert_eq!(grouped.len(), 17);
        assert_eq!(
            ToolGroup::of(DrawingKind::InsidePitchfork),
            Some(ToolGroup::Lines)
        );
        assert_eq!(ToolGroup::of(DrawingKind::Rectangle), None);
        assert_eq!(ToolGroup::Lines.first(), DrawingKind::TrendLine);
    }

    #[test]
    fn shortcuts_pick_their_tools() {
        assert_eq!(by_shortcut("t"), Some(DrawingKind::TrendLine));
        assert_eq!(by_shortcut("C"), Some(DrawingKind::CrossLine));
        assert_eq!(by_shortcut("j"), Some(DrawingKind::HorizontalRay));
        assert_eq!(by_shortcut("x"), None);
        for kind in DrawingKind::ALL {
            if let Some(letter) = shortcut(kind) {
                assert_eq!(by_shortcut(&letter.to_string()), Some(kind));
            }
        }
    }
}
