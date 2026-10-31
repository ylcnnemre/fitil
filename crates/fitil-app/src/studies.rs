//! The indicators on the chart in the window's terms: what the chart draws, and their
//! names in the user's language.

use fitil_chart::{BarColors, Plot, PlotFill, PlotStyle, StudyPlacement, ValueFormat};
use fitil_indicators::{
    Definition, Group, ParamKind, Placement, PlotKind, Rgb, Settings, Shading, Source, Value,
};
use fitil_market::{Studies, Study};
use iced::Color;

use crate::i18n::Translations;

/// Bars below zero, and bars of falling candles.
const NEGATIVE: Color = Color::from_rgb8(0xF2, 0x36, 0x45);
/// How strongly the area between two lines is shaded.
const FILL_ALPHA: f32 = 0.12;

pub fn color(rgb: Rgb) -> Color {
    Color::from_rgb8(rgb.0, rgb.1, rgb.2)
}

/// The visible studies, as the chart draws them.
pub fn chart_studies(studies: &Studies, t: &Translations) -> Vec<fitil_chart::Study> {
    studies
        .iter()
        .filter(|study| study.settings().visible)
        .map(|study| chart_study(study, t))
        .collect()
}

fn chart_study(study: &Study, t: &Translations) -> fitil_chart::Study {
    let definition = study.definition();
    let settings = study.settings();
    let plots = definition
        .outputs
        .iter()
        .zip(&settings.outputs)
        .zip(study.values())
        .zip(definition.offsets(&settings.params))
        .map(|(((output, look), values), offset)| Plot {
            values: values.clone(),
            offset,
            style: match output.kind {
                PlotKind::Line => PlotStyle::Line,
                PlotKind::Dots => PlotStyle::Dots,
                PlotKind::Histogram(Shading::Fixed) => PlotStyle::Histogram(BarColors::Fixed),
                PlotKind::Histogram(Shading::Sign) => {
                    PlotStyle::Histogram(BarColors::Sign { negative: NEGATIVE })
                }
                PlotKind::Histogram(Shading::Candle) => {
                    PlotStyle::Histogram(BarColors::Candle { falling: NEGATIVE })
                }
            },
            color: color(look.color),
            width: f32::from(look.width),
            visible: look.visible,
        })
        .collect();
    let fills = definition
        .fills
        .iter()
        .map(|fill| PlotFill {
            upper: fill.upper,
            lower: fill.lower,
            color: color(fill.color).scale_alpha(FILL_ALPHA),
            crossed: fill
                .crossed
                .map(|crossed| color(crossed).scale_alpha(FILL_ALPHA)),
        })
        .collect();
    fitil_chart::Study {
        id: study.id().get(),
        label: label(settings, t),
        placement: match definition.placement {
            Placement::Overlay => StudyPlacement::Overlay,
            Placement::Pane => StudyPlacement::Pane,
        },
        plots,
        fills,
        levels: definition.levels.to_vec(),
        range: definition.range,
        format: match definition.format {
            fitil_indicators::ValueFormat::Price => ValueFormat::Price,
            fitil_indicators::ValueFormat::Decimals(decimals) => ValueFormat::Decimals(decimals),
            fitil_indicators::ValueFormat::Volume => ValueFormat::Volume,
        },
    }
}

/// A study's short name with its numbers, such as `RSI 14` or `BB 20 2`. Sources and
/// choices are named only when they differ from the default.
pub fn label(settings: &Settings, t: &Translations) -> String {
    let Some(definition) = settings.definition() else {
        return settings.indicator.clone();
    };
    let mut label = t.get(&format!("indicator-{}-short", definition.id));
    for param in definition.params {
        let shown = match (param.kind, settings.params.get(param.id)) {
            (ParamKind::Integer { .. } | ParamKind::Float { .. }, Some(Value::Number(value))) => {
                Some(number(*value))
            }
            (ParamKind::Source { default }, Some(Value::Text(id))) if id != default.id() => {
                Source::from_id(id).map(|source| source_name(source, t))
            }
            (ParamKind::Choice { default, .. }, Some(Value::Text(id))) if id != default => {
                Some(choice_name(id, t))
            }
            _ => None,
        };
        if let Some(shown) = shown {
            label.push(' ');
            label.push_str(&shown);
        }
    }
    label
}

