use std::collections::VecDeque;

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
#[cfg(feature = "tera")]
use tera::Context;
use urlencoding::encode;

use crate::errors::{RustlinkTest, RustlinkTestError};

lazy_static::lazy_static!(
    static ref PAREN_REPLACE: Regex = Regex::new(r"\{.*\}").unwrap();
    static ref PAREN_REGEX: Regex = Regex::new(r"[\{\}]").unwrap();
    static ref HAT_REGEX: Regex = Regex::new(r"\^").unwrap();
);

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum RustlinkType {
    #[cfg(feature = "li")]
    LinkedIn = 0,
    #[cfg(feature = "glean")]
    Glean = 1,
    #[cfg(feature = "tera")]
    Tera = 2,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Rustlink {
    pub url: String,
    #[serde(rename = "type")]
    pub _type: RustlinkType,
    pub revision: i64,
}

impl Rustlink {
    pub fn new(url: String, _type: RustlinkType, revision: i64) -> Self {
        Rustlink {
            url,
            _type,
            revision,
        }
    }

    /// Example links:
    /// * https://google.com/search{?q=^}
    /// * https://linkedin.com/{in/^}
    #[cfg(feature = "li")]
    fn render_linkedin(&self, params: Vec<&str>) -> Result<String> {
        if params.len() == 0 {
            return Ok(PAREN_REPLACE.replace_all(&self.url, "").to_string());
        }

        let joined = params.join(" ");
        let extras = encode(&joined);

        if PAREN_REGEX.is_match(&self.url) {
            let url = HAT_REGEX.replace_all(&self.url, extras);
            let replaced = PAREN_REGEX.replace_all(&url, "");
            return Ok(replaced.to_string());
        }

        if let Some(q_index) = self.url.find("?") {
            let mut url = self.url.clone();
            url.insert_str(q_index, &extras);
            Ok(url)
        } else if let Some(hash_index) = self.url.find("#") {
            let mut url = self.url.clone();
            url.insert_str(hash_index, format!("?{}", extras).as_str());
            Ok(url)
        } else {
            Ok(format!("{}?{}", self.url, extras))
        }
    }

    /// Reference: https://help.glean.com/en/articles/6203607-create-variable-go-links
    ///
    /// Example links:
    /// * https://github.com/search?q={*}
    /// * https://github.com/product/{*}/issues/{*}
    #[cfg(feature = "glean")]
    fn render_glean(&self, params: Vec<&str>) -> Result<String> {
        // It doesn't mention it explicitly, but for now we assume that Glean links
        // place any remainder parameters in the last available variable
        // placeholder for now until I get a chance to actually test its behavior.
        let mut params = VecDeque::from(params);
        let pattern = "{*}";
        let mut url = self.url.clone();
        let placeholder_indices = url
            .match_indices(pattern)
            .map(|(i, _)| i)
            .collect::<Vec<usize>>();
        let mut index_adjustment: isize = 0;

        for (i, range_start) in placeholder_indices.iter().enumerate() {
            // Should hopefully always be a positive value
            let adjusted_start = (*range_start as isize + index_adjustment) as usize;
            let range = adjusted_start..(adjusted_start + pattern.len());
            let mut replacement = "".to_string();

            if i == placeholder_indices.len() - 1 && !params.is_empty() {
                let vecd: Vec<&str> = params.clone().into();
                replacement = encode(&vecd.join(" ")).into_owned();
            } else if let Some(param) = params.pop_front() {
                replacement = encode(param).into_owned();
            }

            index_adjustment =
                index_adjustment + (replacement.len() as isize - pattern.len() as isize);
            url.replace_range(range, &replacement);
        }
        Ok(url)
    }

    /// Reference: https://keats.github.io/tera/
    #[cfg(feature = "tera")]
    fn render_tera(&self, params: Vec<&str>) -> Result<String> {
        let mut context = Context::new();
        context.insert("params", &params);
        if let Ok(url) = tera::Tera::one_off(&self.url, &context, true) {
            Ok(url)
        } else {
            Err(anyhow::anyhow!("Failed to render Tera template"))
        }
    }

    pub fn render(&self, params: Vec<&str>) -> Result<String> {
        match self._type {
            #[cfg(feature = "li")]
            RustlinkType::LinkedIn => self.render_linkedin(params),
            #[cfg(feature = "glean")]
            RustlinkType::Glean => self.render_glean(params),
            #[cfg(feature = "tera")]
            RustlinkType::Tera => self.render_tera(params),
        }
    }

    pub fn test(&self) -> Result<(), RustlinkTestError> {
        // TODO: check whether we can determine the number of params required
        // for now this dumb test suffices
        let tests = vec![
            vec![],                     // 0 params
            vec!["rust"],               // 1 param
            vec!["rust", "is", "cool"], // 3 params
        ];

        for test in tests {
            if let Err(e) = self.render(test.clone()) {
                return Err(RustlinkTestError::RustlinkTestFailed(
                    e.to_string(),
                    RustlinkTest { params: test },
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod unit_tests {
    use std::vec;

    use anyhow::Result;

    fn compare_results(description: String, result: Result<String>, expected: Result<String>) {
        if expected.is_err() {
            assert!(result.is_err(), "{}", description);
            assert_eq!(
                result.err().unwrap().to_string(),
                expected.err().unwrap().to_string(),
                "{}",
                description
            );
            return;
        } else {
            assert_eq!(result.ok(), expected.ok(), "{}", description);
        }
    }

    #[cfg(feature = "li")]
    #[test]
    fn it_renders_li_rustlinks() {
        struct Test<'a> {
            description: String,
            url: String,
            params: Vec<&'a str>,
            expected: Result<String>,
        }

        let tests: Vec<Test> = vec![
            Test {
                description: "it renders a linkedin rustlink with no params".to_string(),
                url: "https://linkedin.com/in/{^}".to_string(),
                params: vec![],
                expected: Ok("https://linkedin.com/in/".to_string()),
            },
            Test {
                description: "it renders a linkedin rustlink with one param".to_string(),
                url: "https://linkedin.com/in/{^}".to_string(),
                params: vec!["rust"],
                expected: Ok("https://linkedin.com/in/rust".to_string()),
            },
            Test {
                description: "it renders a linkedin rustlink with multiple params".to_string(),
                url: "https://linkedin.com/in/{^}".to_string(),
                params: vec!["rust", "is", "cool"],
                expected: Ok("https://linkedin.com/in/rust%20is%20cool".to_string()),
            },
            Test {
                description: "it doesn't render any charaters in parentheses if no params received"
                    .to_string(),
                url: "https://linkedin.com/in/{?q=shouldnotbehere^}".to_string(),
                params: vec![],
                expected: Ok("https://linkedin.com/in/".to_string()),
            },
        ];

        for test in tests {
            let rustlink = super::Rustlink::new(test.url, super::RustlinkType::LinkedIn, 0);
            let rendered = rustlink.render(test.params);
            compare_results(test.description, rendered, test.expected);
        }
    }

    #[cfg(feature = "glean")]
    #[test]
    fn it_renders_glean_rustlinks() {
        struct Test<'a> {
            description: String,
            url: String,
            params: Vec<&'a str>,
            expected: Result<String>,
        }

        let tests: Vec<Test> = vec![
            Test {
                description: "it renders a glean rustlink with no params and one variable"
                    .to_string(),
                url: "https://glean.com/q={*}".to_string(),
                params: vec![],
                expected: Ok("https://glean.com/q=".to_string()),
            },
            Test {
                description: "it renders a glean rustlink with one param and one variable"
                    .to_string(),
                url: "https://glean.com/q={*}".to_string(),
                params: vec!["rust"],
                expected: Ok("https://glean.com/q=rust".to_string()),
            },
            Test {
                description: "it renders a glean rustlink with multiple params and one variable"
                    .to_string(),
                url: "https://glean.com/q={*}".to_string(),
                params: vec!["rust", "is", "cool"],
                expected: Ok("https://glean.com/q=rust%20is%20cool".to_string()),
            },
            Test {
                description: "it renders a glean rustlink with multiple params and fewer variables"
                    .to_string(),
                url: "https://glean.com/q={*}&c={*}".to_string(),
                params: vec!["rust", "is", "cool"],
                expected: Ok("https://glean.com/q=rust&c=is%20cool".to_string()),
            },
            Test {
                description: "it renders a glean rustlink with params of shorter string length than the variable pattern"
                    .to_string(),
                url: "https://glean.com/q={*}&c={*}&d={*}".to_string(),
                params: vec!["a", "b", "ab"],
                expected: Ok("https://glean.com/q=a&c=b&d=ab".to_string()),
            },
        ];

        for test in tests {
            let rustlink = super::Rustlink::new(test.url, super::RustlinkType::Glean, 0);
            let rendered = rustlink.render(test.params);
            compare_results(test.description, rendered, test.expected);
        }
    }

    #[cfg(feature = "tera")]
    #[test]
    fn it_renders_tera_rustlinks() {
        struct Test<'a> {
            description: String,
            url: String,
            params: Vec<&'a str>,
            expected: Result<String>,
        }

        let tests: Vec<Test> = vec![
            Test {
                description: "it renders a tera rustlink with no params".to_string(),
                url: "https://keats.github.io/".to_string(),
                params: vec![],
                expected: Ok("https://keats.github.io/".to_string()),
            },
            Test {
                description: "it renders a tera rustlink with one param".to_string(),
                url: "https://keats.github.io/{{ params[0] }}".to_string(),
                params: vec!["rust"],
                expected: Ok("https://keats.github.io/rust".to_string()),
            },
            Test {
                description: "it returns an error when requiring a param which isnt received"
                    .to_string(),
                url: "https://keats.github.io/{{ params[0] }}".to_string(),
                params: vec![],
                expected: Ok("https://keats.github.io/rust".to_string()),
            },
            Test {
                description:
                    "it renders a tera rustlink with multiple params, join clause, and if statement"
                        .to_string(),
                url: r#"{%- if params | length > 0 -%}
https://keats.github.io/?q={{ params | join(sep=" ") | urlencode }}
{%- else -%}
https://keats.github.io/
{% endif %}"#
                    .to_string(),
                params: vec!["rust", "is", "cool"],
                expected: Ok("https://keats.github.io/?q=rust%20is%20cool".to_string()),
            },
        ];

        for test in tests {
            let rustlink = super::Rustlink::new(test.url, super::RustlinkType::Tera, 0);
            let rendered = rustlink.render(test.params);
            compare_results(test.description, rendered, test.expected);
        }
    }
}
