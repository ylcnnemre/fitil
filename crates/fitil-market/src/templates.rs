//! Indicator templates: named sets of indicators with their settings, kept in the
//! user's data.

use std::sync::{Arc, Mutex};

use fitil_indicators::Settings;
use fitil_store::{StoredTemplate, UserData};
use serde::{Deserialize, Serialize};

use crate::MarketError;

pub type SharedUserData = Arc<Mutex<UserData>>;

/// Runs `operation` on the user data away from the async threads.
pub(crate) async fn with_user<T: Send + 'static>(
    user: SharedUserData,
    operation: impl FnOnce(&mut UserData) -> fitil_store::Result<T> + Send + 'static,
) -> Result<T, MarketError> {
    tokio::task::spawn_blocking(move || {
        let mut user = user.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        operation(&mut user)
    })
    .await
    .map_err(|error| MarketError::Storage(error.to_string()))?
    .map_err(MarketError::from)
}

/// The longest template name, in characters.
pub const TEMPLATE_NAME_MAX: usize = 60;

/// Version of the saved form, for later versions of Fitil to read older templates.
const FORMAT_VERSION: u32 = 1;

/// A named set of indicators.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub name: String,
    pub studies: Vec<Settings>,
}

#[derive(Serialize)]
struct SavedForm<'a> {
    version: u32,
    studies: &'a [Settings],
}

#[derive(Deserialize)]
struct ReadForm {
    /// Read one by one, so that one unreadable study does not lose the others.
    studies: Vec<serde_json::Value>,
}

impl Template {
    /// A template name from what the user typed: trimmed, not empty, not too long.
    pub fn clean_name(input: &str) -> Option<String> {
        let name = input.trim();
        (!name.is_empty()).then(|| name.chars().take(TEMPLATE_NAME_MAX).collect())
    }

    fn encode(&self) -> StoredTemplate {
        let form = SavedForm {
            version: FORMAT_VERSION,
            studies: &self.studies,
        };
        StoredTemplate {
            name: self.name.clone(),
            body: serde_json::to_string(&form).expect("settings always serialize"),
        }
    }

    /// Reads a stored template. Studies that cannot be read, or whose indicator no longer
    /// exists, are left out.
    fn decode(stored: StoredTemplate) -> Self {
        let studies = serde_json::from_str::<ReadForm>(&stored.body)
            .map(|form| {
                form.studies
                    .into_iter()
                    .filter_map(|study| serde_json::from_value::<Settings>(study).ok())
                    .filter_map(Settings::normalized)
                    .collect()
            })
            .unwrap_or_default();
        Self {
            name: stored.name,
            studies,
        }
    }
}

/// Every saved template, by name.
pub async fn load_templates(user: SharedUserData) -> Result<Vec<Template>, MarketError> {
    let stored = with_user(user, |user| user.indicator_templates()).await?;
    Ok(decode_all(stored))
}

/// Saves a template, replacing any with the same name, and returns every template.
pub async fn save_template(
    user: SharedUserData,
    template: Template,
) -> Result<Vec<Template>, MarketError> {
    let stored = template.encode();
    let all = with_user(user, move |user| {
        user.save_indicator_template(&stored)?;
        user.indicator_templates()
    })
    .await?;
    Ok(decode_all(all))
}

/// Deletes a template and returns the ones left.
pub async fn delete_template(
    user: SharedUserData,
    name: String,
) -> Result<Vec<Template>, MarketError> {
    let left = with_user(user, move |user| {
        user.delete_indicator_template(&name)?;
        user.indicator_templates()
    })
    .await?;
    Ok(decode_all(left))
}

fn decode_all(stored: Vec<StoredTemplate>) -> Vec<Template> {
    stored.into_iter().map(Template::decode).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fitil_indicators::{Rgb, Value, definition};

    fn user() -> SharedUserData {
        Arc::new(Mutex::new(UserData::open_in_memory().unwrap()))
    }

    fn template(name: &str) -> Template {
        let mut ema = Settings::defaults(definition("ema").unwrap());
        ema.params.set("length", Value::Number(200.0));
        ema.outputs[0].color = Rgb::hex(0x123456);
        Template {
            name: name.into(),
            studies: vec![ema, Settings::defaults(definition("rsi").unwrap())],
        }
    }

    #[test]
    fn names_are_trimmed_and_limited() {
        assert_eq!(Template::clean_name("  Trend  "), Some("Trend".into()));
        assert_eq!(Template::clean_name("   "), None);
        let long = "x".repeat(100);
        assert_eq!(
            Template::clean_name(&long).unwrap().chars().count(),
            TEMPLATE_NAME_MAX
        );
    }

    #[tokio::test]
    async fn templates_are_saved_read_and_deleted() {
        let user = user();
        let saved = save_template(user.clone(), template("Trend"))
            .await
            .unwrap();
        assert_eq!(saved, [template("Trend")]);
        assert_eq!(load_templates(user.clone()).await.unwrap(), saved);

        let left = delete_template(user.clone(), "Trend".into()).await.unwrap();
        assert!(left.is_empty());
    }

    #[test]
    fn unreadable_studies_are_left_out() {
        let body = r#"{
            "version": 1,
            "studies": [
                {"indicator":"rsi","params":{"length":21},"outputs":[],"visible":true},
                {"indicator":"no-such-indicator","params":{},"outputs":[],"visible":true},
                "not a study"
            ]
        }"#;
        let template = Template::decode(StoredTemplate {
            name: "Old".into(),
            body: body.into(),
        });
        assert_eq!(template.studies.len(), 1);
        assert_eq!(template.studies[0].params.length("length"), 21);
        // The missing output style was filled in from the defaults.
        assert_eq!(template.studies[0].outputs.len(), 1);

        let broken = Template::decode(StoredTemplate {
            name: "Broken".into(),
            body: "{".into(),
        });
        assert!(broken.studies.is_empty());
    }
}