/// A parameter's number without needless decimals: `14`, `2.5`, `0.02`.
pub fn number(value: f64) -> String {
    let text = format!("{value:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A number typed by the user, with a comma or a point before the decimals.
pub fn parse_number(text: &str) -> Option<f64> {
    text.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

/// Whether an indicator matches what was typed in the search box: by its name in the
/// user's language, its short name or its id.
pub fn matches(definition: &Definition, search: &str, t: &Translations) -> bool {
    let search = search.trim().to_lowercase();
    search.is_empty()
        || [
            name(definition, t),
            t.get(&format!("indicator-{}-short", definition.id)),
            definition.id.to_owned(),
        ]
        .iter()
        .any(|text| text.to_lowercase().contains(&search))
}

pub fn name(definition: &Definition, t: &Translations) -> String {
    t.get(&format!("indicator-{}", definition.id))
}

pub fn group_name(group: Group, t: &Translations) -> String {
    t.get(&format!("indicator-group-{}", group.id()))
}

pub fn param_name(id: &str, t: &Translations) -> String {
    t.get(&format!("indicator-param-{id}"))
}

pub fn output_name(id: &str, t: &Translations) -> String {
    t.get(&format!("indicator-output-{id}"))
}

pub fn source_name(source: Source, t: &Translations) -> String {
    t.get(&format!("indicator-source-{}", source.id()))
}

pub fn choice_name(option: &str, t: &Translations) -> String {
    t.get(&format!("indicator-choice-{option}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Language;
    use fitil_indicators::{catalog, definition};

    #[test]
    fn every_indicator_is_translated() {
        for language in Language::ALL {
            let t = Translations::new(language);
            let mut keys = Vec::new();
            for definition in catalog() {
                keys.push(format!("indicator-{}", definition.id));
                keys.push(format!("indicator-{}-short", definition.id));
                keys.extend(
                    definition
                        .params
                        .iter()
                        .map(|p| format!("indicator-param-{}", p.id)),
                );
                keys.extend(
                    definition
                        .outputs
                        .iter()
                        .map(|o| format!("indicator-output-{}", o.id)),
                );
                for param in definition.params {
                    if let ParamKind::Choice { options, .. } = param.kind {
                        keys.extend(options.iter().map(|o| format!("indicator-choice-{o}")));
                    }
                }
            }
            keys.extend(Group::ALL.map(|g| format!("indicator-group-{}", g.id())));
            keys.extend(Source::ALL.map(|s| format!("indicator-source-{}", s.id())));
            for key in keys {
                assert_ne!(t.get(&key), key, "{} has no {key}", language.code());
            }
        }
    }

    #[test]
    fn labels_show_numbers_and_unusual_choices() {
        let t = Translations::new(Language::English);
        let mut bollinger = Settings::defaults(definition("bollinger").unwrap());
        assert_eq!(label(&bollinger, &t), "BB 20 2");
        bollinger.params.set("source", Value::Text("hl2".into()));
        bollinger.params.set("multiplier", Value::Number(2.5));
        assert_eq!(label(&bollinger, &t), "BB 20 hl2 2.5");

        let mut vwap = Settings::defaults(definition("vwap").unwrap());
        assert_eq!(label(&vwap, &t), "VWAP");
        vwap.params.set("anchor", Value::Text("week".into()));
        assert_eq!(label(&vwap, &t), "VWAP Week");
    }

    #[test]
    fn numbers_are_written_and_read_plainly() {
        assert_eq!(number(14.0), "14");
        assert_eq!(number(100.0), "100");
        assert_eq!(number(0.02), "0.02");
        assert_eq!(parse_number(" 2,5 "), Some(2.5));
        assert_eq!(parse_number("0.02"), Some(0.02));
        assert_eq!(parse_number("abc"), None);
        assert_eq!(parse_number("inf"), None);
    }

    #[test]
    fn search_finds_names_in_the_users_language() {
        let tr = Translations::new(Language::Turkish);
        let bollinger = definition("bollinger").unwrap();
        assert!(matches(bollinger, "bant", &tr));
        assert!(matches(bollinger, "BB", &tr));
        assert!(matches(bollinger, "", &tr));
        assert!(!matches(bollinger, "rsi", &tr));
    }
}
