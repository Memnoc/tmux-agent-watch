use ratatui::style::Color;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Variant {
    RosePine,
    Moon,
    Dawn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Theme {
    pub base: Color,
    pub text: Color,
    pub muted: Color,
    pub rose: Color,
    pub pine: Color,
    pub gold: Color,
    pub love: Color,
}

impl Theme {
    pub fn rose_pine(variant: Variant) -> Self {
        let rgb = |r, g, b| Color::Rgb(r, g, b);
        match variant {
            Variant::RosePine => Self {
                base: rgb(25, 23, 36),
                text: rgb(224, 222, 244),
                muted: rgb(110, 106, 134),
                rose: rgb(235, 188, 186),
                pine: rgb(49, 116, 143),
                gold: rgb(246, 193, 119),
                love: rgb(235, 111, 146),
            },
            Variant::Moon => Self {
                base: rgb(35, 33, 54),
                text: rgb(224, 222, 244),
                muted: rgb(110, 106, 134),
                rose: rgb(234, 154, 151),
                pine: rgb(62, 143, 176),
                gold: rgb(246, 193, 119),
                love: rgb(235, 111, 146),
            },
            Variant::Dawn => Self {
                base: rgb(250, 244, 237),
                text: rgb(87, 82, 121),
                muted: rgb(152, 147, 165),
                rose: rgb(215, 130, 126),
                pine: rgb(40, 105, 131),
                gold: rgb(234, 157, 52),
                love: rgb(180, 99, 122),
            },
        }
    }
}
